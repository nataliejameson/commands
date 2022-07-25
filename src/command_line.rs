use std::ops::Deref;

/// Simple wrapper for a vec of strings that lets us push / extend with str literals too.
#[derive(Debug, PartialEq, Eq)]
pub struct CommandLine(pub Vec<String>);

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
    type Target = Vec<String>;

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
}

#[cfg(test)]
mod test {
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
}
