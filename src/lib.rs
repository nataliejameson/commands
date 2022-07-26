pub mod command_line;
pub mod env;

pub use crate::command_line::CommandLine;
pub use crate::env::CommandOpts;
pub use crate::env::CommandRunner;
pub use crate::env::DefaultCommandRunner;
pub use crate::env::ExecutionResult;

#[deny(clippy::all)]
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
