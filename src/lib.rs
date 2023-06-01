mod command_line;
mod runner;

// re-exported because it's needed for things in [`CommandRunner`]
pub use paths;

pub use crate::command_line::CommandLine;
pub use crate::runner::CommandOpts;
pub use crate::runner::CommandRunner;
pub use crate::runner::DefaultCommandRunner;
pub use crate::runner::ExecutionResult;
pub use crate::runner::MissingHomeError;
pub use crate::runner::StdioCapture;

pub mod test {
    pub use crate::runner::test::Invocation;
    pub use crate::runner::test::TestCommandRunner;
}
