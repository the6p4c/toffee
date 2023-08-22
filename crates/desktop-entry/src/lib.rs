// https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html

#[derive(Debug)]
pub enum Line<'input> {
    Blank,
    Comment(&'input str),
    GroupHeader(&'input str),
    Entry(&'input str, &'input str),
}

peg::parser! {
    grammar desktop_entry_parser() for str {
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
            = k:key() whitespace()* "=" whitespace()* v:value() "\n" { (k, v) };

        pub rule line() -> Line<'input>
            = line_blank() { Line::Blank }
            / c:line_comment() { Line::Comment(c) }
            / gh:line_group_header() { Line::GroupHeader(gh)}
            / kv:line_entry() { let (k, v) = kv; Line::Entry(k, v) };
    }
}

#[cfg(test)]
mod tests {
    use super::desktop_entry_parser as parser;

    #[test]
    fn line_blank() {
        // Any line must end with a linefeed
        assert!(parser::line_blank("").is_err());
        // A lonely linefeed is a blank line
        assert_eq!(parser::line_blank("\n"), Ok(()));
        // Any whitespace before the linefeed is ignored
        assert_eq!(parser::line_blank(" \t     \n"), Ok(()));
    }

    #[test]
    fn line_comment() {
        // Any line must end with a linefeed
        assert!(parser::line_comment("").is_err());
        // A comment can be empty
        assert_eq!(parser::line_comment("#\n"), Ok(""));
        // Comments can contain any character, except for a linefeed
        assert_eq!(
            parser::line_comment("# this is a comment!!\r\t🥺🥺\n"),
            Ok(" this is a comment!!\r\t🥺🥺")
        )
    }

    #[test]
    fn line_group_header() {
        // Any line must end with a linefeed
        assert!(parser::line_group_header("").is_err());
        // Group names can be simple strings
        assert_eq!(parser::line_group_header("[groupname]\n"), Ok("groupname"));
        // Group names can also be less simple strings
        assert_eq!(parser::line_group_header("[group🥺\t name]\n"), Ok("group🥺\t name"))
    }
}
