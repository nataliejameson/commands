use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub enum CommandLineError {
    #[error("At least one argument must be provided")]
    MissingProgram,
}

/// Simple wrapper for a vec of strings that lets us push / extend with str literals too.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandLine(pub Vec<String>);

impl Display for CommandLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for a in &self.0 {
            f.write_str(a)?;
        }
        Ok(())
    }
}

impl<T: IntoIterator<Item = impl Into<String>>> From<T> for CommandLine {
    fn from(items: T) -> Self {
        Self(items.into_iter().map(|i| i.into()).collect())
    }
}

impl From<CommandLine> for Vec<String> {
    fn from(c: CommandLine) -> Self {
        c.0
    }
}

impl Deref for CommandLine {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandLine {
    pub fn push<T: Into<String>>(&mut self, v: T) {
        self.0.push(v.into())
    }

    pub fn extend<T: IntoIterator<Item = impl Into<String>>>(&mut self, v: T) {
        self.0.extend(v.into_iter().map(|i| i.into()))
    }

    pub fn clone_with<T: IntoIterator<Item = impl Into<String>>>(&self, v: T) -> Self {
        let mut new = self.clone();
        new.extend(v);
        new
    }

    pub fn program(&self) -> Result<&str, CommandLineError> {
        match self.0.first() {
            Some(s) => Ok(s),
            None => Err(CommandLineError::MissingProgram),
        }
    }

    pub fn args(&self) -> Result<&[String], CommandLineError> {
        if self.0.is_empty() {
            Err(CommandLineError::MissingProgram)
        } else {
            Ok(&self.0[1..])
        }
    }
}

#[cfg(test)]
mod test {
    use crate::command_line::CommandLineError;
    use crate::CommandLine;
    use gazebo::prelude::VecExt;

    #[test]
    fn push_works() {
        let mut cli = CommandLine::from(vec!["foo"]);
        cli.push("bar");
        cli.push("baz".to_owned());
        cli.extend(["foo2", "bar2"]);
        cli.extend(["foo3".to_owned(), "bar3".to_owned()]);

        let expected =
            vec!["foo", "bar", "baz", "foo2", "bar2", "foo3", "bar3"].into_map(String::from);
        assert_eq!(*cli, expected)
    }

    #[test]
    fn equality() {
        let cli = CommandLine::from(["foo", "bar"]);
        assert_eq!(["foo", "bar"], *cli);
    }

    #[test]
    fn clone_with() {
        let mut cli = CommandLine::from(["foo", "bar"]);
        let cloned = cli.clone_with(["baz"]);
        cli.push("quz");

        assert_eq!(["foo", "bar", "quz"], *cli);
        assert_eq!(["foo", "bar", "baz"], *cloned);
    }

    #[test]
    fn program_and_args_work() {
        let cli = CommandLine::from(["foo", "bar", "baz"]);
        assert_eq!(Ok("foo"), cli.program());
        assert_eq!(["bar".to_owned(), "baz".to_owned()], *cli.args().unwrap());

        let bad_cli = CommandLine::from(Vec::<String>::new());
        assert_eq!(Err(CommandLineError::MissingProgram), bad_cli.program());
        assert_eq!(Err(CommandLineError::MissingProgram), bad_cli.args());
    }
}
