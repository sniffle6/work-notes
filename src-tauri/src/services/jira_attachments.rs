//! Saving a ticket's attachments to disk.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::jira::{IssueKey, JiraAttachment, JiraError, JiraOps};

/// Folder under the user's Downloads folder that holds one folder per ticket.
const DOWNLOAD_FOLDER_NAME: &str = "Work Notes";
/// Used when nothing of Jira's file name survives cleaning.
const FALLBACK_FILE_NAME: &str = "attachment";
/// A download is written under this suffix and renamed once complete.
const PARTIAL_SUFFIX: &str = ".part";
/// Jira thumbnails are a few tens of kilobytes; anything far larger is not one.
const MAX_THUMBNAIL_BYTES: usize = 2 * 1024 * 1024;
/// Larger images are opened from their folder instead of in the app.
const MAX_VIEWED_IMAGE_BYTES: u64 = 30 * 1024 * 1024;
const WINDOWS_FORBIDDEN_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const WINDOWS_DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedAttachment {
    pub id: String,
    /// The local file name, which can differ from the name Jira reports.
    pub filename: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDownload {
    /// The ticket's folder. It does not exist when the ticket has no
    /// attachments.
    pub folder: String,
    pub files: Vec<DownloadedAttachment>,
}

/// Returns the current user's Downloads folder.
pub fn user_downloads_dir() -> Result<PathBuf, JiraError> {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(Path::to_path_buf))
        .ok_or_else(|| JiraError::Download("the Downloads folder could not be found".to_string()))
}

/// Returns the folder a ticket's attachments are saved in.
pub fn issue_download_dir(downloads_dir: &Path, key: &IssueKey) -> PathBuf {
    downloads_dir.join(DOWNLOAD_FOLDER_NAME).join(key.as_str())
}

/// Saves a ticket's attachments under `downloads_dir` and returns where each
/// one is.
///
/// `attachment_ids` limits the download to those attachments; `None` saves
/// all of them. An id the ticket does not have is rejected before anything is
/// downloaded. A file already present at the size Jira reports is kept and
/// not downloaded again. The first failure stops the download; files saved
/// before it stay.
pub fn download_attachments(
    client: &impl JiraOps,
    key: &IssueKey,
    attachment_ids: Option<&[String]>,
    downloads_dir: &Path,
) -> Result<AttachmentDownload, JiraError> {
    let attachments = client.attachments(key)?;
    // Names are assigned over the whole ticket so that a file keeps its name
    // whether it is downloaded alone or with the rest.
    let names = local_file_names(&attachments);
    if let Some(unknown) = attachment_ids.into_iter().flatten().find(|id| {
        !attachments
            .iter()
            .any(|attachment| attachment.id == id.trim())
    }) {
        return Err(JiraError::InvalidInput(format!(
            "{key} has no attachment with id \"{}\"",
            unknown.trim()
        )));
    }
    let wanted = |attachment: &JiraAttachment| {
        attachment_ids.is_none_or(|ids| ids.iter().any(|id| id.trim() == attachment.id))
    };

    let folder = issue_download_dir(downloads_dir, key);
    let mut files = Vec::new();
    for (attachment, filename) in attachments.iter().zip(names) {
        if !wanted(attachment) {
            continue;
        }
        fs::create_dir_all(&folder).map_err(io_error)?;
        let path = folder.join(&filename);
        save_attachment(client, attachment, &path)?;
        files.push(DownloadedAttachment {
            id: attachment.id.clone(),
            filename,
            path: path.to_string_lossy().into_owned(),
        });
    }

    Ok(AttachmentDownload {
        folder: folder.to_string_lossy().into_owned(),
        files,
    })
}

fn save_attachment(
    client: &impl JiraOps,
    attachment: &JiraAttachment,
    path: &Path,
) -> Result<(), JiraError> {
    let already_saved = fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == attachment.size);
    if already_saved {
        return Ok(());
    }

    let mut partial = path.as_os_str().to_owned();
    partial.push(PARTIAL_SUFFIX);
    let partial = PathBuf::from(partial);
    let saved = write_content(client, &attachment.id, &partial)
        .and_then(|()| fs::rename(&partial, path).map_err(io_error));
    if saved.is_err() {
        let _ = fs::remove_file(&partial);
    }
    saved
}

