//! Reader for desktop entry files (better known as `.desktop` files). Provides a Rust
//! representation of the group-key-value structure of a desktop entry file, and parser of the
//! file's basic line-oriented structure.
//!
//! This crate is based on freedesktop.org's `desktop-entry-spec`, version 1.5, available at
//! <https://specifications.freedesktop.org/desktop-entry-spec/1.5/>.
//!
//! Note that consistency is not enforced by the file parser itself. Some guarantees are made about
//! the data once within a [`DesktopFile`].
//!
//! # Interpretations
//! The specification is surprisingly unclear and leaves many potential interpretations of
//! important aspects.
//! - parser: "Blank line(s)" is interpreted as "empty line(s)". That is, **a blank line is a line
//!   which contains no characters other than its terminating newline.**
//! - parser: As pertaining to entries, the specification states that "Space before and after the
//!   equals sign should be ignored; the `=` sign is the actual delimiter." As far as I can tell,
//!   this is a contradiction. As such, **spaces before and after the equals sign are ignored, and
//!   not included in the key or value string.**
//!   - This is equivalent to a `trim_end_matches(' ')` on the key and `trim_start_matches(' ')`
//!     on the key and value, respectively.
//!   - A `find / -name "*.desktop" -exec cat {} \; 2>/dev/null | grep -E "(\\s+=|=\\s+)"` on my
//!     system reveals that there are a non-zero amount of cases where developers/translators
//!     have left a space character after the equals sign, for example
//!     `Name[hi]= XMPP हँसमुख प्रसंग` from [kemoticons](https://invent.kde.org/frameworks/kemoticons/-/blob/c010010955e6ac0febe73fca1e28665e1840b4c0/src/providers/xmpp/emoticonstheme_xmpp.desktop#L33).
//! - parser: Is the empty string a valid key? This parser requires that **all keys must be
//!   non-empty.**
//! - parser: Keys can contain a locale suffix, enclosed in square brackets. This isn't mentioned at
//!   all when the specification defines keys, and further the expected format of the locale string
//!   is underspecified. It is simply specified that a key may be suffixed with a locale string
//!   (e.g. `key[locale]=value`). We implement this such that **the locale string may only contain
//!   the characters `A-Z`, `a-z`, `0-9`, `-`, `_`, and `@`** (that is, all characters allowed in
//!   keys plus `_` and `@` to support `LC_MESSAGES` style `lang_COUNTRY@MODIFIER` locale strings).
//! - repr: The specification states that "Multiple groups may not have the same name." I don't see
//!   how this makes sense - if two groups have the same name, they are the same group. Presumably,
//!   this is intended to communicate that you cannot add keys to a previously created but not
//!   currently "active" group. For example, [TOML disallows definining a table more than
//!   once](https://toml.io/en/v1.0.0#table). As such, **we raise a "duplicate group" error in a
//!   case such as the following:**
//!   ```text
//!   [group1]
//!   k1=v1
//!   # group1 is now "closed" and "inactive"
//!   [group2]
//!   k2=v2
//!   # oops! group1 "re-opened" -> duplicate group
//!   [group1]
//!   k3=v3
//!   ```
//!
//! # Deviations
//! We deviate from the specification in a few ways, which overall make our parser more lenient.
//! - parser: **Where the specification permits only ASCII strings, we permit UTF-8 strings. We allow
//!   control characters even when explicitly disallowed.**
//!   - For example, the specification states that "Group names may contain all ASCII characters
//!     except for `[` and `]` and control characters." We permit all characters except for `[`
//!     and `]`.

pub mod define_group;
pub mod desktop_entry;
pub mod parser;
mod parser_util;

use std::collections::HashMap;
use thiserror::Error;

use parser::{file_parser, value_parser, Line};

type PegParseError = peg::error::ParseError<peg::str::LineCol>;

