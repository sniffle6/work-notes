use std::io::Write;
use std::time::Duration;

use serde_json::Value;

use super::JiraError;

const GATEWAY_ROOT: &str = "https://api.atlassian.com/ex/jira";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Attachments can be large, so a download gets longer than a JSON request.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const MAX_ERROR_MESSAGE_CHARS: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
        }
    }
}

pub trait JiraTransport {
    /// Sends one request to `rest/api/3/{path}` and returns the JSON body.
    /// Empty success bodies are returned as `Value::Null`.
    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, JiraError>;

    /// Streams the body of `GET rest/api/3/{path}` into `sink` and returns
    /// the number of bytes written. A failure can leave `sink` partly written.
    fn download(&self, path: &str, sink: &mut dyn Write) -> Result<u64, JiraError>;
}

#[derive(Clone)]
pub struct JiraCredentials {
    pub email: String,
    pub api_token: String,
}

/// Talks to Jira Cloud through the `api.atlassian.com` gateway.
///
/// Scoped API tokens only work through the gateway; the site-direct URL
/// answers 404 for them. The token is therefore only ever sent to
/// `api.atlassian.com`.
pub struct GatewayTransport {
    base_url: String,
    credentials: JiraCredentials,
    client: reqwest::blocking::Client,
}

impl GatewayTransport {
    pub fn new(credentials: JiraCredentials, cloud_id: &str) -> Result<Self, JiraError> {
        Ok(Self {
            base_url: gateway_base_url(cloud_id)?,
            credentials,
            client: http_client()?,
        })
    }
}

impl JiraTransport for GatewayTransport {
    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, JiraError> {
        let url = format!("{}{}", self.base_url, path);
        let request = match method {
            Method::Get => self.client.get(url),
            Method::Post => self.client.post(url),
            Method::Put => self.client.put(url),
        };
        let mut request = request
            .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
            .header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().map_err(network_error)?;
        let status = response.status().as_u16();
        let text = response.text().map_err(network_error)?;
        if !(200..300).contains(&status) {
            return Err(error_from_response(method, path, status, &text));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|error| JiraError::InvalidResponse(error.to_string()))
    }

    fn download(&self, path: &str, sink: &mut dyn Write) -> Result<u64, JiraError> {
        // Jira answers attachment content with a redirect to its media
        // host. reqwest follows it and drops the Authorization header when
        // the host changes, so the token still only reaches the gateway.
        let mut response = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
            .timeout(DOWNLOAD_TIMEOUT)
            .send()
            .map_err(network_error)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let text = response.text().map_err(network_error)?;
            return Err(error_from_response(Method::Get, path, status, &text));
        }
        std::io::copy(&mut response, sink).map_err(|error| JiraError::Download(error.to_string()))
    }
}

/// Looks up the cloud ID for a Jira site from its public tenant endpoint.
/// No credentials are sent.
pub fn resolve_cloud_id(site_url: &str) -> Result<String, JiraError> {
    let site_url = normalize_site_url(site_url)?;
    let response = http_client()?
        .get(format!("{site_url}/_edge/tenant_info"))
        .header("Accept", "application/json")
        .send()
        .map_err(network_error)?;
    let status = response.status().as_u16();
    let text = response.text().map_err(network_error)?;
    if !(200..300).contains(&status) {
        return Err(JiraError::NotConfigured(format!(
            "{site_url} did not identify itself as a Jira Cloud site (HTTP {status})"
        )));
    }
    cloud_id_from_tenant_info(&text)
}

fn http_client() -> Result<reqwest::blocking::Client, JiraError> {
    // reqwest is built without a bundled TLS provider, matching the updater.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(network_error)
}

fn network_error(error: reqwest::Error) -> JiraError {
    // Drop the URL so error text never carries request details.
    JiraError::Network(error.without_url().to_string())
}

/// Returns the gateway REST root for a cloud ID, ending in `/`.
pub fn gateway_base_url(cloud_id: &str) -> Result<String, JiraError> {
    let cloud_id = validate_cloud_id(cloud_id)?;
    Ok(format!("{GATEWAY_ROOT}/{cloud_id}/rest/api/3/"))
}