fn write_content(client: &impl JiraOps, attachment_id: &str, path: &Path) -> Result<(), JiraError> {
    let mut writer = BufWriter::new(File::create(path).map_err(io_error)?);
    client.download_attachment(attachment_id, &mut writer)?;
    writer.flush().map_err(io_error)
}

/// Jira's small preview image of an attachment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentThumbnail {
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Fetches Jira's preview image of an attachment into memory.
///
/// Fails when Jira has no preview for the attachment, when the content is
/// larger than a thumbnail should be, or when it is not a PNG, JPEG, GIF, or
/// WebP image.
pub fn load_thumbnail(
    client: &impl JiraOps,
    attachment_id: &str,
) -> Result<AttachmentThumbnail, JiraError> {
    let mut buffer = CappedBuffer {
        bytes: Vec::new(),
        limit: MAX_THUMBNAIL_BYTES,
    };
    client.download_attachment_thumbnail(attachment_id, &mut buffer)?;
    let mime_type = image_mime_type(&buffer.bytes).ok_or_else(|| {
        JiraError::InvalidResponse("Jira returned a thumbnail that is not an image".to_string())
    })?;
    Ok(AttachmentThumbnail {
        mime_type,
        bytes: buffer.bytes,
    })
}

/// Returns the content of an image attachment for showing in the app,
/// downloading it to the ticket folder first when it is not saved yet.
///
/// Fails with `InvalidInput` when the file is larger than the app shows or
/// is not a PNG, JPEG, GIF, or WebP image. The downloaded file stays either
/// way.
pub fn read_attachment_image(
    client: &impl JiraOps,
    key: &IssueKey,
    attachment_id: &str,
    downloads_dir: &Path,
) -> Result<Vec<u8>, JiraError> {
    read_image_within(
        client,
        key,
        attachment_id,
        downloads_dir,
        MAX_VIEWED_IMAGE_BYTES,
    )
}

fn read_image_within(
    client: &impl JiraOps,
    key: &IssueKey,
    attachment_id: &str,
    downloads_dir: &Path,
    limit: u64,
) -> Result<Vec<u8>, JiraError> {
    let wanted = [attachment_id.to_string()];
    let download = download_attachments(client, key, Some(&wanted), downloads_dir)?;
    let file = download.files.first().ok_or_else(|| {
        JiraError::InvalidResponse("the attachment was not downloaded".to_string())
    })?;

    let size = fs::metadata(&file.path).map_err(io_error)?.len();
    if size > limit {
        return Err(JiraError::InvalidInput(format!(
            "{} is too large to show in the app; open it from its folder",
            file.filename
        )));
    }
    let bytes = fs::read(&file.path).map_err(io_error)?;
    if image_mime_type(&bytes).is_none() {
        return Err(JiraError::InvalidInput(format!(
            "{} is not an image the app can show; open it from its folder",
            file.filename
        )));
    }
    Ok(bytes)
}

/// Collects written bytes and fails once they pass `limit`.
struct CappedBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for CappedBuffer {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.bytes.len() + data.len() > self.limit {
            return Err(std::io::Error::other(
                "the thumbnail is larger than expected",
            ));
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Identifies an image by its leading bytes, since the transport does not
/// pass on the response's content type.
fn image_mime_type(content: &[u8]) -> Option<&'static str> {
    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    const JPEG_SIGNATURE: &[u8] = b"\xff\xd8\xff";
    const GIF_SIGNATURE: &[u8] = b"GIF8";
    const RIFF_SIGNATURE: &[u8] = b"RIFF";
    const WEBP_FORMAT: &[u8] = b"WEBP";
    const WEBP_FORMAT_OFFSET: usize = 8;