#[derive(Error, Debug, PartialEq)]
#[error(transparent)]
pub struct ParseError(#[from] PegParseError);

#[derive(Error, Debug, PartialEq)]
pub enum DesktopFileError<'input> {
    #[error("parsing should succeed")]
    Parse(#[from] ParseError),
    #[error("entries must be preceeded by a group header (found key {0} outside group)")]
    EntryOutsideOfGroup(&'input str),
    #[error("a group must appear in one group header only (found duplicate group [{0}])")]
    DuplicateGroup(&'input str),
    #[error("keys within a group must be unique (found duplicate key {0})")]
    DuplicateKey(&'input str),
}

/// Required to turn a [PegParseError] and into a [DesktopFileError] with `?`.
impl From<PegParseError> for DesktopFileError<'_> {
    fn from(value: PegParseError) -> Self {
        Self::from(ParseError::from(value))
    }
}

#[derive(Debug)]
pub struct DesktopFile<'input> {
    groups: HashMap<&'input str, Group<'input>>,
}

impl<'input> DesktopFile<'input> {
    pub fn parse(s: &'input str) -> Result<Self, DesktopFileError> {
        let lines = file_parser::file(s)?;

        let mut groups = HashMap::new();
        let mut current_group_name = None;
        for line in lines {
            match line {
                Line::Blank | Line::Comment(_) => {}
                Line::GroupHeader(group_name) => {
                    if groups.insert(group_name, Group::new()).is_some() {
                        return Err(DesktopFileError::DuplicateGroup(group_name));
                    }
                    current_group_name = Some(group_name);
                }
                Line::Entry(key, value) => {
                    let group_name =
                        current_group_name.ok_or(DesktopFileError::EntryOutsideOfGroup(key))?;

                    let group = groups
                        .get_mut(group_name)
                        .expect("current group should exist");
                    if group.entries.insert(key, value).is_some() {
                        return Err(DesktopFileError::DuplicateKey(key));
                    }
                }
            }
        }

        Ok(Self { groups })
    }

    pub fn group(&self, group_name: &str) -> Option<&Group> {
        self.groups.get(group_name)
    }

    pub fn groups(&self) -> impl Iterator<Item = (&str, &Group)> {
        self.groups
            .iter()
            .map(|(group_name, group)| (*group_name, group))
    }
}

#[derive(Debug)]
pub struct Group<'input> {
    entries: HashMap<&'input str, &'input str>,
}

impl Group<'_> {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get_raw<'a>(&self, key: impl Into<Key<'a>>) -> Option<&str> {
        let entry = match key.into() {
            Key::String(key) => self.entries.get(key),
            Key::Localized(locale_key) => locale_key
                .matches()
                .into_iter()
                .flat_map(|key| self.entries.get(key.as_str()))
                .next(),
        };

        entry.copied()
    }

    pub fn get<'a, V: FromRaw>(&self, key: impl Into<Key<'a>>) -> Option<Result<V, ParseError>> {
        self.get_raw(key).map(|value| V::from_raw(value))
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(key, value)| (*key, *value))
    }
}

pub enum Key<'a> {
    String(&'a str),
    Localized(LocalizedKey<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalizedKey<'a> {
    pub key: &'a str,
    pub lang: &'a str,
    pub country: Option<&'a str>,
    pub modifier: Option<&'a str>,
}

impl LocalizedKey<'_> {
    fn matches(&self) -> Vec<String> {
        let (key, lang) = (self.key, self.lang);
        let mut matches = Vec::with_capacity(5);

        // `lang_COUNTRY@MODIFIER`
        if let (Some(country), Some(modifier)) = (self.country, self.modifier) {
            matches.push(format!("{key}[{lang}_{country}@{modifier}]"));
        }

        // `lang_COUNTRY`
        if let Some(country) = self.country {
            matches.push(format!("{key}[{lang}_{country}]"));
        }

        // `lang@MODIFIER`
        if let Some(modifier) = self.modifier {
            matches.push(format!("{key}[{lang}@{modifier}]"));
        }

        // `lang`
        matches.push(format!("{key}[{lang}]"));

        // default value
        matches.push(key.to_string());

        matches
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(value)
    }
}

impl<'a> From<LocalizedKey<'a>> for Key<'a> {
    fn from(value: LocalizedKey<'a>) -> Self {
        Self::Localized(value)
    }
}

pub trait FromRaw: Sized {
    fn from_raw(raw: &str) -> Result<Self, ParseError>;
}

/// Parses values of types `string`, `localestring` and `iconstring`.
impl FromRaw for String {
    fn from_raw(value: &str) -> Result<Self, ParseError> {
        Ok(value_parser::string(value)?)
    }
}

/// Parses values of types `strings`, `localestrings` and `iconstrings`.
impl FromRaw for Vec<String> {
    fn from_raw(value: &str) -> Result<Self, ParseError> {
        Ok(value_parser::strings(value)?)
    }
}

