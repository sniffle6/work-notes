pub mod codex_provider;
pub mod fake_provider;
pub mod prompt;
pub mod result_applier;
pub mod types;
pub mod validate;

pub use codex_provider::{
    CodexCommandBuilder, CodexCommandSpec, CodexParserError, CodexParserProvider,
    DEFAULT_CODEX_PROGRAM, DEFAULT_SCHEMA_PATH, DEFAULT_TIMEOUT,
};
pub use fake_provider::FakeParserProvider;
pub use prompt::{build_parse_prompt, build_parse_prompt_with_feedback};
pub use result_applier::{
    ActionItemApplication, ParserResultApplier, ParserResultSink, TagApplication, AI_TAG_SOURCE,
    NEEDS_REVIEW_STATUS, SUGGESTED_ACTION_STATUS,
};
pub use types::{
    ParsedActionItem, ParsedTag, ParserError, ParserOutput, ParserProvider, ParserResult, TagKind,
};
pub use validate::validate_parser_json;

#[cfg(test)]
mod tests {
    use super::{
        fake_provider::FakeParserProvider,
        result_applier::{
            ActionItemApplication, ParserResultApplier, ParserResultSink, TagApplication,
            AI_TAG_SOURCE, NEEDS_REVIEW_STATUS, SUGGESTED_ACTION_STATUS,
        },
        types::{ParsedActionItem, ParsedTag, ParserProvider, ParserResult, TagKind},
    };
    use std::convert::Infallible;

    #[test]
    fn parser_result_deserializes_camel_case_json() {
        let json = r#"{
            "cleanedText": "Met with Alice about launch.",
            "summary": "Launch discussion.",
            "tags": [
                {
                    "kind": "person",
                    "name": "Alice",
                    "confidence": 0.98
                }
            ],
            "actionItems": [
                {
                    "text": "Send launch recap",
                    "owner": "Alice",
                    "dueDate": null,
                    "confidence": 0.84,
                    "requiresReview": true
                }
            ]
        }"#;

        let result: ParserResult = serde_json::from_str(json).expect("camelCase parser result");

        assert_eq!(result.cleaned_text, "Met with Alice about launch.");
        assert_eq!(result.summary, "Launch discussion.");
        assert_eq!(
            result.tags,
            vec![ParsedTag {
                kind: TagKind::Person,
                name: "Alice".to_string(),
                confidence: 0.98,
            }]
        );
        assert_eq!(
            result.action_items,
            vec![ParsedActionItem {
                text: "Send launch recap".to_string(),
                owner: Some("Alice".to_string()),
                due_date: None,
                confidence: 0.84,
                requires_review: true,
            }]
        );
    }

    #[test]
    fn fake_provider_returns_deterministic_parse_result() {
        let input = "Talked with Sam about payroll migration.";

        let first = FakeParserProvider::default()
            .parse(input)
            .expect("fake parse succeeds");
        let second = FakeParserProvider::default()
            .parse(input)
            .expect("fake parse succeeds");

        assert_eq!(first, second);
        assert_eq!(first.cleaned_text, input);
        assert_eq!(first.summary, "Parsed by fake provider.");
        assert!(first.tags.is_empty());
        assert!(first.action_items.is_empty());
    }

    #[test]
    fn result_applier_maps_parse_result_to_sink_contract() {
        let result = ParserResult {
            cleaned_text: "Ask Riley to review the rollout note.".to_string(),
            summary: "Rollout note follow-up.".to_string(),
            tags: vec![ParsedTag {
                kind: TagKind::Project,
                name: "rollout".to_string(),
                confidence: 0.91,
            }],
            action_items: vec![ParsedActionItem {
                text: "Review the rollout note".to_string(),
                owner: Some("Riley".to_string()),
                due_date: Some("2026-05-22".to_string()),
                confidence: 0.72,
                requires_review: true,
            }],
        };
        let mut sink = RecordingSink::default();

        ParserResultApplier::default()
            .apply(&mut sink, 7, &result)
            .expect("applier succeeds");

        assert_eq!(
            sink.cleaned_text,
            Some((
                7,
                "Ask Riley to review the rollout note.".to_string(),
                "Rollout note follow-up.".to_string(),
            ))
        );
        assert_eq!(
            sink.tags,
            vec![RecordedTag {
                note_id: 7,
                kind: TagKind::Project,
                name: "rollout".to_string(),
                confidence: 0.91,
                source: AI_TAG_SOURCE,
            }]
        );
        assert_eq!(
            sink.action_items,
            vec![RecordedActionItem {
                note_id: 7,
                text: "Review the rollout note".to_string(),
                owner: Some("Riley".to_string()),
                due_date: Some("2026-05-22".to_string()),
                confidence: 0.72,
                requires_review: true,
                status: SUGGESTED_ACTION_STATUS,
            }]
        );
        assert_eq!(sink.review_statuses, vec![(7, NEEDS_REVIEW_STATUS)]);
    }

    #[derive(Default)]
    struct RecordingSink {
        cleaned_text: Option<(u64, String, String)>,
        tags: Vec<RecordedTag>,
        action_items: Vec<RecordedActionItem>,
        review_statuses: Vec<(u64, &'static str)>,
    }

    impl ParserResultSink for RecordingSink {
        type Error = Infallible;
        type NoteId = u64;

        fn apply_cleaned_text(
            &mut self,
            note_id: Self::NoteId,
            cleaned_text: &str,
            summary: &str,
        ) -> Result<(), Self::Error> {
            self.cleaned_text = Some((note_id, cleaned_text.to_string(), summary.to_string()));
            Ok(())
        }

        fn add_tag(
            &mut self,
            note_id: Self::NoteId,
            tag: TagApplication<'_>,
        ) -> Result<(), Self::Error> {
            self.tags.push(RecordedTag {
                note_id,
                kind: tag.kind.clone(),
                name: tag.name.to_string(),
                confidence: tag.confidence,
                source: tag.source,
            });
            Ok(())
        }

        fn add_action_item(
            &mut self,
            note_id: Self::NoteId,
            action_item: ActionItemApplication<'_>,
        ) -> Result<(), Self::Error> {
            self.action_items.push(RecordedActionItem {
                note_id,
                text: action_item.text.to_string(),
                owner: action_item.owner.map(ToString::to_string),
                due_date: action_item.due_date.map(ToString::to_string),
                confidence: action_item.confidence,
                requires_review: action_item.requires_review,
                status: action_item.status,
            });
            Ok(())
        }

        fn set_review_status(
            &mut self,
            note_id: Self::NoteId,
            review_status: &'static str,
        ) -> Result<(), Self::Error> {
            self.review_statuses.push((note_id, review_status));
            Ok(())
        }
    }

    #[derive(Debug, PartialEq)]
    struct RecordedTag {
        note_id: u64,
        kind: TagKind,
        name: String,
        confidence: f32,
        source: &'static str,
    }

    #[derive(Debug, PartialEq)]
    struct RecordedActionItem {
        note_id: u64,
        text: String,
        owner: Option<String>,
        due_date: Option<String>,
        confidence: f32,
        requires_review: bool,
        status: &'static str,
    }
}
