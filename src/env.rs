//!-- Simple runner for executables and accessing a few other small pieces of the exeuction env.

use std::ffi::OsStr;
use std::os::unix::prelude::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use tracing::info;

#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("Received empty command")]
    EmptyCommand,
    #[error("Command `{}` failed with exit code {:?}", .0, .1)]
    CommandFailed(String, ExitStatus),
}

#[derive(thiserror::Error, Debug)]
#[error("Could not get $HOME!")]
pub struct MissingHomeError;

/// The outcome of a command
pub struct ExecutionResult {
    pub status: std::process::ExitStatus,
    pub stdout: String,
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

    // fn config_dir(&self) -> anyhow::Result<PathBuf>;
    // fn home_dir(&self) -> anyhow::Result<PathBuf>;
    fn root_systemd_path(&self) -> PathBuf;
    fn user_systemd_path(&self) -> anyhow::Result<PathBuf>;
    fn hostname(&self) -> anyhow::Result<String>;
}

/// The default environment that actually runs commands, has real paths, etc.
pub struct DefaultEnv;

impl Env for DefaultEnv {
    fn execute<T: AsRef<OsStr>>(&self, cmd: &[T]) -> anyhow::Result<ExecutionResult> {
        let cmd_string = cmd
            .iter()
            .map(|a| a.as_ref().to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        info!("Running `{}`", cmd_string);

        if cmd.is_empty() {
            return Err(ExecutionError::EmptyCommand.into());
        }
        let out = Command::new(cmd.get(0).unwrap())
            .args(cmd.iter().skip(1))
            .stderr(Stdio::inherit())
            .output()?;
        if !out.status.success() {
            Err(ExecutionError::CommandFailed(cmd_string, out.status).into())
        } else {
            info!("Command `{}` finished successfully!", cmd_string);
            Ok(ExecutionResult {
                status: out.status,
                stdout: String::from_utf8(out.stdout)?,
            })
        }
    }

    fn exec<T: AsRef<OsStr>>(&self, cmd: &[T]) -> anyhow::Result<()> {
        let cmd_string = cmd
            .iter()
            .map(|a| a.as_ref().to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        info!("Running `{}`", cmd_string);

        if cmd.is_empty() {
            return Err(ExecutionError::EmptyCommand.into());
        }
        Err(Command::new(cmd.get(0).unwrap())
            .args(cmd.iter().skip(1))
            .exec()
            .into())
    }

    fn root_systemd_path(&self) -> PathBuf {
        PathBuf::from("/etc/systemd/user")
    }

    fn user_systemd_path(&self) -> anyhow::Result<PathBuf> {
        match dirs::config_dir() {
            Some(p) => Ok(p.join("systemd/user")),
            None => Err(MissingHomeError.into()),
        }
    }

    fn hostname(&self) -> anyhow::Result<String> {
        Ok(hostname::get()?.to_string_lossy().to_string())
    }
}

pub mod testing {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::ffi::OsStr;
    use std::os::unix::prelude::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::ExitStatus;

    use crate::env::Env;
    use crate::env::ExecutionError;
    use crate::env::ExecutionResult;

    pub struct TestEnv {
        pub temp: tempfile::TempDir,
        pub issued_commands: RefCell<Vec<Vec<String>>>,
        pub results: Option<RefCell<VecDeque<(i32, String)>>>,
    }

    impl TestEnv {
        pub fn new() -> anyhow::Result<Self> {
            Ok(TestEnv {
                temp: tempfile::tempdir()?,
                issued_commands: RefCell::new(Vec::new()),
                results: None,
            })
        }

        pub fn with_results<T: ToString>(results: Vec<(i32, T)>) -> anyhow::Result<Self> {
            Ok(TestEnv {
                temp: tempfile::tempdir()?,
                issued_commands: RefCell::new(Vec::new()),
                results: Some(RefCell::new(
                    results
                        .into_iter()
                        .map(|(code, s)| (code, s.to_string()))
                        .collect(),
                )),
            })
        }
    }

    impl Env for TestEnv {
        fn execute<T: AsRef<OsStr>>(&self, cmd: &[T]) -> anyhow::Result<ExecutionResult>
        where
            Self: Sized,
        {
            let cmd_string = cmd
                .iter()
                .map(|a| a.as_ref().to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            self.issued_commands.borrow_mut().push(
                cmd.iter()
                    .map(|os| os.as_ref().to_string_lossy().to_string())
                    .collect(),
            );
            match self.results.as_ref() {
                None => Ok(ExecutionResult {
                    status: ExitStatus::from_raw(0),
                    stdout: String::new(),
                }),
                Some(results) => match results.borrow_mut().pop_front() {
                    None => Err(anyhow::anyhow!("No results remaining")),
                    Some((0, stdout)) => Ok(ExecutionResult {
                        status: ExitStatus::from_raw(0),
                        stdout,
                    }),
                    Some((exit_code, _)) => Err(ExecutionError::CommandFailed(
                        cmd_string,
                        ExitStatus::from_raw(exit_code),
                    )
                    .into()),
                },
            }
        }
        fn exec<T: AsRef<OsStr>>(&self, _cmd: &[T]) -> anyhow::Result<()>
        where
            Self: Sized,
        {
            Ok(())
        }

        fn root_systemd_path(&self) -> PathBuf {
            self.temp.path().join("systemd/root")
        }

        fn user_systemd_path(&self) -> anyhow::Result<PathBuf> {
            Ok(self.temp.path().join("systemd/user"))
        }

        fn hostname(&self) -> anyhow::Result<String> {
            Ok("local.example.com".to_owned())
        }
    }
}
