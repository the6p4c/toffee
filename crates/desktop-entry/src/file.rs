//! Rust representation of the group-key-value structure of a desktop entry file, and parser of the
//! the file's basic line-oriented structure.
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
//! - repr: The specification states that "Multiple groups may not have the same name." I don't see
//!   how this makes sense - if two groups have the same name, they are the same group. Presumably,
//!   this is intended to communicate that you cannot add keys to a previously created but not
//!   currently "active" group. As such, **we raise a "duplicate group" error in a case such as the
//!   following:**
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

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum DesktopFileError<'input> {
    Parse,
    EntryOutsideOfGroup(&'input str),
    DuplicateGroup(&'input str),
    DuplicateKey(&'input str),
}

#[derive(Debug)]
pub struct DesktopFile<'input> {
    pub(self) groups: HashMap<&'input str, HashMap<&'input str, &'input str>>,
}

impl<'input> DesktopFile<'input> {
    pub fn parse(s: &'input str) -> Result<Self, DesktopFileError> {
        // TODO: pass parse error up to caller
        let lines = parser::file(s).map_err(|_| DesktopFileError::Parse)?;

        let mut groups = HashMap::new();
        let mut current_group_name = None;
        for line in lines {
            match line {
                Line::Blank | Line::Comment(_) => {}
                Line::GroupHeader(group_name) => {
                    if groups.insert(group_name, HashMap::new()).is_some() {
                        return Err(DesktopFileError::DuplicateGroup(group_name));
                    }
                    current_group_name = Some(group_name);
                }
                Line::Entry(key, value) => {
                    let group_name =
                        current_group_name.ok_or(DesktopFileError::EntryOutsideOfGroup(key))?;

                    let group = groups.get_mut(group_name).expect("current group to exist");
                    if group.insert(key, value).is_some() {
                        return Err(DesktopFileError::DuplicateKey(key));
                    }
                }
            }
        }

        Ok(Self { groups })
    }
}

#[derive(Debug)]
pub enum Line<'input> {
    Blank,
    Comment(&'input str),
    GroupHeader(&'input str),
    Entry(&'input str, &'input str),
}

peg::parser! {
    grammar parser() for str {
        pub(super) rule line_blank() = "\n";

        pub(super) rule line_comment() -> &'input str = "#" c:$([^'\n']*) "\n" { c };

        pub(super) rule line_group_header() -> &'input str = "[" gn:$([^'[' | ']']+) "]\n" { gn };

        rule key() -> &'input str = $(['A'..='Z' | 'a'..='z' | '0'..='9' | '-']+);
        rule value() -> &'input str = $([^'\n']*);
        pub(super) rule line_entry() -> (&'input str, &'input str)
            = k:key() " "* "=" " "* v:value() "\n" { (k, v) };

        pub(super) rule line() -> Line<'input>
            = line_blank() { Line::Blank }
            / c:line_comment() { Line::Comment(c) }
            / gn:line_group_header() { Line::GroupHeader(gn)}
            / kv:line_entry() { let (k, v) = kv; Line::Entry(k, v) };

        pub rule file() -> Vec<Line<'input>> = line()*;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use indoc::indoc;

    use super::parser::*;
    use super::DesktopFile;
    use crate::file::DesktopFileError;
    use crate::{assert_errors, assert_parses};

    #[test]
    fn desktop_file_empty() {
        let file = DesktopFile::parse("").unwrap();
        assert_eq!(file.groups, HashMap::from([]));
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
        assert_eq!(
            file.groups,
            HashMap::from([
                ("group1", HashMap::from([("k1", "v1"), ("k2", "v2")])),
                ("group2", HashMap::from([("k3", "v3")]))
            ])
        );
    }

    #[test]
    fn desktop_file_error_parse() {
        let err = DesktopFile::parse(indoc! {"
            [group[name]
            k=v
        "})
        .unwrap_err();
        assert_eq!(err, DesktopFileError::Parse);
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
    fn parser_line_blank() {
        // Only a completely empty line is a blank line
        assert_parses!(line_blank("\n"), ());
        // Any whitespace before the linefeed means the line is not blank
        assert_errors!(line_blank(" \n"));
        assert_errors!(line_blank("\t\n"));
        // Any line must end with a linefeed
        assert_errors!(line_blank(""));
    }

    #[test]
    fn parser_line_comment() {
        // A comment can be empty
        assert_parses!(line_comment("#\n"), "");
        // Comments can contain any character, except for a linefeed
        assert_parses!(
            line_comment("# this is a \x07 comment!!\r\t🥺🥺\n"),
            " this is a \x07 comment!!\r\t🥺🥺"
        );
        // Any line must end with a linefeed
        assert_errors!(line_comment("#"));
    }

    #[test]
    fn parser_line_group_header() {
        // Group names are strings
        assert_parses!(line_group_header("[groupname]\n"), "groupname");
        // ... but they cannot contain '[' or ']'
        assert_errors!(line_group_header("[group[name]\n"));
        assert_errors!(line_group_header("[group]name]\n"));
        // Any line must end with a linefeed
        assert_errors!(line_group_header("[groupname]"));
    }

    #[test]
    fn parser_line_entry() {
        // Keys must be A-Za-z0-9- strings, values are strings
        assert_eq!(line_entry("key=value\n"), Ok(("key", "value")));
        // ... so values can be non-ASCII
        assert_eq!(line_entry("key=val🥺🥺ue\n"), Ok(("key", "val🥺🥺ue")));
        // ... but keys must still be A-Za-z0-9- strings
        assert_errors!(line_entry("key!=value\n"));
        assert_errors!(line_entry("k_ey=value\n"));
        assert_errors!(line_entry("ke🥺y=value\n"));
        // An '=' must be present
        assert_errors!(line_entry("key\n"));
        // Keys must be non-empty (TODO: is this true?)
        assert_errors!(line_entry("=value\n"));
        // .. but values can be empty
        assert_parses!(line_entry("key=\n"), ("key", ""));
        // Any line must end with a linefeed
        assert_errors!(line_entry("key=value"));
    }
}
