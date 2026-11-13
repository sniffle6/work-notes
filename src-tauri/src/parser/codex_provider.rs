use super::{
    directives::extract_directives,
    prompt::build_parse_prompt_with_context,
    types::{ParserError, ParserOutput, ParserProvider, ParserResult},
    validate::validate_parser_json,
};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;
use thiserror::Error;

#[cfg(windows)]
pub const DEFAULT_CODEX_PROGRAM: &str = "codex.cmd";
#[cfg(not(windows))]
pub const DEFAULT_CODEX_PROGRAM: &str = "codex";
pub const DEFAULT_SCHEMA_PATH: &str = "schemas/parse-note.schema.json";
pub const DEFAULT_MODEL: &str = "gpt-5.6-luna";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_SCHEMA_JSON: &str = include_str!("../../../schemas/parse-note.schema.json");
/// How often a running command asks whether its parse was cancelled.
const CANCEL_CHECK_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CodexCommandBuilder {
    program: String,
    schema_path: String,
    output_path: String,
}

impl CodexCommandBuilder {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            schema_path: DEFAULT_SCHEMA_PATH.to_string(),
            output_path: "parse-result.json".to_string(),
        }
    }

    pub fn schema_path(mut self, schema_path: impl AsRef<Path>) -> Self {
        self.schema_path = path_to_arg(schema_path);
        self
    }

    pub fn output_path(mut self, output_path: impl AsRef<Path>) -> Self {
        self.output_path = path_to_arg(output_path);
        self
    }

    pub fn build(self) -> CodexCommandSpec {
        CodexCommandSpec {
            program: self.program,
            args: vec![
                "exec".to_string(),
                "--ephemeral".to_string(),
                "--skip-git-repo-check".to_string(),
                "--model".to_string(),
                DEFAULT_MODEL.to_string(),
                "-s".to_string(),
                "read-only".to_string(),
                "--output-schema".to_string(),
                self.schema_path,
                "-o".to_string(),
                self.output_path,
                "-".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodexParserProvider {
    program: String,
    schema_path: String,
    timeout: Duration,
    linked_workspace_paths: Vec<PathBuf>,
}

impl Default for CodexParserProvider {
    fn default() -> Self {
        Self {
            program: DEFAULT_CODEX_PROGRAM.to_string(),
            schema_path: DEFAULT_SCHEMA_PATH.to_string(),
            timeout: DEFAULT_TIMEOUT,
            linked_workspace_paths: Vec::new(),
        }
    }
}

impl CodexParserProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    pub fn schema_path(mut self, schema_path: impl AsRef<Path>) -> Self {
        self.schema_path = path_to_arg(schema_path);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn linked_workspace_paths(mut self, paths: Vec<String>) -> Self {
        self.linked_workspace_paths = paths.into_iter().map(PathBuf::from).collect();
        self
    }

    pub fn parse_with_typed_errors(
        &self,
        raw_note: &str,
    ) -> Result<ParserResult, CodexParserError> {
        self.parse_output_with_typed_errors(raw_note)
            .map(|output| output.result)
    }

    pub fn parse_output_with_typed_errors(
        &self,
        raw_note: &str,
    ) -> Result<ParserOutput, CodexParserError> {
        self.parse_output_with_feedback(raw_note, None)
    }

    pub fn parse_output_with_feedback(
        &self,
        raw_note: &str,
        feedback: Option<&str>,
    ) -> Result<ParserOutput, CodexParserError> {
        self.parse_output_cancellable(raw_note, feedback, &|| false)
    }

    /// Parses like [`Self::parse_output_with_feedback`], but stops the Codex
    /// command and fails with [`CodexParserError::Cancelled`] once
    /// `cancelled` returns true. `cancelled` is asked a few times a second.
    pub fn parse_output_cancellable(
        &self,
        raw_note: &str,
        feedback: Option<&str>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ParserOutput, CodexParserError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("work-notes-codex-parser-")
            .tempdir()
            .map_err(|source| CodexParserError::Io {
                context: "create temporary parser output directory",
                source,
            })?;
        let output_path = temp_output_path(&temp_dir);
        let output_arg = output_path.to_string_lossy().into_owned();
        let schema_path = default_schema_arg_path(&temp_dir, &self.schema_path)?;
        let command = CodexCommandBuilder::new(self.program.clone())
            .schema_path(schema_path)
            .output_path(output_arg)
            .build();
        let active_workspace_paths = self.active_linked_workspace_paths();
        let prompt_workspace_paths = active_workspace_paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let working_dir = active_workspace_paths.first().map(PathBuf::as_path);

        let extracted = extract_directives(raw_note);

        run_codex_command(
            &command,
            &build_parse_prompt_with_context(
                &extracted.body,
                feedback,
                &prompt_workspace_paths,
                &extracted.directives,
            ),
            self.timeout,
            working_dir,
            cancelled,
        )?;
        read_parser_result(output_path)
    }

    fn active_linked_workspace_paths(&self) -> Vec<PathBuf> {
        self.linked_workspace_paths
            .iter()
            .filter(|path| path.is_dir())
            .cloned()
            .collect()
    }
}

/// Runs one `codex exec` turn whose final message must match `schema_json`.
///
/// Returns the raw text Codex wrote to its output file; the caller validates
/// it. The process uses the read-only sandbox and an empty temporary working
/// directory, and is killed after `timeout`.
pub fn run_structured_exec(
    program: &str,
    schema_json: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<String, CodexParserError> {
    let temp_dir = tempfile::Builder::new()
        .prefix("work-notes-codex-exec-")
        .tempdir()
        .map_err(|source| CodexParserError::Io {
            context: "create temporary codex working directory",
            source,
        })?;
    let schema_path = temp_dir.path().join("output-schema.json");
    fs::write(&schema_path, schema_json).map_err(|source| CodexParserError::Io {
        context: "write codex output schema to temporary file",
        source,
    })?;
    let output_path = temp_dir.path().join("output.json");
    let command = CodexCommandBuilder::new(program)
        .schema_path(&schema_path)
        .output_path(&output_path)
        .build();

    run_codex_command(&command, prompt, timeout, Some(temp_dir.path()), &|| false)?;

    fs::read_to_string(&output_path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            CodexParserError::MissingOutput {
                path: output_path.clone(),
            }
        } else {
            CodexParserError::Io {
                context: "read codex output",
                source,
            }
        }
    })
}

fn path_to_arg(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

impl ParserProvider for CodexParserProvider {
    fn parse(&self, input: &str) -> Result<ParserResult, ParserError> {
        self.parse_with_typed_errors(input).map_err(Into::into)
    }

    fn parse_output(&self, input: &str) -> Result<ParserOutput, ParserError> {
        self.parse_output_with_typed_errors(input)
            .map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum CodexParserError {
    #[error("codex command was not found: {program}")]
    MissingCommand { program: String },
    #[error("codex command timed out after {timeout:?}")]
    Timeout { timeout: Duration },
    #[error("codex command was cancelled")]
    Cancelled,
    #[error("codex command exited with status {code:?}: {stderr}")]
    NonZeroExit { code: Option<i32>, stderr: String },
    #[error("codex parser output file was not written: {path:?}")]
    MissingOutput { path: PathBuf },
    #[error("codex parser output was not valid JSON: {source}")]
    InvalidJson { source: serde_json::Error },
    #[error("codex parser output failed schema validation: {message}")]
    SchemaValidation { message: String },
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: io::Error,
    },
}

impl From<CodexParserError> for ParserError {
    fn from(error: CodexParserError) -> Self {
        match error {
            CodexParserError::MissingOutput { .. }
            | CodexParserError::InvalidJson { .. }
            | CodexParserError::SchemaValidation { .. } => {
                ParserError::InvalidResult(error.to_string())
            }
            CodexParserError::MissingCommand { .. }
            | CodexParserError::Timeout { .. }
            | CodexParserError::Cancelled
            | CodexParserError::NonZeroExit { .. }
            | CodexParserError::Io { .. } => ParserError::Provider(error.to_string()),
        }
    }
}

fn temp_output_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join("parse-result.json")
}

fn default_schema_arg_path(
    temp_dir: &TempDir,
    schema_path: &str,
) -> Result<PathBuf, CodexParserError> {
    if schema_path != DEFAULT_SCHEMA_PATH {
        return Ok(PathBuf::from(schema_path));
    }

    let runtime_schema_path = temp_dir.path().join("parse-note.schema.json");
    fs::write(&runtime_schema_path, DEFAULT_SCHEMA_JSON).map_err(|source| {
        CodexParserError::Io {
            context: "write parser schema to temporary file",
            source,
        }
    })?;
    Ok(runtime_schema_path)
}

fn run_codex_command(
    command: &CodexCommandSpec,
    stdin_body: &str,
    timeout: Duration,
    working_dir: Option<&Path>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), CodexParserError> {
    let mut process = Command::new(&command.program);
    process
        .args(&command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(working_dir) = working_dir {
        process.current_dir(working_dir);
    }

    let mut child = process.spawn().map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            CodexParserError::MissingCommand {
                program: command.program.clone(),
            }
        } else {
            CodexParserError::Io {
                context: "spawn codex parser command",
                source,
            }
        }
    })?;
    let stderr_reader = child.stderr.take().map(|mut pipe| {
        thread::spawn(move || {
            let mut stderr = String::new();
            pipe.read_to_string(&mut stderr).map(|_| stderr)
        })
    });

    let stdin_writer = child.stdin.take().map(|mut stdin| {
        let stdin_body = stdin_body.to_string();
        thread::spawn(move || write_stdin(&mut stdin, &stdin_body))
    });

    let status = wait_with_timeout(&mut child, timeout, cancelled)?;
    read_stdin_result(stdin_writer)?;
    let stderr = read_stderr(stderr_reader)?;

    if status.success() {
        Ok(())
    } else {
        Err(CodexParserError::NonZeroExit {
            code: status.code(),
            stderr,
        })
    }
}

fn write_stdin(stdin: &mut std::process::ChildStdin, stdin_body: &str) -> io::Result<()> {
    match stdin.write_all(stdin_body.as_bytes()) {
        Ok(()) => stdin.flush(),
        Err(source) if source.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(source) => Err(source),
    }
}

fn read_stdin_result(
    writer: Option<thread::JoinHandle<io::Result<()>>>,
) -> Result<(), CodexParserError> {
    let Some(writer) = writer else {
        return Ok(());
    };

    writer
        .join()
        .map_err(|_| CodexParserError::Io {
            context: "write parser prompt to codex stdin",
            source: io::Error::new(io::ErrorKind::Other, "stdin writer panicked"),
        })?
        .map_err(|source| CodexParserError::Io {
            context: "write parser prompt to codex stdin",
            source,
        })
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
    cancelled: &dyn Fn() -> bool,
) -> Result<ExitStatus, CodexParserError> {
    let started_at = Instant::now();
    let mut cancel_checked_at = started_at;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started_at.elapsed() >= timeout => {
                kill_process_tree(child);
                return Err(CodexParserError::Timeout { timeout });
            }
            Ok(None) if cancel_checked_at.elapsed() >= CANCEL_CHECK_INTERVAL => {
                cancel_checked_at = Instant::now();
                if cancelled() {
                    kill_process_tree(child);
                    return Err(CodexParserError::Cancelled);
                }
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(source) => {
                return Err(CodexParserError::Io {
                    context: "wait for codex parser command",
                    source,
                });
            }
        }
    }
}

