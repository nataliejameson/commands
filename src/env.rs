//!-- Simple runner for executables and accessing a few other small pieces of the exeuction env.

use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::io::Write;
use std::ops::Deref;
use std::os::unix::prelude::CommandExt;
use std::process::Output;
use std::process::Stdio;

use maplit::hashset;
use paths::AbsolutePath;
use paths::AbsolutePathBuf;
use tee::Tee;

use crate::CommandLine;

pub trait CommandRunner: Debug + Send + Sync {
    fn run_checked<P: AsRef<AbsolutePath>, C: Into<CommandLine>>(
        &self,
        command_line: C,
        cwd: P,
    ) -> anyhow::Result<ExecutionResult> {
        self.run_checked_with_opts(command_line, cwd, CommandOpts::default())
    }

    fn run_checked_with_opts<P: AsRef<AbsolutePath>, C: Into<CommandLine>>(
        &self,
        command_line: C,
        cwd: P,
        opts: CommandOpts,
    ) -> anyhow::Result<ExecutionResult> {
        let command_line = command_line.into();
        let program_name = command_line.program()?.to_owned();
        let ret = self.run_with_opts(command_line, cwd, opts)?;
        match ret.status.success() {
            true => Ok(ret),
            false => Err(anyhow::anyhow!(
                "Command `{}` failed with status `{}`\nStdout:\n{}\nStderr:\n{}",
                program_name,
                ret.status,
                String::from_utf8_lossy(&ret.stdout),
                String::from_utf8_lossy(&ret.stderr)
            )),
        }
    }

    fn run<P: AsRef<AbsolutePath>, C: Into<CommandLine>>(
        &self,
        command_line: C,
        cwd: P,
    ) -> anyhow::Result<ExecutionResult> {
        self.run_with_opts(command_line, cwd, CommandOpts::default())
    }

    fn run_with_opts<P: AsRef<AbsolutePath>, C: Into<CommandLine>>(
        &self,
        command_line: C,
        cwd: P,
        opts: CommandOpts,
    ) -> anyhow::Result<ExecutionResult> {
        let command_line = command_line.into();
        let program_name = command_line.program()?.to_owned();
        log::info!("Running `{}` in `{}`", command_line, cwd.as_ref());
        let output = self.run_inner(command_line, cwd.as_ref(), opts)?;
        log::debug!(
            "Completed `{}` with exit status `{}`",
            program_name,
            output.status
        );
        Ok(ExecutionResult(output))
    }
    fn run_inner(
        &self,
        command_line: CommandLine,
        cwd: &AbsolutePath,
        opts: CommandOpts,
    ) -> anyhow::Result<Output>;

    /// `exec()` the command, handing the process over to this command.
    fn exec(&self, command_line: CommandLine) -> anyhow::Result<()>
    where
        Self: Sized;

    fn root_systemd_path(&self) -> AbsolutePathBuf {
        AbsolutePathBuf::try_new("/etc/systemd/user").expect("already validated")
    }

    fn user_systemd_path(&self) -> anyhow::Result<AbsolutePathBuf> {
        match dirs::config_dir() {
            Some(p) => Ok(AbsolutePathBuf::try_from(p.join("systemd/user"))?),
            None => Err(MissingHomeError.into()),
        }
    }

    fn hostname(&self) -> anyhow::Result<String> {
        Ok(hostname::get()?.to_string_lossy().to_string())
    }
}

#[derive(Clone)]
pub struct CommandOpts {
    pub capture_stderr: bool,
    pub stdin: Option<Vec<u8>>,
}

impl Default for CommandOpts {
    fn default() -> Self {
        Self {
            capture_stderr: true,
            stdin: None,
        }
    }
}

#[derive(Debug)]
pub struct DefaultCommandRunner {
    ignored_env_vars: Option<HashSet<&'static str>>,
    allowed_env_vars: Option<HashSet<&'static str>>,
}

impl Default for DefaultCommandRunner {
    fn default() -> Self {
        Self {
            ignored_env_vars: Some(hashset!["GIT_DIR"]),
            allowed_env_vars: None,
        }
    }
}

