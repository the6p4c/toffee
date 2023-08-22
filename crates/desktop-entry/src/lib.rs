//! Reader for desktop entry files (better known as `.desktop` files).
//!
//! This crate is based on freedesktop.org's `desktop-entry-spec`, version 1.5, available at
//! <https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html>.

use std::collections::HashMap;
use std::error;
use std::fmt;
use std::str::FromStr;

mod parser;

use parser::Line;

#[derive(Debug)]
enum Error {
    Parse(ParseError),
    KeyWithoutGroup,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(ParseError { inner }) => write!(f, "{}", inner),
            Error::KeyWithoutGroup => write!(f, "key without group"),
        }
    }
}

impl error::Error for Error {}

type PegParseError = peg::error::ParseError<peg::str::LineCol>;

#[derive(Debug)]
struct ParseError {
    inner: PegParseError,
}

impl From<PegParseError> for Error {
    fn from(inner: PegParseError) -> Self {
        Error::Parse(ParseError { inner })
    }
}

#[derive(Debug)]
struct DesktopEntry {
    entries: HashMap<String, HashMap<String, String>>,
}

impl FromStr for DesktopEntry {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lines = parser::file(s)?;

        let mut entries = HashMap::new();
        let mut group = None;

        for line in lines {
            match line {
                Line::Blank | Line::Comment(_) => {}
                Line::GroupHeader(g) => {
                    entries.entry(g.to_owned()).or_insert_with(HashMap::new);
                    group = Some(g);
                }
                Line::Entry(k, v) => {
                    if let Some(g) = group {
                        entries
                            .get_mut(g)
                            .expect("")
                            .insert(k.to_owned(), v.to_owned());
                    } else {
                        return Err(Error::KeyWithoutGroup);
                    }
                }
            }
        }

        Ok(DesktopEntry { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    macro_rules! from_str {
        ($s:expr) => {
            DesktopEntry::from_str($s)
        };
    }

    macro_rules! from_str_ok {
        ($s:expr) => {
            DesktopEntry::from_str($s).expect("parsing to succeed")
        };
    }

    #[test]
    fn empty_file() {
        let entry = from_str_ok!("");
        assert_eq!(entry.entries, HashMap::new());
    }

    #[test]
    fn comments_only() {
        let entry = from_str_ok!(indoc! {"
            # this file contains nothing but blank lines and comments

            # here's another comment
        "});
        assert_eq!(entry.entries, HashMap::new());
    }

    #[test]
    fn empty_group() {
        let entry = from_str_ok!(indoc! {"
            [group1]
        "});
        assert_eq!(
            entry.entries,
            HashMap::from([("group1".to_owned(), HashMap::new())])
        );
    }

    #[test]
    fn empty_groups() {
        let entry = from_str_ok!(indoc! {"
            [group1]
            [group2]
            [group3]
        "});
        assert_eq!(
            entry.entries,
            HashMap::from([
                ("group1".to_owned(), HashMap::new()),
                ("group2".to_owned(), HashMap::new()),
                ("group3".to_owned(), HashMap::new()),
            ])
        );
    }

    #[test]
    fn single_group() {
        let entry = from_str_ok!(indoc! {"
            [group1]
            k1=v1
            k2=v2
        "});
        assert_eq!(
            entry.entries,
            HashMap::from([(
                "group1".to_owned(),
                HashMap::from([
                    ("k1".to_owned(), "v1".to_owned()),
                    ("k2".to_owned(), "v2".to_owned()),
                ])
            ),])
        );
    }

    #[test]
    fn multiple_groups() {
        let entry = from_str_ok!(indoc! {"
            [group1]
            k1=v1
            k2=v2
            [group2]
            k1=v3
            k2=v4
        "});
        assert_eq!(
            entry.entries,
            HashMap::from([
                (
                    "group1".to_owned(),
                    HashMap::from([
                        ("k1".to_owned(), "v1".to_owned()),
                        ("k2".to_owned(), "v2".to_owned()),
                    ])
                ),
                (
                    "group2".to_owned(),
                    HashMap::from([
                        ("k1".to_owned(), "v3".to_owned()),
                        ("k2".to_owned(), "v4".to_owned()),
                    ])
                ),
            ])
        );
    }

    #[test]
    fn error_parse() {
        let entry = from_str!(indoc! {"
            [group1
        "});
        assert!(matches!(entry, Err(Error::Parse(_))));
    }

    #[test]
    fn error_key_without_group() {
        let entry = from_str!(indoc! {"
            k=v
        "});
        assert!(matches!(entry, Err(Error::KeyWithoutGroup)));
    }
}
