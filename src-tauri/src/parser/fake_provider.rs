use super::types::{ParserError, ParserProvider, ParserResult};

#[derive(Debug, Default, Clone, Copy)]
pub struct FakeParserProvider;

impl ParserProvider for FakeParserProvider {
    fn parse(&self, input: &str) -> Result<ParserResult, ParserError> {
        Ok(ParserResult {
            cleaned_text: input.to_string(),
            summary: "Parsed by fake provider.".to_string(),
            tags: Vec::new(),
            action_items: Vec::new(),
        })
    }
}