impl DefaultCommandRunner {
    pub fn ignoring_env(ignored: &[&'static str]) -> Self {
        let ignored = ignored.iter().copied().collect();
        Self {
            ignored_env_vars: Some(ignored),
            allowed_env_vars: None,
        }
    }

    pub fn allowing_env(ignored: &[&'static str]) -> Self {
        let allowed = ignored.iter().copied().collect();
        Self {
            ignored_env_vars: None,
            allowed_env_vars: Some(allowed),
        }
    }

    fn env_vars(&self) -> HashMap<String, String> {
        if let Some(ignored) = self.ignored_env_vars.as_ref() {
            std::env::vars()
                .filter(|(name, _)| !ignored.contains(name.as_str()))
                .collect()
        } else if let Some(allowed) = self.allowed_env_vars.as_ref() {
            std::env::vars()
                .filter(|(name, _)| allowed.contains(name.as_str()))
                .collect()
        } else {
            std::env::vars().collect()
        }
    }
}

impl CommandRunner for DefaultCommandRunner {
    fn run_inner(
        &self,
        command_line: CommandLine,
        cwd: &AbsolutePath,
        opts: CommandOpts,
    ) -> anyhow::Result<Output> {
        let (stderr, stderr_tee) = if opts.capture_stderr {
            let tee = Tee::new(std::io::stderr())?;
            (tee.clone().into(), Some(tee))
        } else {
            (Stdio::inherit(), None)
        };
        let stdin = if opts.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::inherit()
        };
        let mut child = std::process::Command::new(command_line.program()?)
            .args(command_line.args()?)
            .current_dir(cwd)
            .env_clear()
            .envs(self.env_vars())
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()?;
        if let (Some(stdin), Some(stdin_bytes)) = (child.stdin.as_mut(), opts.stdin) {
            stdin.write_all(&stdin_bytes)?;
        }
        let mut res = child.wait_with_output()?;

        if let Some(tee) = stderr_tee {
            res.stderr = tee.get_output()?;
        }
        Ok(res)
    }

    fn exec(&self, command_line: CommandLine) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        log::info!("Exec'ing `{}`", command_line);

        Err(std::process::Command::new(command_line.program()?)
            .args(command_line.args()?)
            .exec()
            .into())
    }
}

pub mod test {
    use std::collections::VecDeque;
    use std::ops::Deref;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    use std::process::Output;
    use std::sync::RwLock;

    use paths::AbsolutePath;
    use paths::AbsolutePathBuf;

    use super::CommandOpts;
    use super::CommandRunner;
    use crate::CommandLine;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Invocation {
        command_line: CommandLine,
        cwd: AbsolutePathBuf,
    }

    impl Deref for Invocation {
        type Target = CommandLine;

        fn deref(&self) -> &Self::Target {
            &self.command_line
        }
    }

    impl Invocation {
        pub fn new<P: Into<AbsolutePathBuf>>(command_line: CommandLine, cwd: P) -> Self {
            Self {
                command_line,
                cwd: cwd.into(),
            }
        }
    }

    #[derive(Debug)]
    pub struct TestCommandRunner {
        pub hostname: String,
        pub temp: tempfile::TempDir,
        pub issued_commands: RwLock<Vec<Invocation>>,
        pub outputs: RwLock<VecDeque<Output>>,
    }

    impl Default for TestCommandRunner {
        fn default() -> Self {
            Self {
                hostname: "local.example.com".to_owned(),
                temp: tempfile::tempdir().expect("to be able to create a tempdir"),
                issued_commands: RwLock::new(vec![]),
                outputs: RwLock::new(Default::default()),
            }
        }
    }

    impl TestCommandRunner {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_results<I: IntoIterator<Item = (i32, S)>, S: ToString>(
            i: I,
        ) -> anyhow::Result<Self> {
            let issued_commands = RwLock::new(Vec::new());
            let outputs: VecDeque<_> = i
                .into_iter()
                .map(|(code, stdout)| {
                    let _s = String::new();
                    Output {
                        status: ExitStatus::from_raw(code),
                        stdout: stdout.to_string().into_bytes(),
                        stderr: vec![],
                    }
                })
                .collect();
            Ok(Self {
                issued_commands,
                outputs: RwLock::new(outputs),
                ..Default::default()
            })
        }
    }

