pub fn build_parse_prompt(raw_note: &str) -> String {
    build_parse_prompt_with_feedback(raw_note, None)
}

pub fn build_parse_prompt_with_feedback(raw_note: &str, feedback: Option<&str>) -> String {
    let rules = [
        "You clean and organize a raw workplace note.",
        "Return only JSON matching the provided schema.",
        "Do not invent facts.",
        "Preserve the meaning of the raw note.",
        "Make cleanedText easier to scan and grammatically cleaner.",
        "Use tags for people, projects, topics, urgency, category, and custom labels.",
        "Extract action items when the note implies work to do.",
        "Set requiresReview true for due dates, owners, commitments, or inferred obligations.",
    ];

    let feedback = feedback
        .map(str::trim)
        .filter(|feedback| !feedback.is_empty())
        .map(|feedback| {
            format!(
                "\nReparse feedback:\n{}\nUse the feedback to revise the cleaned note, tags, summary, and actions.\n",
                feedback
            )
        })
        .unwrap_or_default();

    format!(
        "{}\n\nRaw note:\n{}\n{}",
        rules.join("\n"),
        raw_note,
        feedback
    )
}

#[cfg(test)]
mod tests {
    use super::{build_parse_prompt, build_parse_prompt_with_feedback};

    #[test]
    fn build_parse_prompt_includes_parser_rules_and_raw_note() {
        let raw_note = "Ask Riley to review the payroll rollout by Friday.";

        let prompt = build_parse_prompt(raw_note);

        for rule in [
            "You clean and organize a raw workplace note.",
            "Return only JSON matching the provided schema.",
            "Do not invent facts.",
            "Preserve the meaning of the raw note.",
            "Make cleanedText easier to scan and grammatically cleaner.",
            "Use tags for people, projects, topics, urgency, category, and custom labels.",
            "Extract action items when the note implies work to do.",
            "Set requiresReview true for due dates, owners, commitments, or inferred obligations.",
        ] {
            assert!(prompt.contains(rule), "missing parser rule: {rule}");
        }
        assert!(prompt.contains(raw_note));
    }

    #[test]
    fn build_parse_prompt_with_feedback_includes_revision_instructions() {
        let prompt = build_parse_prompt_with_feedback(
            "Robert said check local AI feasibility.",
            Some("Treat Robert as the requester and tag this as research."),
        );

        assert!(prompt.contains("Raw note:"));
        assert!(prompt.contains("Robert said check local AI feasibility."));
        assert!(prompt.contains("Reparse feedback:"));
        assert!(prompt.contains("Treat Robert as the requester and tag this as research."));
        assert!(prompt
            .contains("Use the feedback to revise the cleaned note, tags, summary, and actions."));
    }
}