    if content.starts_with(PNG_SIGNATURE) {
        return Some("image/png");
    }
    if content.starts_with(JPEG_SIGNATURE) {
        return Some("image/jpeg");
    }
    if content.starts_with(GIF_SIGNATURE) {
        return Some("image/gif");
    }
    let is_webp = content.starts_with(RIFF_SIGNATURE)
        && content
            .get(WEBP_FORMAT_OFFSET..)
            .is_some_and(|format| format.starts_with(WEBP_FORMAT));
    is_webp.then_some("image/webp")
}

fn io_error(error: std::io::Error) -> JiraError {
    JiraError::Download(error.to_string())
}

/// Assigns each attachment a file name that is safe on Windows and unique
/// within the ticket, ignoring case. A later attachment whose name is taken
/// gets its Jira id added.
fn local_file_names(attachments: &[JiraAttachment]) -> Vec<String> {
    let mut taken = HashSet::new();
    attachments
        .iter()
        .map(|attachment| {
            let mut name = safe_file_name(&attachment.filename);
            if taken.contains(&name.to_lowercase()) {
                name = with_id(&name, &attachment.id);
            }
            taken.insert(name.to_lowercase());
            name
        })
        .collect()
}

/// Reduces a name from Jira to one path component Windows accepts, so it
/// cannot point outside the ticket folder.
fn safe_file_name(filename: &str) -> String {
    let replaced: String = filename
        .chars()
        .map(|character| {
            if character.is_control() || WINDOWS_FORBIDDEN_CHARS.contains(&character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    // Windows drops trailing dots and spaces, which also rules out `.` and `..`.
    let trimmed = replaced.trim_start().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return FALLBACK_FILE_NAME.to_string();
    }

    let stem = trimmed.split('.').next().unwrap_or_default().trim_end();
    let is_device_name = WINDOWS_DEVICE_NAMES
        .iter()
        .any(|device| device.eq_ignore_ascii_case(stem));
    if is_device_name {
        return format!("_{trimmed}");
    }
    trimmed.to_string()
}

fn with_id(name: &str, id: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => format!("{stem} ({id}).{extension}"),
        _ => format!("{name} ({id})"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::{json, Value};

    use super::{
        download_attachments, issue_download_dir, load_thumbnail, read_attachment_image,
        read_image_within, MAX_THUMBNAIL_BYTES,
    };
    use crate::jira::transport::testing::FakeTransport;
    use crate::jira::{IssueKey, JiraClient, JiraError};

    const SITE: &str = "https://example.atlassian.net";

    fn key() -> IssueKey {
        IssueKey::parse("AB-7").unwrap()
    }

    fn attachment(id: &str, filename: &str, size: usize) -> Value {
        json!({ "id": id, "filename": filename, "size": size })
    }

    fn issue_with(attachments: Vec<Value>) -> Result<Value, JiraError> {
        Ok(json!({ "key": "AB-7", "fields": { "attachment": attachments } }))
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn downloads_every_attachment_into_the_ticket_folder() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("10001", "bench.log", 3),
            attachment("10002", "photo.png", 4),
        ])])
        .with_downloads(vec![Ok(b"jam".to_vec()), Ok(b"\x89PNG".to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let result = download_attachments(&client, &key(), None, downloads.path()).unwrap();

        let folder = downloads.path().join("Work Notes").join("AB-7");
        assert_eq!(issue_download_dir(downloads.path(), &key()), folder);
        assert_eq!(result.folder, folder.to_string_lossy());
        assert_eq!(result.files.len(), 2);
        assert_eq!(result.files[0].id, "10001");
        assert_eq!(result.files[0].filename, "bench.log");
        assert_eq!(
            result.files[0].path,
            folder.join("bench.log").to_string_lossy()
        );
        assert_eq!(fs::read(folder.join("bench.log")).unwrap(), b"jam");
        assert_eq!(fs::read(folder.join("photo.png")).unwrap(), b"\x89PNG");
        assert_eq!(file_names(&folder), vec!["bench.log", "photo.png"]);
        let paths: Vec<String> = transport
            .requests()
            .into_iter()
            .map(|request| request.path)
            .collect();
        assert_eq!(
            paths,
            vec![
                "issue/AB-7?fields=attachment",
                "attachment/content/10001",
                "attachment/content/10002",
            ]
        );
    }

    #[test]
    fn downloads_only_the_requested_attachment() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("10001", "bench.log", 3),
            attachment("10002", "photo.png", 4),
        ])])
        .with_downloads(vec![Ok(b"\x89PNG".to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let result = download_attachments(
            &client,
            &key(),
            Some(&["10002".to_string()]),
            downloads.path(),
        )
        .unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].filename, "photo.png");
        assert_eq!(
            file_names(&issue_download_dir(downloads.path(), &key())),
            vec!["photo.png"]
        );
    }

    #[test]
    fn rejects_an_attachment_the_ticket_does_not_have() {
        let downloads = tempfile::tempdir().unwrap();
        let transport =
            FakeTransport::with_responses(vec![issue_with(vec![attachment("10001", "a.log", 1)])]);
        let client = JiraClient::new(&transport, SITE);

        let error = download_attachments(
            &client,
            &key(),
            Some(&["99999".to_string()]),
            downloads.path(),
        )
        .unwrap_err();

        assert!(matches!(error, JiraError::InvalidInput(_)), "{error:?}");
        assert_eq!(transport.requests().len(), 1);
        assert!(!issue_download_dir(downloads.path(), &key()).exists());
    }

    #[test]
    fn a_ticket_without_attachments_creates_no_folder() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(Vec::new())]);
        let client = JiraClient::new(&transport, SITE);

        let result = download_attachments(&client, &key(), None, downloads.path()).unwrap();

        assert!(result.files.is_empty());
        assert!(!issue_download_dir(downloads.path(), &key()).exists());
    }

    #[test]
    fn file_names_from_jira_cannot_leave_the_ticket_folder() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("1", "..\\..\\evil.txt", 1),
            attachment("2", "../up/and:away?.txt", 1),
            attachment("3", "CON.txt", 1),
            attachment("4", " . ", 1),
        ])])
        .with_downloads(vec![
            Ok(b"a".to_vec()),
            Ok(b"b".to_vec()),
            Ok(b"c".to_vec()),
            Ok(b"d".to_vec()),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let result = download_attachments(&client, &key(), None, downloads.path()).unwrap();

        let folder = issue_download_dir(downloads.path(), &key());
        assert_eq!(
            file_names(&folder),
            vec![
                ".._.._evil.txt",
                ".._up_and_away_.txt",
                "_CON.txt",
                "attachment"
            ]
        );
        for file in &result.files {
            assert_eq!(Path::new(&file.path).parent().unwrap(), folder);
        }
        // Only the two levels this download created exist under Downloads.
        assert_eq!(file_names(downloads.path()), vec!["Work Notes"]);
    }

    #[test]
    fn attachments_sharing_a_name_get_separate_files() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("10001", "screen.png", 1),
            attachment("10002", "Screen.PNG", 1),
            attachment("10003", "README", 1),
            attachment("10004", "readme", 1),
        ])])
        .with_downloads(vec![
            Ok(b"a".to_vec()),
            Ok(b"b".to_vec()),
            Ok(b"c".to_vec()),
            Ok(b"d".to_vec()),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let result = download_attachments(&client, &key(), None, downloads.path()).unwrap();

        let names: Vec<&str> = result
            .files
            .iter()
            .map(|file| file.filename.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "screen.png",
                "Screen (10002).PNG",
                "README",
                "readme (10004)"
            ]
        );
        let folder = issue_download_dir(downloads.path(), &key());
        assert_eq!(
            fs::read(folder.join("Screen (10002).PNG")).unwrap(),
            b"b"
        );
    }

    #[test]
    fn a_file_already_saved_at_the_reported_size_is_not_downloaded_again() {
        let downloads = tempfile::tempdir().unwrap();
        let folder = issue_download_dir(downloads.path(), &key());
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("bench.log"), b"jam").unwrap();
        fs::write(folder.join("photo.png"), b"stale").unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("10001", "bench.log", 3),
            attachment("10002", "photo.png", 4),
        ])])
        .with_downloads(vec![Ok(b"\x89PNG".to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let result = download_attachments(&client, &key(), None, downloads.path()).unwrap();

        assert_eq!(result.files.len(), 2);
        assert_eq!(transport.requests().len(), 2);
        assert_eq!(transport.requests()[1].path, "attachment/content/10002");
        assert_eq!(fs::read(folder.join("photo.png")).unwrap(), b"\x89PNG");
    }

    #[test]
    fn thumbnail_is_returned_with_the_image_type_found_in_its_content() {
        let images: [(&[u8], &str); 4] = [
            (b"\x89PNG\r\n\x1a\nrest", "image/png"),
            (b"\xff\xd8\xff\xe0rest", "image/jpeg"),
            (b"GIF89a rest", "image/gif"),
            (b"RIFF\x10\x00\x00\x00WEBPrest", "image/webp"),
        ];
        for (content, mime_type) in images {
            let transport = FakeTransport::default().with_downloads(vec![Ok(content.to_vec())]);
            let client = JiraClient::new(&transport, SITE);

            let thumbnail = load_thumbnail(&client, "10002").unwrap();

            assert_eq!(thumbnail.mime_type, mime_type);
            assert_eq!(thumbnail.bytes, content);
            assert_eq!(transport.requests()[0].path, "attachment/thumbnail/10002");
        }
    }

    #[test]
    fn thumbnail_content_that_is_not_an_image_is_rejected() {
        let transport =
            FakeTransport::default().with_downloads(vec![Ok(b"<html>sign in</html>".to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let error = load_thumbnail(&client, "10002").unwrap_err();

        assert!(matches!(error, JiraError::InvalidResponse(_)), "{error:?}");
    }

    #[test]
    fn a_thumbnail_over_the_size_limit_is_rejected() {
        let mut content = b"\x89PNG\r\n\x1a\n".to_vec();
        content.resize(MAX_THUMBNAIL_BYTES + 1, 0);
        let transport = FakeTransport::default().with_downloads(vec![Ok(content)]);
        let client = JiraClient::new(&transport, SITE);

        let error = load_thumbnail(&client, "10002").unwrap_err();

        assert!(matches!(error, JiraError::Download(_)), "{error:?}");
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\npixels";

    #[test]
    fn reading_an_image_downloads_it_first_when_it_is_not_saved() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("10001", "bench.log", 3),
            attachment("10002", "photo.png", PNG.len()),
        ])])
        .with_downloads(vec![Ok(PNG.to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let bytes = read_attachment_image(&client, &key(), "10002", downloads.path()).unwrap();

        assert_eq!(bytes, PNG);
        let folder = issue_download_dir(downloads.path(), &key());
        assert_eq!(fs::read(folder.join("photo.png")).unwrap(), PNG);
        assert_eq!(transport.requests()[1].path, "attachment/content/10002");
    }

    #[test]
    fn reading_a_saved_image_does_not_download_it_again() {
        let downloads = tempfile::tempdir().unwrap();
        let folder = issue_download_dir(downloads.path(), &key());
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("photo.png"), PNG).unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![attachment(
            "10002",
            "photo.png",
            PNG.len(),
        )])]);
        let client = JiraClient::new(&transport, SITE);

        let bytes = read_attachment_image(&client, &key(), "10002", downloads.path()).unwrap();

        assert_eq!(bytes, PNG);
        assert_eq!(transport.requests().len(), 1);
    }

    #[test]
    fn a_file_that_is_not_an_image_is_not_read_for_viewing() {
        let downloads = tempfile::tempdir().unwrap();
        let transport =
            FakeTransport::with_responses(vec![issue_with(vec![attachment("10001", "bench.log", 3)])])
                .with_downloads(vec![Ok(b"jam".to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let error = read_attachment_image(&client, &key(), "10001", downloads.path()).unwrap_err();

        let JiraError::InvalidInput(message) = error else {
            panic!("expected invalid input");
        };
        assert!(message.contains("bench.log"), "{message}");
    }

    #[test]
    fn an_image_over_the_viewing_limit_is_not_read() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![attachment(
            "10002",
            "photo.png",
            PNG.len(),
        )])])
        .with_downloads(vec![Ok(PNG.to_vec())]);
        let client = JiraClient::new(&transport, SITE);

        let error = read_image_within(
            &client,
            &key(),
            "10002",
            downloads.path(),
            PNG.len() as u64 - 1,
        )
        .unwrap_err();

        let JiraError::InvalidInput(message) = error else {
            panic!("expected invalid input");
        };
        assert!(message.contains("too large"), "{message}");
        // The file stays downloaded, so it can be opened from its folder.
        assert!(issue_download_dir(downloads.path(), &key())
            .join("photo.png")
            .exists());
    }

    #[test]
    fn a_failed_download_leaves_no_partial_file() {
        let downloads = tempfile::tempdir().unwrap();
        let transport = FakeTransport::with_responses(vec![issue_with(vec![
            attachment("10001", "bench.log", 3),
            attachment("10002", "photo.png", 4),
        ])])
        .with_downloads(vec![
            Ok(b"jam".to_vec()),
            Err(JiraError::Network("connection reset".to_string())),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let error = download_attachments(&client, &key(), None, downloads.path()).unwrap_err();

        assert_eq!(error, JiraError::Network("connection reset".to_string()));
        assert_eq!(
            file_names(&issue_download_dir(downloads.path(), &key())),
            vec!["bench.log"]
        );
    }
}
