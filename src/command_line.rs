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
pub struct CommandLine(Vec<String>);

impl Display for CommandLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for a in &self.0 {
            if first {
                first = false;
            } else {
                f.write_str(" ")?;
            }
            f.write_str(a)?;
        }
        Ok(())
    }
}

/// [`From`] for anything that is an iterator of things that can be `str`s
impl<T: IntoIterator<Item = impl AsRef<str>>> From<T> for CommandLine {
    fn from(items: T) -> Self {
        Self(items.into_iter().map(|i| i.as_ref().to_owned()).collect())
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
    /// Add an argument onto this command line.
    pub fn push<T: Into<String>>(&mut self, v: T) {
        self.0.push(v.into())
    }

    /// Add all elements of another command line onto this one.
    pub fn extend<T: Into<CommandLine>>(&mut self, v: T) {
        self.0.extend(v.into().0)
    }

    /// Clones this command line and adds `v` to that clone.
    pub fn clone_with<T: Into<CommandLine>>(&self, v: T) -> Self {
        let mut new = self.clone();
        new.extend(v.into().0);
        new
    }

    /// Get the program that is to be executed.
    ///
    /// This is the first element of the list of arguments. If
    /// there are zero arguments in this command line, this method
    /// fails.
    pub fn program(&self) -> Result<&str, CommandLineError> {
        match self.0.first() {
            Some(s) => Ok(s),
            None => Err(CommandLineError::MissingProgram),
        }
    }

    /// Get the args for this command line.
    ///
    /// This is all arguments after [`Self::program`]. This will fail
    /// if there are zero arguments in the command line.
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

    #[test]
    fn from_string_collection() {
        assert_eq!(["foo", "bar"], *CommandLine::from(["foo", "bar"]));
        assert_eq!(["foo", "bar"], *CommandLine::from(&["foo", "bar"]));
        assert_eq!(["foo", "bar"], *CommandLine::from(vec!["foo", "bar"]));
        assert_eq!(["foo", "bar"], *CommandLine::from(&vec!["foo", "bar"]));
        assert_eq!(
            ["foo", "bar"],
            *CommandLine::from(["foo".to_owned(), "bar".to_owned()])
        );
        assert_eq!(
            ["foo", "bar"],
            *CommandLine::from(&["foo".to_owned(), "bar".to_owned()])
        );
        assert_eq!(
            ["foo", "bar"],
            *CommandLine::from(vec!["foo".to_owned(), "bar".to_owned()])
        );
        assert_eq!(
            ["foo", "bar"],
            *CommandLine::from(&vec!["foo".to_owned(), "bar".to_owned()])
        );
    }

    #[test]
    fn push_works() {
        let mut cli = CommandLine::from(vec!["foo"]);
        cli.push("bar");
        cli.push("baz".to_owned());
        cli.extend(["foo1", "bar1"]);
        cli.extend(["foo2", "bar2"]);
        cli.extend(vec!["foo3", "bar3"]);
        cli.extend(&vec!["foo4", "bar4"]);
        cli.extend(["foo5", "bar5"]);
        cli.extend(["foo6", "bar6"]);
        cli.extend(vec!["foo7", "bar7"]);
        cli.extend(&vec!["foo8", "bar8"]);

        let mut expected = vec!["foo".to_owned(), "bar".to_owned(), "baz".to_owned()];
        expected.extend((1..9).flat_map(|i| [format!("foo{}", i), format!("bar{}", i)]));

        assert_eq!(expected, *cli);
    }

    #[test]
    fn equality() {
        let cli = CommandLine::from(["foo", "bar"]);
        assert_eq!(["foo", "bar"], *cli);
    }

    #[test]
    fn clone_with() {
        let mut cli = CommandLine::from(["foo", "bar"]);
        let cloned1 = cli.clone_with(["baz"]);
        let cloned2 = cli.clone_with(["baz"]);
        let cloned3 = cli.clone_with(vec!["baz"]);
        let cloned4 = cli.clone_with(vec!["baz"]);
        cli.push("quz");

        assert_eq!(["foo", "bar", "quz"], *cli);
        assert_eq!(["foo", "bar", "baz"], *cloned1);
        assert_eq!(["foo", "bar", "baz"], *cloned2);
        assert_eq!(["foo", "bar", "baz"], *cloned3);
        assert_eq!(["foo", "bar", "baz"], *cloned4);
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
