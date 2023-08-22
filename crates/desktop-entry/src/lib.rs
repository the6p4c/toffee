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
        // TODO: unify and move this
        // All characters, except for control characters
        rule utf8_string() -> &'input str
            = s:$([^'\x00'..='\x1f']*) { s }

        pub(super) rule line_blank()
            = "\n";

        pub(super) rule line_comment() -> &'input str
            = "#" c:$([^'\n']*) "\n" { c };

        // All ASCII characters, except for '[', ']', and control characters
        rule group_name() -> &'input str
            = gn:$(['\x20'..='\x5a' | '\x5c' | '\x5e'..='\x7e']+) { gn };
        pub(super) rule line_group_header() -> &'input str
            = "[" gn:group_name() "]\n" { gn };

        rule key() -> &'input str
            = k:$(['A'..='Z' | 'a'..='z' | '0'..='9' | '-']+) { k };
        rule value() -> &'input str
            = utf8_string();
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
        // All ASCII characters, except for control characters
        rule ascii_string() -> &'input str
            = s:$(['\x20'..='\x7e']*) { s }
        // All characters, except for control characters
        rule utf8_string() -> &'input str
            = s:$([^'\x00'..='\x1f']*) { s }

        pub rule string() -> &'input str
            = ascii_string()

        pub rule locale_string() -> &'input str
            = utf8_string()

        pub rule icon_string() -> &'input str
            = utf8_string()

        pub rule boolean() -> bool
            = "true" { true } / "false" { false };

        rule sign() = ['+' | '-'];
        rule digit() = ['0'..='9'];
        pub rule numeric() -> f64
            = n:$(sign()? digit()+ ("." digit()*)?) {? n.parse().or(Err("f64")) };
    }
}

#[cfg(test)]
mod tests {
    use super::{desktop_entry, value_type};

    macro_rules! assert_parses {
        ($parsed:expr, $expected:expr) => {
            assert_eq!($parsed, Ok($expected));
        };
    }

    macro_rules! assert_errors {
        ($parsed:expr) => {
            assert!($parsed.is_err())
        };
    }

    #[test]
    fn desktop_entry_line_blank() {
        // Only a completely empty line is a blank line
        assert_parses!(desktop_entry::line_blank("\n"), ());
        // Any whitespace before the linefeed means the line is not blank
        assert_errors!(desktop_entry::line_blank(" \n"));
        assert_errors!(desktop_entry::line_blank("\t\n"));
        // Any line must end with a linefeed
        assert_errors!(desktop_entry::line_blank(""));
    }

    #[test]
    fn desktop_entry_line_comment() {
        // A comment can be empty
        assert_parses!(desktop_entry::line_comment("#\n"), "");
        // Comments can contain any character, except for a linefeed
        assert_parses!(
            desktop_entry::line_comment("# this is a \x07 comment!!\r\t🥺🥺\n"),
            " this is a \x07 comment!!\r\t🥺🥺"
        );
        // Any line must end with a linefeed
        assert_errors!(desktop_entry::line_comment("#"));
    }

    #[test]
    fn desktop_entry_line_group_header() {
        // Group names must be ASCII strings
        assert_parses!(
            desktop_entry::line_group_header("[groupname]\n"),
            "groupname"
        );
        // ... but they cannot contain '[' or ']'
        assert_errors!(desktop_entry::line_group_header("[group[name]\n"));
        assert_errors!(desktop_entry::line_group_header("[group]name]\n"));
        // ... but they cannot contain control characters
        assert_errors!(desktop_entry::line_group_header("[group\x07name]\n"));
        // Group names cannot contain non-ASCII characters
        assert_errors!(desktop_entry::line_group_header("[group🥺name]\n"));
        // Any line must end with a linefeed
        assert_errors!(desktop_entry::line_group_header("[groupname]"));
    }

    #[test]
    fn desktop_entry_line_entry() {
        // Keys must be A-Za-z0-9- strings, values can be ASCII strings
        assert_eq!(
            desktop_entry::line_entry("key=value\n"),
            Ok(("key", "value"))
        );
        // ... but values can be non-ASCII
        assert_eq!(
            desktop_entry::line_entry("key=val🥺🥺ue\n"),
            Ok(("key", "val🥺🥺ue"))
        );
        // ... but values still cannot contain control characters
        assert_errors!(desktop_entry::line_entry("key=val🥺\x07🥺ue\n"));
        // ... and keys must still be A-Za-z0-9- strings
        assert_errors!(desktop_entry::line_entry("key!=value\n"));
        assert_errors!(desktop_entry::line_entry("k_ey=value\n"));
        assert_errors!(desktop_entry::line_entry("ke🥺y=value\n"));
        // An '=' must be present
        assert_errors!(desktop_entry::line_entry("key\n"));
        // Keys must be non-empty (TODO: is this true?)
        assert_errors!(desktop_entry::line_entry("=value\n"));
        // .. but values can be empty
        assert_parses!(desktop_entry::line_entry("key=\n"), ("key", ""));
        // Any line must end with a linefeed
        assert_errors!(desktop_entry::line_entry("key=value"));
    }

    #[test]
    fn value_type_string() {
        // Strings must be ASCII
        assert_parses!(value_type::string("abc.123"), "abc.123");
        // ... but cannot contain control characters
        assert_errors!(value_type::string("abc\x07123"));
        // Strings cannot contain non-ASCII characters
        assert_errors!(value_type::string("🥺🐶💜"));
    }

    #[test]
    fn value_type_localestring() {
        // Locale strings can be UTF-8
        assert_parses!(
            value_type::locale_string("abc.💜🐶💜.123"),
            "abc.💜🐶💜.123"
        );
        // ... but still cannot contain control characters
        assert_errors!(value_type::locale_string("abc.💜🐶💜.1\x0723"));
    }

    #[test]
    fn value_type_iconstring() {
        // Icon strings can be UTF-8
        assert_parses!(
            value_type::locale_string("abc.💜🐶💜.123"),
            "abc.💜🐶💜.123"
        );
        // ... but still cannot contain control characters
        assert_errors!(value_type::locale_string("abc.💜🐶💜.1\x0723"));
    }

    #[test]
    fn value_type_boolean() {
        // Booleans are either true or false
        assert_parses!(value_type::boolean("true"), true);
        assert_parses!(value_type::boolean("false"), false);
        // Anything else isn't a boolean
        assert_errors!(value_type::boolean("blorp"));
    }

    #[test]
    fn value_type_numeric() {
        // Integers are valid
        assert_parses!(value_type::numeric("1"), 1.0);
        assert_parses!(value_type::numeric("10"), 10.0);
        assert_parses!(value_type::numeric("+10"), 10.0);
        assert_parses!(value_type::numeric("-5"), -5.0);
        // Floating point values are valid
        assert_parses!(value_type::numeric("1.2"), 1.2);
        assert_parses!(value_type::numeric("10.02"), 10.02);
        assert_parses!(value_type::numeric("+10.02"), 10.02);
        assert_parses!(value_type::numeric("-5.5"), -5.5);
        // TODO: failing cases
    }
}
