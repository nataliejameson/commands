# commands

Simple classes and functions to wrap up std::process to be a little more ergonomic.

"Ergonomic" meaning "what Natalie likes".

## Adding to a project

```toml
# Cargo.toml
[dependencies]
commands = { git = "https://github.com/nataliejameson/commands", tag = "0.1.0" }
```

## Example Usage

```rust
use commands::{CommandRunner, DefaultCommandRunner, ExecutionResult};
use commands::paths::AbsolutePathBuf;

fn run_echo(runner: &impl CommandRunner, fail: bool) -> anyhow::Result<ExecutionResult> {
    let cwd = AbsolutePathBuf::current_dir();
    if fail {
        runner.run(
            [
                "/bin/sh",
                "-c",
                "echo failure; echo failure stderr >&2; exit 1",
            ],
            &cwd,
        )
    } else {
        runner.run_checked(
            ["/bin/sh", "-c", "echo success; echo success stderr >&2"],
            &cwd,
        )
    }
}

fn main() -> anyhow::Result<()> {
    let runner = DefaultCommandRunner::default();
    assert_eq!("success\n", run_echo(&runner, false)?.stdout()?);

    let res = run_echo(&runner, true)?;
    assert_eq!(1, res.status.code().unwrap());
    assert_eq!("failure\n", res.stdout()?);
    Ok(())
}
```

It also logs every command it runs to stderr at `info` level to aid in understanding what a program is doing. If this is undesired, the following information is printed by default at the following levels, so you can configure your logger accordingly for the `commands` module.

| Log Level | Details                                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------------------|
| debug     | When a program completes, even on success. Note that if the runner returns an error, this will not be logged. |
| info      | When a command is started, or when `exec` is run.                                                             |

The main structs for this crate are:

## `commands::CommandLine`

This is a wrapper around a list of string arguments that is passed into functions in the `commands::CommandRunner` trait.

The main constraints are:
  - There must be at least one argument (the program name) present to be valid.
  - This is generally constructed with the `From` impl. e.g. `CommandLine::from(["foo", "--bar"])`

## `commands::CommandRunner`

This is the main interface to actually run commands.

There are two implementations included.

### `commands::DefaultCommandRunner`

This is the default way to run commands. It implements `commands::CommandRunner`, and uses `std::process::Process` to execute.

### `commands::test::TestCommandRunner`

A command runner to use in testing that just records the commands that were requested to be executed, and returns a specified status code and stdout.



