pub fn build_parse_prompt(raw_note: &str) -> String {
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

    format!("{}\n\nRaw note:\n{}\n", rules.join("\n"), raw_note)
}

#[cfg(test)]
mod tests {
    use super::build_parse_prompt;

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
}
