pub mod command_line;
pub mod env;

pub use crate::command_line::CommandLine;
pub use crate::env::DefaultEnv;

#[deny(clippy::all)]
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
