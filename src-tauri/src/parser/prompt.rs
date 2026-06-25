pub fn build_parse_prompt(raw_note: &str) -> String {
    build_parse_prompt_with_feedback(raw_note, None)
}

pub fn build_parse_prompt_with_feedback(raw_note: &str, feedback: Option<&str>) -> String {
    build_parse_prompt_with_context(raw_note, feedback, &[])
}

pub fn build_parse_prompt_with_context(
    raw_note: &str,
    feedback: Option<&str>,
    linked_workspace_paths: &[String],
) -> String {
    let rules = [
        "You clean and organize a raw workplace note.",
        "Return only JSON matching the provided schema.",
        "Do not invent facts.",
        "Preserve the meaning of the raw note.",
        "Create title as a 3-8 word task-oriented label that is easy to scan in an inbox.",
        "Make cleanedText GitHub-flavored Markdown that cleans the note without erasing the source structure.",
        "Use headings and bullets for ordinary shorthand notes.",
        "Use fenced code blocks for multi-line code, config, enum, command, stack trace, or log snippets, preserving those snippets mostly verbatim.",
        "Keep single identifiers, methods, file names, fields, and enum values as inline code.",
        "For copied tickets, preserve the ticket labels and order such as Actual, Expected, Steps, Environment, and Notes instead of rewriting the ticket into unrelated categories.",
        "Expand shorthand lightly, but do not over-summarize or split one coherent technical artifact into many bullets.",
        "Do not repeat title as the first cleanedText heading.",
        "Make summary one concise sentence.",
        "Use tags for people, projects, topics, urgency, category, and custom labels.",
        "Extract action items when the note implies work to do.",
        "Set requiresReview true for due dates, owners, commitments, or inferred obligations.",
        "If linked repo or directory context is provided, inspect it only when it is useful for understanding the note or checking a task.",
        "Do not claim repo facts unless you inspected the linked context or the raw note explicitly states them.",
    ];

    let linked_context = if linked_workspace_paths.is_empty() {
        String::new()
    } else {
        format!(
            "\nLinked repos/directories available for optional read-only inspection. Each is an absolute directory root you may read from directly by its full path; they are equally available and none is the primary repo:\n{}\n",
            linked_workspace_paths
                .iter()
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let feedback = feedback
        .map(str::trim)
        .filter(|feedback| !feedback.is_empty())
        .map(|feedback| {
            format!(
                "\nReparse feedback:\n{}\nUse the feedback to revise the title, Markdown cleaned note, tags, summary, and actions.\n",
                feedback
            )
        })
        .unwrap_or_default();

    format!(
        "{}\n{}\nRaw note:\n{}\n{}",
        rules.join("\n"),
        linked_context,
        raw_note,
        feedback
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_parse_prompt, build_parse_prompt_with_context, build_parse_prompt_with_feedback,
    };

    #[test]
    fn build_parse_prompt_includes_parser_rules_and_raw_note() {
        let raw_note = "Ask Riley to review the payroll rollout by Friday.";

        let prompt = build_parse_prompt(raw_note);

        for rule in [
            "You clean and organize a raw workplace note.",
            "Return only JSON matching the provided schema.",
            "Do not invent facts.",
            "Preserve the meaning of the raw note.",
            "Create title as a 3-8 word task-oriented label that is easy to scan in an inbox.",
            "Make cleanedText GitHub-flavored Markdown that cleans the note without erasing the source structure.",
            "Use headings and bullets for ordinary shorthand notes.",
            "Use fenced code blocks for multi-line code, config, enum, command, stack trace, or log snippets, preserving those snippets mostly verbatim.",
            "Keep single identifiers, methods, file names, fields, and enum values as inline code.",
            "For copied tickets, preserve the ticket labels and order such as Actual, Expected, Steps, Environment, and Notes instead of rewriting the ticket into unrelated categories.",
            "Expand shorthand lightly, but do not over-summarize or split one coherent technical artifact into many bullets.",
            "Do not repeat title as the first cleanedText heading.",
            "Make summary one concise sentence.",
            "Use tags for people, projects, topics, urgency, category, and custom labels.",
            "Extract action items when the note implies work to do.",
            "Set requiresReview true for due dates, owners, commitments, or inferred obligations.",
            "If linked repo or directory context is provided, inspect it only when it is useful for understanding the note or checking a task.",
            "Do not claim repo facts unless you inspected the linked context or the raw note explicitly states them.",
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
            .contains("Use the feedback to revise the title, Markdown cleaned note, tags, summary, and actions."));
    }

    #[test]
    fn build_parse_prompt_with_context_lists_linked_workspaces() {
        let prompt = build_parse_prompt_with_context(
            "Review this task against the local checkout.",
            None,
            &[
                "C:\\code\\product".to_string(),
                "D:\\scratch\\other".to_string(),
            ],
        );

        assert!(prompt.contains("available for optional read-only inspection"));
        assert!(prompt.contains("none is the primary repo"));
        assert!(prompt.contains("- C:\\code\\product"));
        assert!(prompt.contains("- D:\\scratch\\other"));
    }
}
