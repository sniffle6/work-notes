//! Colour themes the user builds with the theme agent.
//!
//! The agent runs through local `codex exec` and returns JSON matching
//! `schemas/theme-agent.schema.json`: a name, a note, and one colour per
//! token. The app normalizes the colours, works out whether the theme is
//! dark or light, and checks that text stays readable.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::parser::run_structured_exec;

pub const THEME_SCHEMA_JSON: &str = include_str!("../../schemas/theme-agent.schema.json");
/// WCAG AA contrast for body text.
const MIN_TEXT_CONTRAST: f64 = 4.5;
/// WCAG contrast for large text and meaningful non-text marks.
const MIN_MARK_CONTRAST: f64 = 3.0;
/// Backgrounds darker than this are treated as a dark theme.
const DARK_MODE_MAX_LUMINANCE: f64 = 0.4;
const MAX_REQUEST_CHARS: usize = 2_000;
pub const MAX_THEME_NAME_CHARS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

/// One colour per theme token, each as `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTokens {
    pub app_bg: String,
    #[serde(rename = "surface1")]
    pub surface_1: String,
    #[serde(rename = "surface2")]
    pub surface_2: String,
    pub surface_input: String,
    pub border_default: String,
    pub border_strong: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent_primary: String,
    pub accent_hot: String,
    pub status_success: String,
    pub status_warning: String,
    pub status_error: String,
}