/// Trims a cloud ID and checks it is safe to place in a URL path.
pub fn validate_cloud_id(cloud_id: &str) -> Result<String, JiraError> {
    let cloud_id = cloud_id.trim();
    let valid = !cloud_id.is_empty()
        && cloud_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if !valid {
        return Err(JiraError::NotConfigured(
            "the Jira cloud ID is missing or malformed".to_string(),
        ));
    }
    Ok(cloud_id.to_string())
}

/// Reduces user input to `https://host` for a Jira Cloud site.
pub fn normalize_site_url(input: &str) -> Result<String, JiraError> {
    let invalid = || {
        JiraError::InvalidInput(
            "Jira site URL must look like https://your-team.atlassian.net".to_string(),
        )
    };
    let trimmed = input.trim();
    let without_scheme = match trimmed.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("https") => rest,
        Some(_) => return Err(invalid()),
        None => trimmed,
    };
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let valid_host = host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });
    if !valid_host {
        return Err(invalid());
    }
    Ok(format!("https://{host}"))
}

fn cloud_id_from_tenant_info(body: &str) -> Result<String, JiraError> {
    let not_jira =
        || JiraError::NotConfigured("the site did not report a Jira cloud ID".to_string());
    let value: Value = serde_json::from_str(body).map_err(|_| not_jira())?;
    let cloud_id = value
        .get("cloudId")
        .and_then(Value::as_str)
        .ok_or_else(not_jira)?;
    validate_cloud_id(cloud_id).map_err(|_| not_jira())
}

/// Maps a non-2xx Jira response to an error with a readable message.
pub fn error_from_response(method: Method, path: &str, status: u16, body: &str) -> JiraError {
    match status {
        401 => JiraError::Unauthorized,
        404 => JiraError::NotFound {
            path: path.to_string(),
        },
        _ => JiraError::Http {
            method: method.as_str(),
            path: path.to_string(),
            status,
            message: error_message_from_body(body),
        },
    }
}

