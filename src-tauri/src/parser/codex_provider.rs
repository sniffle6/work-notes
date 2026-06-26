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
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_SCHEMA_JSON: &str = include_str!("../../../schemas/parse-note.schema.json");

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

    let status = wait_with_timeout(&mut child, timeout)?;
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
) -> Result<ExitStatus, CodexParserError> {
    let started_at = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started_at.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CodexParserError::Timeout { timeout });
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
        default_schema_arg_path, CodexCommandBuilder, CodexParserProvider, DEFAULT_CODEX_PROGRAM,
        DEFAULT_SCHEMA_PATH,
    };

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
