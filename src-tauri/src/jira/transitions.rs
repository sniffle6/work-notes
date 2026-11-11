use super::types::JiraTransition;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransitionMatchError {
    #[error("a target status is required")]
    EmptyStatus,
    #[error("no transition matches \"{wanted}\"; available: {}", format_available(.available))]
    NoMatch {
        wanted: String,
        available: Vec<String>,
    },
    #[error("\"{wanted}\" matches more than one transition: {}", .candidates.join(", "))]
    Ambiguous {
        wanted: String,
        candidates: Vec<String>,
    },
}

fn format_available(available: &[String]) -> String {
    if available.is_empty() {
        "(none)".to_string()
    } else {
        available.join(", ")
    }
}

/// Finds the transition a user means by a status or transition name.
///
/// Matching ignores case and repeated whitespace. An exact match on the
/// transition name or its destination status wins; otherwise a single
/// substring match is accepted.
pub fn match_transition<'a>(
    transitions: &'a [JiraTransition],
    wanted: &str,
) -> Result<&'a JiraTransition, TransitionMatchError> {
    let needle = normalize_label(wanted);
    if needle.is_empty() {
        return Err(TransitionMatchError::EmptyStatus);
    }
    let labels = |transition: &JiraTransition| {
        [
            normalize_label(&transition.name),
            normalize_label(&transition.to_status),
        ]
    };
    let ambiguous = |candidates: &[&JiraTransition]| TransitionMatchError::Ambiguous {
        wanted: wanted.trim().to_string(),
        candidates: candidates
            .iter()
            .map(|transition| describe_transition(transition))
            .collect(),
    };

    let exact: Vec<&JiraTransition> = transitions
        .iter()
        .filter(|transition| labels(transition).contains(&needle))
        .collect();
    match exact.as_slice() {
        [only] => return Ok(*only),
        [] => {}
        many => return Err(ambiguous(many)),
    }

    let partial: Vec<&JiraTransition> = transitions
        .iter()
        .filter(|transition| {
            labels(transition)
                .iter()
                .any(|label| label.contains(&needle))
        })
        .collect();
    match partial.as_slice() {
        [only] => Ok(*only),
        [] => Err(TransitionMatchError::NoMatch {
            wanted: wanted.trim().to_string(),
            available: transitions.iter().map(describe_transition).collect(),
        }),
        many => Err(ambiguous(many)),
    }
}

/// Describes a transition as `name` or `name -> destination status`.
pub fn describe_transition(transition: &JiraTransition) -> String {
    if transition.name == transition.to_status || transition.to_status.is_empty() {
        transition.name.clone()
    } else {
        format!("{} -> {}", transition.name, transition.to_status)
    }
}

fn normalize_label(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{describe_transition, match_transition, TransitionMatchError};
    use crate::jira::types::JiraTransition;

    fn transition(id: &str, name: &str, to_status: &str) -> JiraTransition {
        JiraTransition {
            id: id.to_string(),
            name: name.to_string(),
            to_status: to_status.to_string(),
        }
    }

    fn workflow() -> Vec<JiraTransition> {
        vec![
            transition("11", "Start work", "In Progress"),
            transition("21", "Dev Complete", "Dev Complete"),
            transition("31", "Send to QA", "In QA"),
            transition("41", "Fail QA", "QA Failed"),
        ]
    }

    #[test]
    fn matches_destination_status_ignoring_case_and_spacing() {
        let transitions = workflow();

        let matched = match_transition(&transitions, "  in   PROGRESS ").unwrap();

        assert_eq!(matched.id, "11");
    }

    #[test]
    fn matches_transition_name() {
        let transitions = workflow();

        assert_eq!(
            match_transition(&transitions, "send to qa").unwrap().id,
            "31"
        );
    }

    #[test]
    fn exact_match_beats_partial_matches() {
        let transitions = vec![
            transition("1", "QA", "QA"),
            transition("2", "Fail QA", "QA Failed"),
        ];

        assert_eq!(match_transition(&transitions, "qa").unwrap().id, "1");
    }

    #[test]
    fn single_partial_match_is_accepted() {
        let transitions = workflow();

        assert_eq!(match_transition(&transitions, "complete").unwrap().id, "21");
    }

    #[test]
    fn ambiguous_partial_match_lists_candidates() {
        let transitions = workflow();

        let error = match_transition(&transitions, "qa").unwrap_err();

        assert_eq!(
            error,
            TransitionMatchError::Ambiguous {
                wanted: "qa".to_string(),
                candidates: vec![
                    "Send to QA -> In QA".to_string(),
                    "Fail QA -> QA Failed".to_string(),
                ],
            }
        );
    }

    #[test]
    fn ambiguous_exact_match_lists_candidates() {
        let transitions = vec![
            transition("1", "Reopen", "To Do"),
            transition("2", "Reset", "To Do"),
        ];

        let error = match_transition(&transitions, "to do").unwrap_err();

        assert!(
            matches!(error, TransitionMatchError::Ambiguous { candidates, .. } if candidates.len() == 2)
        );
    }

    #[test]
    fn missing_match_lists_available_transitions() {
        let transitions = workflow();

        let error = match_transition(&transitions, "Closed").unwrap_err();

        assert_eq!(
            error.to_string(),
            "no transition matches \"Closed\"; available: Start work -> In Progress, Dev Complete, \
             Send to QA -> In QA, Fail QA -> QA Failed"
        );
    }

    #[test]
    fn empty_status_is_rejected() {
        assert_eq!(
            match_transition(&workflow(), "   ").unwrap_err(),
            TransitionMatchError::EmptyStatus
        );
    }

    #[test]
    fn describes_transition_with_destination_only_when_it_differs() {
        assert_eq!(
            describe_transition(&transition("1", "Start work", "In Progress")),
            "Start work -> In Progress"
        );
        assert_eq!(
            describe_transition(&transition("2", "Done", "Done")),
            "Done"
        );
    }
}
