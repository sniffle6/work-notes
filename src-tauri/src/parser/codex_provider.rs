use super::{
    prompt::build_parse_prompt,
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

pub const DEFAULT_CODEX_PROGRAM: &str = "codex";
pub const DEFAULT_SCHEMA_PATH: &str = "schemas/parse-note.schema.json";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

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
}

impl Default for CodexParserProvider {
    fn default() -> Self {
        Self {
            program: DEFAULT_CODEX_PROGRAM.to_string(),
            schema_path: DEFAULT_SCHEMA_PATH.to_string(),
            timeout: DEFAULT_TIMEOUT,
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
        let temp_dir = tempfile::Builder::new()
            .prefix("work-notes-codex-parser-")
            .tempdir()
            .map_err(|source| CodexParserError::Io {
                context: "create temporary parser output directory",
                source,
            })?;
        let output_path = temp_output_path(&temp_dir);
        let output_arg = output_path.to_string_lossy().into_owned();
        let command = CodexCommandBuilder::new(self.program.clone())
            .schema_path(self.schema_path.clone())
            .output_path(output_arg)
            .build();

        run_codex_command(&command, &build_parse_prompt(raw_note), self.timeout)?;
        read_parser_result(output_path)
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

fn run_codex_command(
    command: &CodexCommandSpec,
    stdin_body: &str,
    timeout: Duration,
) -> Result<(), CodexParserError> {
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| {
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
    use super::CodexCommandBuilder;

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
                "--output-schema",
                "schemas/parse-note.schema.json",
                "-o",
                "parse-result.json",
                "-",
            ]
        );
    }
}
