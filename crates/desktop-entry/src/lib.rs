// https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html

#[derive(Debug)]
pub enum Line<'input> {
    Blank,
    Comment(&'input str),
    GroupHeader(&'input str),
    Entry(&'input str, &'input str),
}

peg::parser! {
    grammar desktop_entry() for str {
        rule whitespace()
            = [' ' | '\t']; // TODO: not all of it lol

        pub(super) rule line_blank()
            = whitespace()* "\n";

        pub(super) rule line_comment() -> &'input str
            = "#" c:$([^'\n']*) "\n" { c };

        rule group_name() -> &'input str
            = gn:$([^'[' | ']']+) { gn }; // TODO: forbid control chars
        pub(super) rule line_group_header() -> &'input str
            = "[" gn:group_name() "]\n" { gn };

        rule key() -> &'input str
            = k:$(['A'..='Z' | 'a'..='z' | '0'..='9' | '-']+) { k };
        rule value() -> &'input str
            = v:$([^'\n']*) { v };
        pub(super) rule line_entry() -> (&'input str, &'input str)
            = k:key() "=" v:value() "\n" { (k, v) };

        pub rule line() -> Line<'input>
            = line_blank() { Line::Blank }
            / c:line_comment() { Line::Comment(c) }
            / gn:line_group_header() { Line::GroupHeader(gn)}
            / kv:line_entry() { let (k, v) = kv; Line::Entry(k, v) };
    }
}

peg::parser! {
    grammar value_type() for str {
        rule ascii_string() -> &'input str
            = s:$([_]*) { s } // TODO: ascii-only
        rule utf8_string() -> &'input str
            = s:$([_]*) { s }

        pub rule string() -> &'input str
            = ascii_string()
        pub rule localestring() -> &'input str
            = utf8_string()
        pub rule iconstring() -> &'input str
            = utf8_string()
        pub rule boolean() -> bool
            = "true" { true } / "false" { false };
        pub rule numeric() -> f64
            = n:$(['+' | '-']?['0'..='9']+("." ['0'..='9']*)?) {? n.parse().or(Err("f64")) }
    }
}

#[cfg(test)]
mod tests {
    use super::{desktop_entry, value_type};

    #[test]
    fn desktop_entry_line_blank() {
        // Any line must end with a linefeed
        assert!(desktop_entry::line_blank("").is_err());
        // A lonely linefeed is a blank line
        assert_eq!(desktop_entry::line_blank("\n"), Ok(()));
        // Any whitespace before the linefeed is ignored
        assert_eq!(desktop_entry::line_blank(" \t     \n"), Ok(()));
    }

    #[test]
    fn desktop_entry_line_comment() {
        // Any line must end with a linefeed
        assert!(desktop_entry::line_comment("").is_err());
        // A comment can be empty
        assert_eq!(desktop_entry::line_comment("#\n"), Ok(""));
        // Comments can contain any character, except for a linefeed
        assert_eq!(
            desktop_entry::line_comment("# this is a comment!!\r\t🥺🥺\n"),
            Ok(" this is a comment!!\r\t🥺🥺")
        )
    }

    #[test]
    fn desktop_entry_line_group_header() {
        // Any line must end with a linefeed
        assert!(desktop_entry::line_group_header("").is_err());
        // Group names can be simple strings
        assert_eq!(
            desktop_entry::line_group_header("[groupname]\n"),
            Ok("groupname")
        );
        // Group names can also be less simple strings
        assert_eq!(
            desktop_entry::line_group_header("[group🥺\t name]\n"),
            Ok("group🥺\t name")
        )
    }

    #[test]
    fn desktop_entry_line_entry() {
        // Any line must end with a linefeed
        assert!(desktop_entry::line_entry("").is_err());
        // Keys must be simple strings, values can be simple strings
        assert_eq!(
            desktop_entry::line_entry("key=value\n"),
            Ok(("key", "value"))
        );
        // Values can be less simple strings
        assert_eq!(
            desktop_entry::line_entry("key=val🥺🥺\tue\n"),
            Ok(("key", "val🥺🥺\tue"))
        );
        // Whitespace before and after the = separator is part of the key and value
        // TODO: how is this actually implemented?
        // assert_eq!(desktop_entry::line_entry("key  = value"), Ok(("key  ", " value")));
    }

    #[test]
    fn value_type_string() {
        // ASCII-only data is passed through unchanged
        assert_eq!(value_type::string("abc\t.\t123"), Ok("abc\t.\t123"));
        // Non-ASCII is forbidden
        // TODO: implement this
        // assert!(value_type::string("abc\t.💜🐶💜\t123").is_err());
    }

    #[test]
    fn value_type_localestring() {
        // Data is passed through unchanged
        assert_eq!(
            value_type::localestring("abc\t.💜🐶💜\t123"),
            Ok("abc\t.💜🐶💜\t123")
        );
    }

    #[test]
    fn value_type_iconstring() {
        // Data is passed through unchanged
        assert_eq!(
            value_type::iconstring("abc\t.💜🐶💜\t123"),
            Ok("abc\t.💜🐶💜\t123")
        );
    }

    #[test]
    fn value_type_boolean() {
        // Booleans are either true or false
        assert_eq!(value_type::boolean("true"), Ok(true));
        assert_eq!(value_type::boolean("false"), Ok(false));
        // Anything else is not a boolean
        assert!(value_type::boolean("blorp").is_err());
    }

    #[test]
    fn value_type_numeric() {
        // Integers are valid
        assert_eq!(value_type::numeric("1"), Ok(1.0));
        assert_eq!(value_type::numeric("10"), Ok(10.0));
        assert_eq!(value_type::numeric("+10"), Ok(10.0));
        assert_eq!(value_type::numeric("-5"), Ok(-5.0));
        // Floating point values are valid
        assert_eq!(value_type::numeric("1.2"), Ok(1.2));
        assert_eq!(value_type::numeric("10.02"), Ok(10.02));
        assert_eq!(value_type::numeric("+10.02"), Ok(10.02));
        assert_eq!(value_type::numeric("-5.5"), Ok(-5.5));
    }
}