impl ThemeTokens {
    fn fields_mut(&mut self) -> [(&'static str, &mut String); 13] {
        [
            ("appBg", &mut self.app_bg),
            ("surface1", &mut self.surface_1),
            ("surface2", &mut self.surface_2),
            ("surfaceInput", &mut self.surface_input),
            ("borderDefault", &mut self.border_default),
            ("borderStrong", &mut self.border_strong),
            ("textPrimary", &mut self.text_primary),
            ("textMuted", &mut self.text_muted),
            ("accentPrimary", &mut self.accent_primary),
            ("accentHot", &mut self.accent_hot),
            ("statusSuccess", &mut self.status_success),
            ("statusWarning", &mut self.status_warning),
            ("statusError", &mut self.status_error),
        ]
    }

    /// Rewrites every colour as lowercase `#rrggbb`. Fails naming the first
    /// token whose value is not a hex colour.
    pub fn normalized(mut self) -> Result<Self, String> {
        for (name, value) in self.fields_mut() {
            *value = normalize_hex(value)
                .ok_or_else(|| format!("{name} must be a hex colour like #1a2b3c"))?;
        }
        Ok(self)
    }

    /// Dark or light, judged from the app background.
    pub fn mode(&self) -> ThemeMode {
        if relative_luminance(&self.app_bg) < DARK_MODE_MAX_LUMINANCE {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        }
    }
}

/// A theme the agent produced, ready to preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedTheme {
    pub name: String,
    pub mode: ThemeMode,
    pub tokens: ThemeTokens,
    /// The agent's one-line account of the choices it made.
    pub notes: String,
    /// Readability problems that remain after the agent's repair attempt.
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThemeAgentError {
    #[error("describe the theme you want")]
    EmptyRequest,
    #[error("the theme description is too long")]
    RequestTooLong,
    #[error("the theme agent could not run: {0}")]
    Run(String),
    #[error("the theme agent returned something unusable: {0}")]
    InvalidOutput(String),
}

/// Runs one theme agent turn and returns its raw JSON output.
pub trait ThemeRunner {
    fn run_turn(&self, prompt: &str) -> Result<String, String>;
}

/// Runs theme agent turns through local `codex exec`.
pub struct CodexThemeRunner {
    pub program: String,
    pub timeout: Duration,
}

impl ThemeRunner for CodexThemeRunner {
    fn run_turn(&self, prompt: &str) -> Result<String, String> {
        run_structured_exec(&self.program, THEME_SCHEMA_JSON, prompt, self.timeout)
            .map_err(|error| error.to_string())
    }
}

/// Accepts `#rgb` or `#rrggbb` in any case and returns lowercase `#rrggbb`.
pub fn normalize_hex(value: &str) -> Option<String> {
    let digits = value.trim().strip_prefix('#')?;
    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    match digits.len() {
        3 => Some(format!(
            "#{}",
            digits
                .chars()
                .flat_map(|digit| [digit, digit])
                .collect::<String>()
                .to_ascii_lowercase()
        )),
        6 => Some(format!("#{}", digits.to_ascii_lowercase())),
        _ => None,
    }
}

/// WCAG relative luminance of a normalized `#rrggbb` colour. Anything else
/// counts as black.
pub fn relative_luminance(hex: &str) -> f64 {
    let channel = |offset: usize| -> f64 {
        let value = hex
            .get(offset..offset + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .map_or(0.0, |byte| f64::from(byte) / 255.0);
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

/// WCAG contrast ratio between two normalized colours, from 1 to 21.
pub fn contrast_ratio(first: &str, second: &str) -> f64 {
    let (first, second) = (relative_luminance(first), relative_luminance(second));
    let (lighter, darker) = if first > second {
        (first, second)
    } else {
        (second, first)
    };
    (lighter + 0.05) / (darker + 0.05)
}

/// Lists the places where a theme would be hard to read. Empty means the
/// theme passes.
pub fn contrast_problems(tokens: &ThemeTokens) -> Vec<String> {
    let checks: [(&str, &str, &str, &str, f64); 11] = [
        (
            "textPrimary",
            &tokens.text_primary,
            "appBg",
            &tokens.app_bg,
            MIN_TEXT_CONTRAST,
        ),
        (
            "textPrimary",
            &tokens.text_primary,
            "surface1",
            &tokens.surface_1,
            MIN_TEXT_CONTRAST,
        ),
        (
            "textPrimary",
            &tokens.text_primary,
            "surface2",
            &tokens.surface_2,
            MIN_TEXT_CONTRAST,
        ),
        (
            "textPrimary",
            &tokens.text_primary,
            "surfaceInput",
            &tokens.surface_input,
            MIN_TEXT_CONTRAST,
        ),
        (
            "textMuted",
            &tokens.text_muted,
            "appBg",
            &tokens.app_bg,
            MIN_TEXT_CONTRAST,
        ),
        (
            "textMuted",
            &tokens.text_muted,
            "surface1",
            &tokens.surface_1,
            MIN_TEXT_CONTRAST,
        ),
        (
            "accentHot",
            &tokens.accent_hot,
            "appBg",
            &tokens.app_bg,
            MIN_MARK_CONTRAST,
        ),
        (
            "accentHot",
            &tokens.accent_hot,
            "surface2",
            &tokens.surface_2,
            MIN_MARK_CONTRAST,
        ),
        (
            "statusSuccess",
            &tokens.status_success,
            "appBg",
            &tokens.app_bg,
            MIN_MARK_CONTRAST,
        ),
        (
            "statusWarning",
            &tokens.status_warning,
            "appBg",
            &tokens.app_bg,
            MIN_MARK_CONTRAST,
        ),
        (
            "statusError",
            &tokens.status_error,
            "appBg",
            &tokens.app_bg,
            MIN_MARK_CONTRAST,
        ),
    ];

    checks
        .into_iter()
        .filter_map(|(text_name, text, surface_name, surface, minimum)| {
            let ratio = contrast_ratio(text, surface);
            (ratio < minimum).then(|| {
                format!(
                    "{text_name} on {surface_name} has contrast {ratio:.1}; it needs at least {minimum:.1}"
                )
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTheme {
    name: String,
    notes: String,
    tokens: ThemeTokens,
}

fn parse_theme(raw: &str) -> Result<RawTheme, ThemeAgentError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| ThemeAgentError::InvalidOutput(error.to_string()))?;
    let schema: Value = serde_json::from_str(THEME_SCHEMA_JSON)
        .map_err(|error| ThemeAgentError::InvalidOutput(format!("schema is invalid: {error}")))?;
    let validator = jsonschema::validator_for(&schema).map_err(|error| {
        ThemeAgentError::InvalidOutput(format!("schema could not be compiled: {error}"))
    })?;
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if !errors.is_empty() {
        return Err(ThemeAgentError::InvalidOutput(errors.join("; ")));
    }

    let mut theme: RawTheme = serde_json::from_value(value)
        .map_err(|error| ThemeAgentError::InvalidOutput(error.to_string()))?;
    theme.tokens = theme
        .tokens
        .normalized()
        .map_err(ThemeAgentError::InvalidOutput)?;
    Ok(theme)
}

const THEME_INSTRUCTIONS: &str = r###"You design colour themes for Work Notes, a Windows desktop app where an engineer captures notes and works Jira tickets. The interface is text-first: lists, long reading text, and thin rules instead of boxes.

Respond only with JSON matching the output schema. Do not run shell commands, read files, or browse.

Output:
- `name`: a short name for the theme, at most three words, no quotes.
- `notes`: one sentence on the look you went for.
- `tokens`: one colour per token, each as #rrggbb.

What each token paints:
- appBg: the window background behind everything.
- surface1: side panels and dialogs. Close to appBg.
- surface2: hovered and selected rows. A clear step away from appBg.
- surfaceInput: text fields and code blocks.
- borderDefault: thin dividing rules and field outlines. Subtle but visible on appBg.
- borderStrong: outlines that must stand out, such as an unticked checkbox.
- textPrimary: body text and headings.
- textMuted: secondary text such as dates, counts, and labels.
- accentPrimary: tinted fills behind the main button and the active item. Mixed into backgrounds at low strength.
- accentHot: the colour of emphasis in text: links, ticket keys, focus rings, the active tab marker.
- statusSuccess, statusWarning, statusError: done, waiting or due, and failed or overdue.

Rules:
- Pick either a dark theme or a light theme and keep every surface on that side.
- textPrimary must contrast at least 4.5:1 with appBg, surface1, surface2, and surfaceInput.
- textMuted must contrast at least 4.5:1 with appBg and surface1.
- accentHot and the three status colours must contrast at least 3:1 with appBg. accentHot must also reach 3:1 on surface2.
- Keep the three status colours recognisable as green, amber, and red families unless the user asks otherwise.
- Take the palette from what the user describes. Do not fall back to a blue-black background with a teal or violet accent unless asked.
"###;

/// Builds the prompt for one theme agent turn. `base` is the theme being
/// refined, and `problems` lists what the previous attempt got wrong.
pub fn build_theme_prompt(
    request: &str,
    base: Option<&GeneratedTheme>,
    problems: &[String],
) -> String {
    let mut prompt = String::from(THEME_INSTRUCTIONS);

    if let Some(base) = base {
        prompt.push_str(
            "\nThe user is refining this theme. Change only what they ask for and keep the rest:\n",
        );
        prompt.push_str(&format!("name: {}\n", base.name));
        prompt.push_str(
            &serde_json::to_string_pretty(&base.tokens).unwrap_or_else(|_| "{}".to_string()),
        );
        prompt.push('\n');
    }

    if !problems.is_empty() {
        prompt.push_str("\nYour last attempt had these problems. Fix every one and return the whole theme again:\n");
        for problem in problems {
            prompt.push_str(&format!("- {problem}\n"));
        }
    }

    prompt.push_str("\nWhat the user asked for:\n");
    prompt.push_str(request);
    prompt.push('\n');
    prompt
}

/// Asks the agent for a theme, giving it one chance to repair readability
/// problems. Problems that survive the repair are returned as `warnings`.
///
/// Fails with `EmptyRequest` or `RequestTooLong` before the agent runs, `Run`
/// when the agent cannot run, and `InvalidOutput` when its answer does not
/// match the schema or holds a value that is not a hex colour.
pub fn generate_theme(
    runner: &dyn ThemeRunner,
    request: &str,
    base: Option<&GeneratedTheme>,
) -> Result<GeneratedTheme, ThemeAgentError> {
    let request = request.trim();
    if request.is_empty() {
        return Err(ThemeAgentError::EmptyRequest);
    }
    if request.chars().count() > MAX_REQUEST_CHARS {
        return Err(ThemeAgentError::RequestTooLong);
    }

    let attempt = |problems: &[String], base: Option<&GeneratedTheme>| {
        let raw = runner
            .run_turn(&build_theme_prompt(request, base, problems))
            .map_err(ThemeAgentError::Run)?;
        let theme = parse_theme(&raw)?;
        let warnings = contrast_problems(&theme.tokens);
        Ok::<_, ThemeAgentError>(GeneratedTheme {
            name: theme_name(&theme.name),
            mode: theme.tokens.mode(),
            tokens: theme.tokens,
            notes: theme.notes.trim().to_string(),
            warnings,
        })
    };

    let first = attempt(&[], base)?;
    if first.warnings.is_empty() {
        return Ok(first);
    }
    // The repair starts from the first attempt, so the agent keeps what worked.
    attempt(&first.warnings, Some(&first))
}

fn theme_name(raw: &str) -> String {
    let name: String = raw.trim().chars().take(MAX_THEME_NAME_CHARS).collect();
    if name.trim().is_empty() {
        "Custom theme".to_string()
    } else {
        name.trim().to_string()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use serde_json::json;

    use super::{
        build_theme_prompt, contrast_problems, contrast_ratio, generate_theme, normalize_hex,
        GeneratedTheme, ThemeAgentError, ThemeMode, ThemeRunner, ThemeTokens,
    };

    pub(crate) fn dark_tokens() -> ThemeTokens {
        ThemeTokens {
            app_bg: "#11151c".to_string(),
            surface_1: "#1b212b".to_string(),
            surface_2: "#202833".to_string(),
            surface_input: "#111720".to_string(),
            border_default: "#334052".to_string(),
            border_strong: "#4a5a70".to_string(),
            text_primary: "#edf4fb".to_string(),
            text_muted: "#93a3b7".to_string(),
            accent_primary: "#2f6f7a".to_string(),
            accent_hot: "#6fc7bd".to_string(),
            status_success: "#4f9f6f".to_string(),
            status_warning: "#d69a2d".to_string(),
            status_error: "#d05252".to_string(),
        }
    }

    fn output(name: &str, tokens: &ThemeTokens) -> Result<String, String> {
        Ok(json!({ "name": name, "notes": " Calm and dark. ", "tokens": tokens }).to_string())
    }

    struct FakeRunner {
        outputs: RefCell<VecDeque<Result<String, String>>>,
        prompts: RefCell<Vec<String>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<Result<String, String>>) -> Self {
            Self {
                outputs: RefCell::new(outputs.into()),
                prompts: RefCell::new(Vec::new()),
            }
        }
    }

    impl ThemeRunner for FakeRunner {
        fn run_turn(&self, prompt: &str) -> Result<String, String> {
            self.prompts.borrow_mut().push(prompt.to_string());
            self.outputs
                .borrow_mut()
                .pop_front()
                .expect("no fake theme output queued")
        }
    }

    /// Manual check with real Codex: the schema is accepted and the answer
    /// is a usable theme. Run with:
    /// `cargo test live_theme -- --ignored --nocapture`
    #[test]
    #[ignore = "runs codex exec"]
    fn live_theme_agent_builds_a_theme() {
        use crate::parser::DEFAULT_CODEX_PROGRAM;

        let runner = super::CodexThemeRunner {
            program: DEFAULT_CODEX_PROGRAM.to_string(),
            timeout: std::time::Duration::from_secs(120),
        };

        let theme = generate_theme(&runner, "amber phosphor terminal, low glare", None)
            .expect("theme agent run");

        println!("{} ({:?}): {}", theme.name, theme.mode, theme.notes);
        println!("{}", serde_json::to_string_pretty(&theme.tokens).unwrap());
        println!("warnings: {:?}", theme.warnings);
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    #[test]
    fn hex_colours_are_normalized_and_other_values_rejected() {
        assert_eq!(normalize_hex(" #ABCDEF "), Some("#abcdef".to_string()));
        assert_eq!(normalize_hex("#Fa0"), Some("#ffaa00".to_string()));
        for bad in ["abcdef", "#abcd", "#gggggg", "rgb(0,0,0)", ""] {
            assert_eq!(normalize_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn contrast_follows_the_wcag_formula() {
        assert!((contrast_ratio("#000000", "#ffffff") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#ffffff", "#000000") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#777777", "#ffffff") - 4.48).abs() < 0.01);
        assert!((contrast_ratio("#336699", "#336699") - 1.0).abs() < 0.001);
    }

    #[test]
    fn mode_comes_from_the_background() {
        assert_eq!(dark_tokens().mode(), ThemeMode::Dark);
        let light = ThemeTokens {
            app_bg: "#f6f0d8".to_string(),
            ..dark_tokens()
        };
        assert_eq!(light.mode(), ThemeMode::Light);
    }

    #[test]
    fn a_readable_theme_has_no_contrast_problems() {
        assert_eq!(contrast_problems(&dark_tokens()), Vec::<String>::new());
    }

    #[test]
    fn unreadable_text_and_marks_are_named_with_their_ratio() {
        let tokens = ThemeTokens {
            text_muted: "#3a4350".to_string(),
            status_error: "#3b1d1d".to_string(),
            ..dark_tokens()
        };

        let problems = contrast_problems(&tokens);

        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].starts_with("textMuted on appBg has contrast 1."));
        assert!(problems[0].ends_with("it needs at least 4.5"));
        assert!(problems[1].starts_with("textMuted on surface1"));
        assert!(problems[2].starts_with("statusError on appBg"));
        assert!(problems[2].ends_with("it needs at least 3.0"));
    }

    #[test]
    fn a_good_first_answer_is_returned_without_a_second_turn() {
        let runner = FakeRunner::new(vec![output("  Night Desk  ", &dark_tokens())]);

        let theme = generate_theme(&runner, "  a calm dark desk  ", None).unwrap();

        assert_eq!(
            theme,
            GeneratedTheme {
                name: "Night Desk".to_string(),
                mode: ThemeMode::Dark,
                tokens: dark_tokens(),
                notes: "Calm and dark.".to_string(),
                warnings: Vec::new(),
            }
        );
        let prompts = runner.prompts.borrow();
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].ends_with("What the user asked for:\na calm dark desk\n"));
        assert!(!prompts[0].contains("refining this theme"));
    }

    #[test]
    fn colours_are_normalized_and_an_empty_name_is_replaced() {
        let tokens = ThemeTokens {
            accent_hot: "#6FC7BD".to_string(),
            border_default: "#345".to_string(),
            ..dark_tokens()
        };
        let runner = FakeRunner::new(vec![output("   ", &tokens)]);

        let theme = generate_theme(&runner, "anything", None).unwrap();

        assert_eq!(theme.name, "Custom theme");
        assert_eq!(theme.tokens.accent_hot, "#6fc7bd");
        assert_eq!(theme.tokens.border_default, "#334455");
    }

    #[test]
    fn readability_problems_get_one_repair_turn_built_on_the_first_attempt() {
        let unreadable = ThemeTokens {
            text_muted: "#3a4350".to_string(),
            ..dark_tokens()
        };
        let runner = FakeRunner::new(vec![
            output("Dim", &unreadable),
            output("Dim", &dark_tokens()),
        ]);

        let theme = generate_theme(&runner, "dim grey", None).unwrap();

        assert_eq!(theme.tokens, dark_tokens());
        assert!(theme.warnings.is_empty());
        let prompts = runner.prompts.borrow();
        assert_eq!(prompts.len(), 2);
        assert!(prompts[1].contains("- textMuted on appBg has contrast"));
        assert!(prompts[1].contains("\"textMuted\": \"#3a4350\""));
    }

    #[test]
    fn problems_that_survive_the_repair_are_returned_as_warnings() {
        let unreadable = ThemeTokens {
            text_muted: "#3a4350".to_string(),
            ..dark_tokens()
        };
        let runner = FakeRunner::new(vec![output("Dim", &unreadable), output("Dim", &unreadable)]);

        let theme = generate_theme(&runner, "dim grey", None).unwrap();

        assert_eq!(theme.warnings.len(), 2);
        assert_eq!(runner.prompts.borrow().len(), 2);
    }

    #[test]
    fn refining_sends_the_current_theme_to_the_agent() {
        let base = GeneratedTheme {
            name: "Night Desk".to_string(),
            mode: ThemeMode::Dark,
            tokens: dark_tokens(),
            notes: String::new(),
            warnings: Vec::new(),
        };

        let prompt = build_theme_prompt("warmer accent", Some(&base), &[]);

        assert!(prompt.contains("The user is refining this theme."));
        assert!(prompt.contains("name: Night Desk"));
        assert!(prompt.contains("\"accentHot\": \"#6fc7bd\""));
        assert!(prompt.ends_with("What the user asked for:\nwarmer accent\n"));
    }

    #[test]
    fn bad_requests_fail_before_the_agent_runs() {
        let runner = FakeRunner::new(Vec::new());

        assert_eq!(
            generate_theme(&runner, "   ", None),
            Err(ThemeAgentError::EmptyRequest)
        );
        assert_eq!(
            generate_theme(&runner, &"x".repeat(2_001), None),
            Err(ThemeAgentError::RequestTooLong)
        );
        assert!(runner.prompts.borrow().is_empty());
    }

    #[test]
    fn agent_failures_and_unusable_output_are_reported() {
        let failing = FakeRunner::new(vec![Err("codex command was not found".to_string())]);
        assert_eq!(
            generate_theme(&failing, "anything", None),
            Err(ThemeAgentError::Run(
                "codex command was not found".to_string()
            ))
        );

        let prose = FakeRunner::new(vec![Ok("Here is a nice theme".to_string())]);
        assert!(matches!(
            generate_theme(&prose, "anything", None),
            Err(ThemeAgentError::InvalidOutput(_))
        ));

        let not_a_colour = ThemeTokens {
            accent_hot: "teal".to_string(),
            ..dark_tokens()
        };
        let named = FakeRunner::new(vec![output("Odd", &not_a_colour)]);
        assert_eq!(
            generate_theme(&named, "anything", None),
            Err(ThemeAgentError::InvalidOutput(
                "accentHot must be a hex colour like #1a2b3c".to_string()
            ))
        );
    }
}