    impl CommandRunner for TestCommandRunner {
        fn run_inner(
            &self,
            command_line: CommandLine,
            cwd: &AbsolutePath,
            _opts: CommandOpts,
        ) -> anyhow::Result<Output> {
            let invocation = Invocation::new(command_line, cwd);
            self.issued_commands.write().unwrap().push(invocation);
            Ok(self
                .outputs
                .write()
                .unwrap()
                .pop_front()
                .expect("An output"))
        }

        fn exec(&self, _command_line: CommandLine) -> anyhow::Result<()>
        where
            Self: Sized,
        {
            Ok(())
        }

        fn hostname(&self) -> anyhow::Result<String> {
            Ok(self.hostname.clone())
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Could not get $HOME!")]
pub struct MissingHomeError;

/// The outcome of a command + helper methods
pub struct ExecutionResult(pub Output);
impl ExecutionResult {
    pub fn stdout(&self) -> anyhow::Result<String> {
        Ok(String::from_utf8(self.0.stdout.clone())?)
    }
}

impl Deref for ExecutionResult {
    type Target = Output;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Various pieces of the environment needed to execute commands, find paths, etc.
///
/// A trait so that testing is easier.
pub trait Env {
    fn execute<T: AsRef<OsStr>>(&self, cmd: &[T]) -> anyhow::Result<ExecutionResult>
    where
        Self: Sized;

    fn exec<T: AsRef<OsStr>>(&self, cmd: &[T]) -> anyhow::Result<()>
    where
        Self: Sized;

    fn root_systemd_path(&self) -> AbsolutePathBuf;
    fn user_systemd_path(&self) -> anyhow::Result<AbsolutePathBuf>;
    fn hostname(&self) -> anyhow::Result<String>;
}

#[cfg(test)]
mod default_runner_tests {
    use paths::AbsolutePath;
    use paths::AbsolutePathBuf;

    use crate::CommandOpts;
    use crate::CommandRunner;
    use crate::DefaultCommandRunner;

    #[test]
    fn sets_cwd_correctly() -> anyhow::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let runner = DefaultCommandRunner::default();
        std::fs::write(temp.path().join("test_file"), "contents")?;

        let out = runner.run_checked(["ls", "-1", "."], &AbsolutePath::try_new(temp.path())?)?;
        assert_eq!("test_file", out.stdout()?.trim());

        let out = runner.run(["ls", "-1", "."], &AbsolutePath::try_new(temp.path())?)?;
        assert_eq!("test_file", out.stdout()?.trim());

        let out = runner.run_with_opts(
            ["ls", "-1", "."],
            &AbsolutePath::try_new(temp.path())?,
            CommandOpts::default(),
        )?;
        assert_eq!("test_file", out.stdout()?.trim());
        Ok(())
    }

    #[test]
    fn excludes_env_vars() -> anyhow::Result<()> {
        let runner = DefaultCommandRunner::default();
        let stdout = runner
            .run_checked(["env"], AbsolutePathBuf::current_dir())?
            .stdout()?;

        assert!(stdout.contains("CARGO_MANIFEST_DIR"));

        let runner = DefaultCommandRunner::ignoring_env(&["CARGO_MANIFEST_DIR"]);
        let stdout = runner
            .run_checked(["env"], AbsolutePathBuf::current_dir())?
            .stdout()?;

        assert!(!stdout.contains("CARGO_MANIFEST_DIR"));

        let runner = DefaultCommandRunner::allowing_env(&["CWD", "CARGO_MANIFEST_DIR"]);
        let stdout = runner
            .run_checked(["env"], AbsolutePathBuf::current_dir())?
            .stdout()?;

        assert!(stdout.contains("CARGO_MANIFEST_DIR"));

        let runner = DefaultCommandRunner::allowing_env(&["CWD"]);
        let stdout = runner
            .run_checked(["env"], AbsolutePathBuf::current_dir())?
            .stdout()?;

        assert!(!stdout.contains("CARGO_MANIFEST_DIR"));

        Ok(())
    }

    #[test]
    fn uses_stdin() -> anyhow::Result<()> {
        let runner = DefaultCommandRunner::default();
        let stdout = runner
            .run_checked_with_opts(
                ["cat"],
                &AbsolutePathBuf::current_dir(),
                CommandOpts {
                    stdin: Some("TESTING".as_bytes().to_vec()),
                    ..Default::default()
                },
            )?
            .stdout()?;

        assert_eq!("TESTING", stdout);
        Ok(())
    }
}