/// Parses values of type `boolean`.
impl FromRaw for bool {
    fn from_raw(value: &str) -> Result<Self, ParseError> {
        Ok(value_parser::boolean(value)?)
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::{DesktopFile, DesktopFileError, LocalizedKey};

    #[test]
    fn desktop_file_empty() {
        // should simply succeed
        let _file = DesktopFile::parse("").unwrap();
    }

    #[test]
    fn desktop_file_simple() {
        let file = DesktopFile::parse(indoc! {"
            [group1]
            k1=v1
            k2=v2
            [group2]
            k3=v3
        "})
        .unwrap();

        // simple getters
        assert_eq!(file.group("group1").unwrap().get_raw("k1").unwrap(), "v1");
        assert_eq!(file.group("group1").unwrap().get_raw("k2").unwrap(), "v2");
        assert!(file.group("group1").unwrap().get_raw("k3").is_none());

        assert!(file.group("group2").unwrap().get_raw("k1").is_none());
        assert!(file.group("group2").unwrap().get_raw("k2").is_none());
        assert_eq!(file.group("group2").unwrap().get_raw("k3").unwrap(), "v3");

        assert!(file.group("group3").is_none());
    }

    #[test]
    fn desktop_file_localized() {
        let file = DesktopFile::parse(indoc! {"
            [group]
            Name=default value
            Name[sr_YU]=localized sr_YU
            Name[sr@Latn]=localized sr@Latn
            Name[sr]=localized sr
        "})
        .unwrap();

        let group = file.group("group").unwrap();
        assert_eq!(
            group
                .get_raw(LocalizedKey {
                    key: "Name",
                    lang: "sr",
                    country: Some("YU"),
                    modifier: Some("Latn"),
                })
                .unwrap(),
            "localized sr_YU"
        );
        assert_eq!(
            group
                .get_raw(LocalizedKey {
                    key: "Name",
                    lang: "sr",
                    country: Some("YU"),
                    modifier: None,
                })
                .unwrap(),
            "localized sr_YU"
        );
        assert_eq!(
            group
                .get_raw(LocalizedKey {
                    key: "Name",
                    lang: "sr",
                    country: None,
                    modifier: Some("Latn"),
                })
                .unwrap(),
            "localized sr@Latn"
        );
        assert_eq!(
            group
                .get_raw(LocalizedKey {
                    key: "Name",
                    lang: "sr",
                    country: None,
                    modifier: None,
                })
                .unwrap(),
            "localized sr"
        );
        assert_eq!(
            group
                .get_raw(LocalizedKey {
                    key: "Name",
                    lang: "de",
                    country: None,
                    modifier: None,
                })
                .unwrap(),
            "default value"
        );
    }

    #[test]
    fn desktop_file_error_parse() {
        let err = DesktopFile::parse(indoc! {"
            [group[name]
            k=v
        "})
        .unwrap_err();
        assert!(matches!(err, DesktopFileError::Parse(_)));
    }

    #[test]
    fn desktop_file_error_entry_outside_of_group() {
        let err = DesktopFile::parse(indoc! {"
            k=v
        "})
        .unwrap_err();
        assert_eq!(err, DesktopFileError::EntryOutsideOfGroup("k"));
    }

    #[test]
    fn desktop_file_error_duplicate_group() {
        let err = DesktopFile::parse(indoc! {"
            [group1]
            k1=v1
            [group2]
            k2=v2
            [group1]
            k3=v3
        "})
        .unwrap_err();
        assert_eq!(err, DesktopFileError::DuplicateGroup("group1"));
    }

    #[test]
    fn desktop_file_error_duplicate_key() {
        let err = DesktopFile::parse(indoc! {"
            [group1]
            k1=v1
            k2=v2
            k1=v3
        "})
        .unwrap_err();
        assert_eq!(err, DesktopFileError::DuplicateKey("k1"));
    }

    #[test]
    fn localized_key_matches() {
        // lang_COUNTRY@MODIFIER
        let locale_key = LocalizedKey {
            key: "key",
            lang: "de",
            country: Some("AT"),
            modifier: Some("euro"),
        };
        assert_eq!(
            locale_key.matches(),
            vec![
                "key[de_AT@euro]",
                "key[de_AT]",
                "key[de@euro]",
                "key[de]",
                "key",
            ]
        );

        // lang_COUNTRY
        let locale_key = LocalizedKey {
            key: "key",
            lang: "de",
            country: Some("AT"),
            modifier: None,
        };
        assert_eq!(locale_key.matches(), vec!["key[de_AT]", "key[de]", "key",]);

        // lang@MODIFIER
        let locale_key = LocalizedKey {
            key: "key",
            lang: "de",
            country: None,
            modifier: Some("euro"),
        };
        assert_eq!(
            locale_key.matches(),
            vec!["key[de@euro]", "key[de]", "key",]
        );

        // lang
        let locale_key = LocalizedKey {
            key: "key",
            lang: "de",
            country: None,
            modifier: None,
        };
        assert_eq!(locale_key.matches(), vec!["key[de]", "key",]);
    }
}
