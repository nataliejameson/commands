//!-- Simple runner for executables and accessing a few other small pieces of the exeuction env.

use crate::CommandLine;
use paths::AbsolutePath;
use paths::AbsolutePathBuf;
use std::ffi::OsStr;
use std::ops::Deref;
use std::os::unix::prelude::CommandExt;
use std::process::Output;
use std::process::Stdio;
use tee::Tee;

pub trait CommandRunner {
    fn run_checked<P: AsRef<AbsolutePath>>(
        &self,
        command_line: CommandLine,
        cwd: P,
    ) -> anyhow::Result<ExecutionResult> {
        self.run_checked_with_opts(command_line, cwd, CommandOpts::default())
    }

    fn run_checked_with_opts<P: AsRef<AbsolutePath>>(
        &self,
        command_line: CommandLine,
        cwd: P,
        opts: CommandOpts,
    ) -> anyhow::Result<ExecutionResult> {
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

    fn run<P: AsRef<AbsolutePath>>(
        &self,
        command_line: CommandLine,
        cwd: P,
    ) -> anyhow::Result<ExecutionResult> {
        self.run_with_opts(command_line, cwd, CommandOpts::default())
    }

    fn run_with_opts<P: AsRef<AbsolutePath>>(
        &self,
        command_line: CommandLine,
        cwd: P,
        opts: CommandOpts,
    ) -> anyhow::Result<ExecutionResult> {
        let program_name = command_line.program()?.to_owned();
        log::info!(
            "Running `{}` in `{}`",
            command_line,
            cwd.as_ref().to_string_lossy()
        );
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
}

impl Default for CommandOpts {
    fn default() -> Self {
        Self {
            capture_stderr: true,
        }
    }
}

pub struct DefaultCommandRunner {}

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
        let mut res = std::process::Command::new(command_line.program()?)
            .args(command_line.args()?)
            .current_dir(cwd)
            .stderr(stderr)
            .output()?;
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
    use super::CommandOpts;
    use super::CommandRunner;
    use crate::CommandLine;
    use paths::AbsolutePath;
    use paths::AbsolutePathBuf;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::ops::Deref;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    use std::process::Output;

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

    pub struct TestCommandRunner {
        pub hostname: String,
        pub temp: tempfile::TempDir,
        pub issued_commands: RefCell<Vec<Invocation>>,
        pub outputs: RefCell<VecDeque<Output>>,
    }

    impl Default for TestCommandRunner {
        fn default() -> Self {
            Self {
                hostname: "local.example.com".to_owned(),
                temp: tempfile::tempdir().expect("to be able to create a tempdir"),
                issued_commands: RefCell::new(vec![]),
                outputs: RefCell::new(Default::default()),
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
            let issued_commands = RefCell::new(Vec::new());
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
                outputs: RefCell::new(outputs),
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
            self.issued_commands.borrow_mut().push(invocation);
            Ok(self.outputs.borrow_mut().pop_front().expect("An output"))
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
pub struct ExecutionResult(Output);
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
