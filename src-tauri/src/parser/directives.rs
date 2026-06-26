/// A raw note split into the content to clean and the author's `@codex:`
/// directives, in the order they appear.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedNote {
    pub body: String,
    pub directives: Vec<String>,
}

const MARKER: &str = "@codex:";

/// Split a raw note into its body (content to clean) and an ordered list of
/// `@codex:` directives. See `docs/parser-contract.md` for the bounding rules.
pub fn extract_directives(raw_note: &str) -> ExtractedNote {
    let mut directives = Vec::new();
    let mut body_lines: Vec<&str> = Vec::new();
    let mut lines = raw_note.lines().peekable();

    while let Some(line) = lines.next() {
        let Some(remainder) = directive_start(line) else {
            body_lines.push(line);
            continue;
        };

        let mut parts: Vec<&str> = Vec::new();
        let remainder = remainder.trim();
        if !remainder.is_empty() {
            parts.push(remainder);
        }
        while let Some(&next) = lines.peek() {
            if next.trim().is_empty() || directive_start(next).is_some() {
                break;
            }
            parts.push(next.trim());
            lines.next();
        }

        if !parts.is_empty() {
            directives.push(parts.join(" "));
        }
    }

    ExtractedNote {
        body: body_lines.join("\n").trim().to_string(),
        directives,
    }
}

/// If `line` starts a directive — the case-insensitive `@codex:` marker after
/// optional leading whitespace — return the text after the marker.
fn directive_start(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let head = trimmed.get(..MARKER.len())?;
    head.eq_ignore_ascii_case(MARKER)
        .then(|| &trimmed[MARKER.len()..])
}

#[cfg(test)]
mod tests {
    use super::extract_directives;

    #[test]
    fn no_directive_returns_body_unchanged() {
        let note = "Met with Bob.\n\nShip date moved to Friday.";
        let extracted = extract_directives(note);
        assert_eq!(extracted.body, "Met with Bob.\n\nShip date moved to Friday.");
        assert!(extracted.directives.is_empty());
    }

    #[test]
    fn single_line_directive_is_extracted_and_removed_from_body() {
        let note = "@codex: tag urgent\n\nMet with Bob about the rollout.";
        let extracted = extract_directives(note);
        assert_eq!(extracted.directives, vec!["tag urgent".to_string()]);
        assert_eq!(extracted.body, "Met with Bob about the rollout.");
    }

    #[test]
    fn multi_line_directive_joins_continuation_lines_with_spaces() {
        let note = "@codex: rewrite as a checklist,\ntag urgent, and pull the version\nfrom repo-b\n\nMet with Bob about the rollout.";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["rewrite as a checklist, tag urgent, and pull the version from repo-b".to_string()]
        );
        assert_eq!(extracted.body, "Met with Bob about the rollout.");
    }

    #[test]
    fn stacked_directive_lines_stay_separate() {
        let note = "@codex: tag urgent\n@codex: keep it terse\n\nMet with Bob.";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["tag urgent".to_string(), "keep it terse".to_string()]
        );
        assert_eq!(extracted.body, "Met with Bob.");
    }

    #[test]
    fn marker_is_case_insensitive_and_allows_leading_whitespace() {
        let note = "   @CoDeX:   tag urgent  \n\nBody.";
        let extracted = extract_directives(note);
        assert_eq!(extracted.directives, vec!["tag urgent".to_string()]);
        assert_eq!(extracted.body, "Body.");
    }

    #[test]
    fn bare_marker_with_no_text_is_ignored() {
        let note = "@codex:\n\nMet with Bob.";
        let extracted = extract_directives(note);
        assert!(extracted.directives.is_empty());
        assert_eq!(extracted.body, "Met with Bob.");
    }

    #[test]
    fn content_without_a_blank_line_is_absorbed_into_the_directive() {
        let note = "@codex: tag urgent\nMet with Bob.";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["tag urgent Met with Bob.".to_string()]
        );
        assert_eq!(extracted.body, "");
    }

    #[test]
    fn directive_only_note_yields_empty_body() {
        let note = "@codex: summarize the standup as bullets";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["summarize the standup as bullets".to_string()]
        );
        assert_eq!(extracted.body, "");
    }

    #[test]
    fn marker_not_at_line_start_is_left_in_the_body() {
        let note = "Discuss the @codex: integration with Bob.";
        let extracted = extract_directives(note);
        assert!(extracted.directives.is_empty());
        assert_eq!(extracted.body, "Discuss the @codex: integration with Bob.");
    }
}
