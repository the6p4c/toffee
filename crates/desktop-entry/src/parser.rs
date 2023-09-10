#[derive(Debug)]
pub enum Line<'input> {
    Blank,
    Comment(&'input str),
    GroupHeader(&'input str),
    Entry(&'input str, &'input str),
}

peg::parser! {
    pub grammar file_parser() for str {
        pub(super) rule line_blank() = "\n";

        pub(super) rule line_comment() -> &'input str = "#" c:$([^'\n']*) "\n" { c };

        pub(super) rule line_group_header() -> &'input str = "[" gn:$([^'[' | ']']+) "]\n" { gn };

        rule locale() = "[" ['A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '@']* "]";
        rule key() -> &'input str = $(['A'..='Z' | 'a'..='z' | '0'..='9' | '-']+ locale()?);
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
mod file_tests {
    use super::file_parser::*;
    use crate::{assert_errors, assert_parses};

    #[test]
    fn parse_line_blank() {
        // Only a completely empty line is a blank line
        assert_parses!(line_blank("\n"), ());
        // Any whitespace before the linefeed means the line is not blank
        assert_errors!(line_blank(" \n"));
        assert_errors!(line_blank("\t\n"));
        // Any line must end with a linefeed
        assert_errors!(line_blank(""));
    }

    #[test]
    fn parse_line_comment() {
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
    fn parse_line_group_header() {
        // Group names are strings
        assert_parses!(line_group_header("[groupname]\n"), "groupname");
        // ... but they cannot contain '[' or ']'
        assert_errors!(line_group_header("[group[name]\n"));
        assert_errors!(line_group_header("[group]name]\n"));
        // Any line must end with a linefeed
        assert_errors!(line_group_header("[groupname]"));
    }

    #[test]
    fn parse_line_entry() {
        // Keys must be A-Za-z0-9- strings, values are strings
        assert_eq!(line_entry("key=value\n"), Ok(("key", "value")));
        // ... so values can be non-ASCII
        assert_eq!(line_entry("key=val🥺🥺ue\n"), Ok(("key", "val🥺🥺ue")));
        // ... but keys must still be A-Za-z0-9- strings
        assert_errors!(line_entry("key!=value\n"));
        assert_errors!(line_entry("k_ey=value\n"));
        assert_errors!(line_entry("ke🥺y=value\n"));
        // Keys can also include a locale
        assert_eq!(
            line_entry("key[en_AU@Latn]=value\n"),
            Ok(("key[en_AU@Latn]", "value"))
        );
        // ... which must be at the end of the key
        assert_errors!(line_entry("ke[locale]y=value\n"));
        // ... and cannot contain '[' or ']'
        assert_errors!(line_entry("key[loc[ale]=value\n"));
        assert_errors!(line_entry("key[loc]ale]=value\n"));
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
