#[derive(Debug)]
pub enum Line<'input> {
    Blank,
    Comment(&'input str),
    GroupHeader(&'input str),
    Entry(&'input str, &'input str),
}

peg::parser! {
    /// Parser for the basic line-oriented structure of a desktop entry file.
    ///
    /// Data consistency is not enforced by the file parser itself.
    ///
    /// # Interpretations
    /// The specification is surprisingly unclear and leaves many potential interpretations of
    /// important aspects.
    /// - "Blank line(s)" is interpreted as "empty line(s)". That is, a blank line is a line which
    ///   contains no characters other than its terminating newline.
    /// - As pertaining to entries, the specification states that "Space before and after the equals
    ///   sign should be ignored; the `=` sign is the actual delimiter." As far as I can tell, this
    ///   is a contradiction. As such, spaces before and after the equals sign are ignored, and not
    ///   included in the key or value string.
    ///   - This is equivalent to a `trim_end_matches(' ')` on the key and `trim_start_matches(' ')`
    ///     on the key and value, respectively.
    ///   - A `find / -name "*.desktop" -exec cat {} \; 2>/dev/null | grep -E "(\\s+=|=\\s+)"` on my
    ///     system reveals that there are a non-zero amount of cases where developers/translators
    ///     have left a space character after the equals sign, for example
    ///     `Name[hi]= XMPP हँसमुख प्रसंग` from [kemoticons](https://invent.kde.org/frameworks/kemoticons/-/blob/c010010955e6ac0febe73fca1e28665e1840b4c0/src/providers/xmpp/emoticonstheme_xmpp.desktop#L33).
    /// - Is the empty string a valid key? This parser requires a non-empty key.
    ///
    /// # Deviations
    /// We deviate from the specification in a few ways, which overall make our parser more lenient.
    /// - Where the specification permits only ASCII strings, we permit UTF-8 strings. We allow
    ///   control characters even when explicitly disallowed.
    ///   - For example, the specification states that "Group names may contain all ASCII characters
    ///     except for `[` and `]` and control characters." We permit all characters except for `[`
    ///     and `]`.
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
    use super::parser::*;
    use crate::{assert_errors, assert_parses};

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
