use super::types::{ParserResult, TagKind};

pub const AI_TAG_SOURCE: &str = "ai";
pub const SUGGESTED_ACTION_STATUS: &str = "suggested";
pub const NEEDS_REVIEW_STATUS: &str = "needs_review";

#[derive(Debug, Default, Clone, Copy)]
pub struct ParserResultApplier;

impl ParserResultApplier {
    pub fn apply<S>(
        &self,
        sink: &mut S,
        note_id: S::NoteId,
        result: &ParserResult,
    ) -> Result<(), S::Error>
    where
        S: ParserResultSink,
    {
        sink.apply_cleaned_text(note_id, &result.cleaned_text, &result.summary)?;

        for tag in &result.tags {
            sink.add_tag(
                note_id,
                TagApplication {
                    kind: &tag.kind,
                    name: &tag.name,
                    confidence: tag.confidence,
                    source: AI_TAG_SOURCE,
                },
            )?;
        }

        let needs_review = result
            .action_items
            .iter()
            .any(|action_item| action_item.requires_review);

        for action_item in &result.action_items {
            sink.add_action_item(
                note_id,
                ActionItemApplication {
                    text: &action_item.text,
                    owner: action_item.owner.as_deref(),
                    due_date: action_item.due_date.as_deref(),
                    confidence: action_item.confidence,
                    requires_review: action_item.requires_review,
                    status: SUGGESTED_ACTION_STATUS,
                },
            )?;
        }

        if needs_review {
            sink.set_review_status(note_id, NEEDS_REVIEW_STATUS)?;
        }

        Ok(())
    }
}

pub trait ParserResultSink {
    type Error;
    type NoteId: Copy;

    fn apply_cleaned_text(
        &mut self,
        note_id: Self::NoteId,
        cleaned_text: &str,
        summary: &str,
    ) -> Result<(), Self::Error>;

    fn add_tag(
        &mut self,
        note_id: Self::NoteId,
        tag: TagApplication<'_>,
    ) -> Result<(), Self::Error>;

    fn add_action_item(
        &mut self,
        note_id: Self::NoteId,
        action_item: ActionItemApplication<'_>,
    ) -> Result<(), Self::Error>;

    fn set_review_status(
        &mut self,
        note_id: Self::NoteId,
        review_status: &'static str,
    ) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagApplication<'a> {
    pub kind: &'a TagKind,
    pub name: &'a str,
    pub confidence: f32,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionItemApplication<'a> {
    pub text: &'a str,
    pub owner: Option<&'a str>,
    pub due_date: Option<&'a str>,
    pub confidence: f32,
    pub requires_review: bool,
    pub status: &'static str,
}
