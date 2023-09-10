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

#[derive(Clone, Copy)]
enum Semicolons {
    Escaped,
    Raw,
}

peg::parser! {
    pub grammar value_parser() for str {
        rule string_escape(semicolons: Semicolons) -> char
            = "\\" c:(
                "s" { ' ' }
                / "n" { '\n' }
                / "t" { '\t' }
                / "r" { '\r' }
                / ";" {?
                    match semicolons {
                        Semicolons::Escaped => Ok(';'),
                        Semicolons::Raw => Err("")
                    }
                }
                / "\\" { '\\' }
                / expected!("")
            ) { c };
        rule string_char(semicolons: Semicolons) -> char
            = [^';']
            / ";" {?
                match semicolons {
                    Semicolons::Escaped => Err(""),
                    Semicolons::Raw => Ok(';')
                }
            };

        rule string_internal(semicolons: Semicolons) -> String
            = s:(string_escape(semicolons) / string_char(semicolons))* {
                s.iter().collect::<String>()
            };
        rule string_raw_semicolons() -> String = string_internal(Semicolons::Raw);
        rule string_escaped_semicolons() -> String = string_internal(Semicolons::Escaped);

        pub rule string() -> String = string_raw_semicolons();
        pub rule strings() -> Vec<String> = ss:(string_escaped_semicolons() ** ";") {
            let mut ss = ss;

            // Optional termination with `;` results in an empty string as the final entry. Remove
            // it if it's not the only entry.
            let len = ss.len();
            if len > 1 && ss[len - 1].is_empty() {
                ss.pop();
            }

            ss
        };

        pub rule boolean() -> bool = "true" { true } / "false" { false };
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

#[cfg(test)]
mod value_tests {
    use super::value_parser::*;
    use crate::{assert_errors, assert_parses};

    #[test]
    fn parse_string() {
        // Strings can empty
        assert_parses!(string(r""), "".to_string());
        // ... or not
        assert_parses!(string(r"puppy"), "puppy".to_string());
        // They can contain single escape sequences
        assert_parses!(string(r"\s"), " ".to_string());
        assert_parses!(string(r"\n"), "\n".to_string());
        assert_parses!(string(r"\t"), "\t".to_string());
        assert_parses!(string(r"\r"), "\r".to_string());
        assert_parses!(string(r"\\"), "\\".to_string());
        // ... or many
        assert_parses!(
            string(r"a \s b \n c \t d \r e \\ f"),
            "a   b \n c \t d \r e \\ f".to_string()
        );
    }

    #[test]
    fn parse_strings() {
        // String lists cannot be empty, they always contain at least one (maybe empty) string
        assert_parses!(strings(r""), vec!["".to_string()]);
        assert_parses!(strings(r";"), vec!["".to_string()]);
        assert_parses!(strings(r"dog"), vec!["dog".to_string()]);
        assert_parses!(strings(r"dog;"), vec!["dog".to_string()]);
        // ... but can of course contain several strings
        assert_parses!(
            strings(r"dog;cat;bird"),
            vec!["dog".to_string(), "cat".to_string(), "bird".to_string()]
        );
        assert_parses!(
            strings(r"dog;cat;bird;"),
            vec!["dog".to_string(), "cat".to_string(), "bird".to_string()]
        );
        // Strings in the list can contain semicolons, if they're escaped
        assert_parses!(
            strings(r"dog\;cat;bird"),
            vec!["dog;cat".to_string(), "bird".to_string()]
        );
        assert_parses!(
            strings(r"dog\;cat;bird;"),
            vec!["dog;cat".to_string(), "bird".to_string()]
        );
        assert_parses!(
            strings(r"dog;cat\;bird"),
            vec!["dog".to_string(), "cat;bird".to_string()]
        );
        assert_parses!(
            strings(r"dog;cat\;bird;"),
            vec!["dog".to_string(), "cat;bird".to_string()]
        );
        assert_parses!(
            strings(r"dog;cat;bird\;"),
            vec!["dog".to_string(), "cat".to_string(), "bird;".to_string()]
        );
        assert_parses!(
            strings(r"dog\;cat\;bird;"),
            vec!["dog;cat;bird".to_string()]
        );
        assert_parses!(
            strings(r"dog\;cat\;bird\;"),
            vec!["dog;cat;bird;".to_string()]
        );
    }

    #[test]
    fn parse_boolean() {
        // Booleans are either true or false
        assert_parses!(boolean("true"), true);
        assert_parses!(boolean("false"), false);
        // Anything else isn't a boolean
        assert_errors!(boolean("blorp"));
    }
}