fn error_message_from_body(body: &str) -> String {
    let structured = serde_json::from_str::<Value>(body).ok().and_then(|value| {
        let mut parts: Vec<String> = value
            .get("errorMessages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        if let Some(errors) = value.get("errors").and_then(Value::as_object) {
            parts.extend(errors.iter().map(|(field, message)| {
                format!(
                    "{field}: {}",
                    message.as_str().unwrap_or(&message.to_string())
                )
            }));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    });

    structured
        .unwrap_or_else(|| body.trim().to_string())
        .chars()
        .take(MAX_ERROR_MESSAGE_CHARS)
        .collect()
}

#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io::Write;

    use serde_json::Value;

    use super::{JiraTransport, Method};
    use crate::jira::JiraError;

    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct RecordedRequest {
        pub method: Method,
        pub path: String,
        pub body: Option<Value>,
    }

    /// Replays queued responses in call order and records each request.
    /// Downloads replay from their own queue and are recorded as GETs.
    #[derive(Default)]
    pub(crate) struct FakeTransport {
        responses: RefCell<VecDeque<Result<Value, JiraError>>>,
        downloads: RefCell<VecDeque<Result<Vec<u8>, JiraError>>>,
        requests: RefCell<Vec<RecordedRequest>>,
    }

    impl FakeTransport {
        pub(crate) fn with_responses(responses: Vec<Result<Value, JiraError>>) -> Self {
            Self {
                responses: RefCell::new(responses.into()),
                ..Self::default()
            }
        }

        pub(crate) fn with_downloads(self, downloads: Vec<Result<Vec<u8>, JiraError>>) -> Self {
            *self.downloads.borrow_mut() = downloads.into();
            self
        }

        pub(crate) fn requests(&self) -> Vec<RecordedRequest> {
            self.requests.borrow().clone()
        }
    }

    impl JiraTransport for FakeTransport {
        fn send(
            &self,
            method: Method,
            path: &str,
            body: Option<&Value>,
        ) -> Result<Value, JiraError> {
            self.requests.borrow_mut().push(RecordedRequest {
                method,
                path: path.to_string(),
                body: body.cloned(),
            });
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| panic!("no fake response queued for {} {path}", method.as_str()))
        }

        fn download(&self, path: &str, sink: &mut dyn Write) -> Result<u64, JiraError> {
            self.requests.borrow_mut().push(RecordedRequest {
                method: Method::Get,
                path: path.to_string(),
                body: None,
            });
            let bytes = self
                .downloads
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| panic!("no fake download queued for {path}"))?;
            sink.write_all(&bytes)
                .map_err(|error| JiraError::Download(error.to_string()))?;
            Ok(bytes.len() as u64)
        }
    }

    impl<T: JiraTransport> JiraTransport for &T {
        fn send(
            &self,
            method: Method,
            path: &str,
            body: Option<&Value>,
        ) -> Result<Value, JiraError> {
            (**self).send(method, path, body)
        }

        fn download(&self, path: &str, sink: &mut dyn Write) -> Result<u64, JiraError> {
            (**self).download(path, sink)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cloud_id_from_tenant_info, error_from_response, gateway_base_url, normalize_site_url,
        Method,
    };
    use crate::jira::JiraError;

    #[test]
    fn gateway_url_embeds_cloud_id() {
        assert_eq!(
            gateway_base_url(" 0a1b2c3d-1111-2222-3333-444455556666 ").unwrap(),
            "https://api.atlassian.com/ex/jira/0a1b2c3d-1111-2222-3333-444455556666/rest/api/3/"
        );
    }

    #[test]
    fn gateway_url_rejects_values_that_are_not_cloud_ids() {
        for value in ["", "abc/../def", "abc?x=1", "not a cloud id"] {
            assert!(
                gateway_base_url(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn site_url_is_reduced_to_https_origin() {
        assert_eq!(
            normalize_site_url(" https://Example-Team.atlassian.net/jira/software/projects/AB ")
                .unwrap(),
            "https://example-team.atlassian.net"
        );
        assert_eq!(
            normalize_site_url("example-team.atlassian.net").unwrap(),
            "https://example-team.atlassian.net"
        );
    }

    #[test]
    fn site_url_rejects_insecure_or_malformed_input() {
        for value in [
            "",
            "http://example.atlassian.net",
            "https://",
            "https://user:pw@example.atlassian.net",
            "https://exa mple.atlassian.net",
            "ftp://example.atlassian.net",
        ] {
            assert!(
                normalize_site_url(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn tenant_info_yields_cloud_id() {
        assert_eq!(
            cloud_id_from_tenant_info(r#"{"cloudId":"0a1b2c3d-1111-2222-3333-444455556666"}"#)
                .unwrap(),
            "0a1b2c3d-1111-2222-3333-444455556666"
        );
        assert!(cloud_id_from_tenant_info("<html>login</html>").is_err());
        assert!(cloud_id_from_tenant_info(r#"{"cloudId":"../x"}"#).is_err());
    }

    #[test]
    fn unauthorized_and_missing_map_to_specific_errors() {
        assert_eq!(
            error_from_response(Method::Get, "myself", 401, ""),
            JiraError::Unauthorized
        );
        assert_eq!(
            error_from_response(Method::Get, "issue/AB-1", 404, "{}"),
            JiraError::NotFound {
                path: "issue/AB-1".to_string()
            }
        );
    }

    #[test]
    fn jira_error_messages_are_extracted_from_the_body() {
        let error = error_from_response(
            Method::Post,
            "issue",
            400,
            r#"{"errorMessages":["Top level"],"errors":{"summary":"You must specify a summary."}}"#,
        );

        assert_eq!(
            error,
            JiraError::Http {
                method: "POST",
                path: "issue".to_string(),
                status: 400,
                message: "Top level; summary: You must specify a summary.".to_string(),
            }
        );
    }

    #[test]
    fn unstructured_error_bodies_are_truncated() {
        let error = error_from_response(Method::Put, "issue/AB-1", 500, &"x".repeat(5_000));

        let JiraError::Http { message, .. } = error else {
            panic!("expected http error");
        };
        assert_eq!(message.chars().count(), 600);
    }
}