/// Stops the command and waits for it to exit.
///
/// On Windows `codex.cmd` runs Codex as a child of `cmd.exe`, and killing
/// `cmd.exe` alone leaves Codex running, so the whole process tree is ended.
fn kill_process_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Keeps the helper from flashing a console window over the app.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn read_stderr(
    reader: Option<thread::JoinHandle<io::Result<String>>>,
) -> Result<String, CodexParserError> {
    let Some(reader) = reader else {
        return Ok(String::new());
    };

    reader
        .join()
        .map_err(|_| CodexParserError::Io {
            context: "read codex parser stderr",
            source: io::Error::new(io::ErrorKind::Other, "stderr reader panicked"),
        })?
        .map_err(|source| CodexParserError::Io {
            context: "read codex parser stderr",
            source,
        })
}

fn read_parser_result(output_path: PathBuf) -> Result<ParserOutput, CodexParserError> {
    if !output_path.exists() {
        return Err(CodexParserError::MissingOutput { path: output_path });
    }

    let output = fs::read_to_string(&output_path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            CodexParserError::MissingOutput {
                path: output_path.clone(),
            }
        } else {
            CodexParserError::Io {
                context: "read codex parser output",
                source,
            }
        }
    })?;
    let value =
        serde_json::from_str(&output).map_err(|source| CodexParserError::InvalidJson { source })?;

    validate_parser_json(&value).map_err(|error| CodexParserError::SchemaValidation {
        message: error.to_string(),
    })?;
    let result =
        serde_json::from_value(value).map_err(|source| CodexParserError::InvalidJson { source })?;
    Ok(ParserOutput {
        raw_response: output,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        default_schema_arg_path, run_structured_exec, CodexCommandBuilder, CodexParserError,
        CodexParserProvider, DEFAULT_CODEX_PROGRAM, DEFAULT_MODEL, DEFAULT_SCHEMA_PATH,
    };
    use std::time::Duration;

    /// Writes a stand-in for the Codex CLI that runs `body` after reading the
    /// `--output-schema` and `-o` arguments into `%SCHEMA%` and `%OUT%`.
    #[cfg(windows)]
    fn fake_codex(directory: &std::path::Path, body: &str) -> String {
        let script = directory.join("fake-codex.cmd");
        std::fs::write(
            &script,
            format!(
                "@echo off\r\nsetlocal\r\n:next\r\nif \"%~1\"==\"\" goto run\r\n\
                 if \"%~1\"==\"--output-schema\" set \"SCHEMA=%~2\"\r\n\
                 if \"%~1\"==\"-o\" set \"OUT=%~2\"\r\nshift\r\ngoto next\r\n:run\r\n{body}\r\n"
            ),
        )
        .expect("write fake codex");
        script.to_string_lossy().into_owned()
    }

    #[test]
    fn structured_exec_reports_missing_command() {
        let error = run_structured_exec(
            "work-notes-missing-codex-binary",
            "{}",
            "prompt",
            Duration::from_secs(5),
        )
        .unwrap_err();

        assert!(matches!(error, CodexParserError::MissingCommand { .. }));
    }

    #[cfg(windows)]
    #[test]
    fn structured_exec_passes_schema_file_and_returns_output_text() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let program = fake_codex(temp_dir.path(), "copy /y \"%SCHEMA%\" \"%OUT%\" >nul");
        let schema = r#"{"type":"object"}"#;

        let output = run_structured_exec(&program, schema, "prompt", Duration::from_secs(30))
            .expect("structured exec");

        assert_eq!(output, schema);
    }

    #[cfg(windows)]
    #[test]
    fn structured_exec_reports_missing_output_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let program = fake_codex(temp_dir.path(), "exit /b 0");

        let error =
            run_structured_exec(&program, "{}", "prompt", Duration::from_secs(30)).unwrap_err();

        assert!(matches!(error, CodexParserError::MissingOutput { .. }));
    }

    #[cfg(windows)]
    #[test]
    fn structured_exec_reports_non_zero_exit() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let program = fake_codex(temp_dir.path(), "echo not logged in 1>&2\r\nexit /b 3");

        let error =
            run_structured_exec(&program, "{}", "prompt", Duration::from_secs(30)).unwrap_err();

        let CodexParserError::NonZeroExit { code, stderr } = error else {
            panic!("expected non-zero exit");
        };
        assert_eq!(code, Some(3));
        assert!(stderr.contains("not logged in"), "{stderr}");
    }

    #[cfg(windows)]
    #[test]
    fn a_cancelled_parse_stops_the_command_without_waiting_for_it() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let program = fake_codex(temp_dir.path(), "ping -n 60 127.0.0.1 >nul");
        let provider = CodexParserProvider::new()
            .program(program)
            .timeout(Duration::from_secs(120));
        let started = std::time::Instant::now();

        let error = provider
            .parse_output_cancellable("raw note", None, &|| true)
            .unwrap_err();

        assert!(matches!(error, CodexParserError::Cancelled), "{error}");
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    #[test]
    fn default_codex_program_uses_runnable_platform_shim() {
        #[cfg(windows)]
        assert_eq!(DEFAULT_CODEX_PROGRAM, "codex.cmd");

        #[cfg(not(windows))]
        assert_eq!(DEFAULT_CODEX_PROGRAM, "codex");
    }

    #[test]
    fn default_schema_is_written_to_temp_file_for_runtime_access() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        let schema_path =
            default_schema_arg_path(&temp_dir, DEFAULT_SCHEMA_PATH).expect("schema path");

        assert!(schema_path.is_absolute());
        assert!(schema_path.exists());
        assert_eq!(
            std::fs::read_to_string(schema_path).expect("schema content"),
            include_str!("../../../schemas/parse-note.schema.json")
        );
    }

    #[test]
    fn builds_codex_exec_command_with_schema_and_output_file() {
        let command = CodexCommandBuilder::new("codex")
            .schema_path("schemas/parse-note.schema.json")
            .output_path("parse-result.json")
            .build();

        assert_eq!(command.program, "codex");
        assert_eq!(
            command.args,
            vec![
                "exec",
                "--ephemeral",
                "--skip-git-repo-check",
                "--model",
                DEFAULT_MODEL,
                "-s",
                "read-only",
                "--output-schema",
                "schemas/parse-note.schema.json",
                "-o",
                "parse-result.json",
                "-",
            ]
        );
    }

    #[test]
    fn linked_workspace_paths_only_include_existing_directories() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let missing = temp_dir.path().join("missing");
        let provider = CodexParserProvider::new().linked_workspace_paths(vec![
            temp_dir.path().display().to_string(),
            missing.display().to_string(),
        ]);

        assert_eq!(
            provider.active_linked_workspace_paths(),
            vec![temp_dir.path().to_path_buf()]
        );
    }
}
