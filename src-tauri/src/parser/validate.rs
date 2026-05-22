use super::types::ParserError;
use serde_json::Value;

const PARSE_NOTE_SCHEMA_JSON: &str = include_str!("../../../schemas/parse-note.schema.json");

pub fn validate_parser_json(value: &Value) -> Result<(), ParserError> {
    let schema: Value = serde_json::from_str(PARSE_NOTE_SCHEMA_JSON).map_err(|error| {
        ParserError::InvalidResult(format!("parse note schema JSON is invalid: {error}"))
    })?;
    let validator = jsonschema::validator_for(&schema).map_err(|error| {
        ParserError::InvalidResult(format!("parse note schema could not be compiled: {error}"))
    })?;

    let errors = validator
        .iter_errors(value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ParserError::InvalidResult(format!(
            "parser JSON failed schema validation: {}",
            errors.join("; ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::validate_parser_json;
    use serde_json::json;

    #[test]
    fn validates_schema_and_rejects_missing_title() {
        let valid_result = json!({
            "title": "Payroll Review",
            "cleanedText": "Ask Riley to review the payroll rollout by Friday.",
            "summary": "Payroll rollout review request.",
            "tags": [
                {
                    "name": "Riley",
                    "kind": "person",
                    "confidence": 0.95
                }
            ],
            "actionItems": [
                {
                    "text": "Review the payroll rollout",
                    "owner": "Riley",
                    "dueDate": "Friday",
                    "confidence": 0.82,
                    "requiresReview": true
                }
            ]
        });
        let missing_title = json!({
            "cleanedText": "Ask Riley to review the payroll rollout by Friday.",
            "summary": "Payroll rollout review request.",
            "tags": [],
            "actionItems": []
        });

        validate_parser_json(&valid_result).expect("valid parser JSON should pass schema");
        let error =
            validate_parser_json(&missing_title).expect_err("missing title should fail schema");

        assert!(error.to_string().contains("title"));
    }
}
