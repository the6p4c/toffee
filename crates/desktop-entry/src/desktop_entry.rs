use crate::FromValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecArgument {
    String(String),
    FieldCode(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    program: String,
    arguments: Vec<ExecArgument>,
}

peg::parser! {
    grammar exec_parser() for str {
        rule program() -> &'input str = $([^' ']+);

        rule argument_field_code() -> char = "%" fc:[^' '] { fc };

        // TODO: is this the correct escape behaviour?
        rule argument_quoted_string_escape() -> char = "\\" c:[_] { c };
        rule argument_quoted_string_char() -> char = [^'\\' | '\"'];
        rule argument_quoted_string() -> String
            = "\"" s:(argument_quoted_string_escape() / argument_quoted_string_char())* "\"" {
                s.iter().collect::<String>()
            };

        // TODO: how to handle reserved characters?
        rule argument_string() -> &'input str = $([^' ']+);

        rule argument() -> ExecArgument
            = fc:argument_field_code() {
                match fc {
                    '%' => ExecArgument::String("%".to_string()),
                    _ => ExecArgument::FieldCode(fc),
                }
            }
            / s:argument_quoted_string() { ExecArgument::String(s) }
            / s:argument_string() { ExecArgument::String(s.to_string()) };

        pub rule exec() -> (String, Vec<ExecArgument>)
            = p:program() a:(" " a:(argument() ** " ") { a })? {
                let program = p.to_string();
                let arguments = a.unwrap_or_default();

                (program, arguments)
            };
    }
}

impl FromValue for Exec {
    fn from_value(value: &str) -> Result<Self, crate::ParseError> {
        let value = String::from_value(value)?;
        let (program, arguments) = exec_parser::exec(&value)?;

        Ok(Exec { program, arguments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdrpp() {
        assert_eq!(
            Exec::from_value("/usr/bin/sdrpp").unwrap(),
            Exec {
                program: "/usr/bin/sdrpp".to_string(),
                arguments: vec![],
            }
        );
    }

    #[test]
    fn ipython() {
        assert_eq!(
            Exec::from_value("kitty python -m IPython").unwrap(),
            Exec {
                program: "kitty".to_string(),
                arguments: vec![
                    ExecArgument::String("python".to_string()),
                    ExecArgument::String("-m".to_string()),
                    ExecArgument::String("IPython".to_string()),
                ],
            }
        );
    }

    #[test]
    fn audacity() {
        assert_eq!(
            Exec::from_value("env UBUNTU_MENUPROXY=0 audacity %F").unwrap(),
            Exec {
                program: "env".to_string(),
                arguments: vec![
                    ExecArgument::String("UBUNTU_MENUPROXY=0".to_string()),
                    ExecArgument::String("audacity".to_string()),
                    ExecArgument::FieldCode('F'),
                ],
            }
        );
    }

    #[test]
    fn kate() {
        assert_eq!(
            Exec::from_value("kate -b %U").unwrap(),
            Exec {
                program: "kate".to_string(),
                arguments: vec![
                    ExecArgument::String("-b".to_string()),
                    ExecArgument::FieldCode('U'),
                ],
            }
        );
    }

    #[test]
    fn love() {
        assert_eq!(
            Exec::from_value("/usr/bin/love %f").unwrap(),
            Exec {
                program: "/usr/bin/love".to_string(),
                arguments: vec![ExecArgument::FieldCode('f')],
            }
        );
    }

    #[test]
    fn openstreetmap_geo_handler() {
        assert_eq!(
            Exec::from_value(
                r#"kde-geo-uri-handler --coordinate-template "https://www.openstreetmap.org/#map=<Z>/<LAT>/<LON>" --query-template "https://www.openstreetmap.org/search?query=<Q>" --fallback "https://www.openstreetmap.org" %u"#
            ).unwrap(),
            Exec {
                program: "kde-geo-uri-handler".to_string(),
                arguments: vec![
                    ExecArgument::String("--coordinate-template".to_string()),
                    ExecArgument::String(
                        "https://www.openstreetmap.org/#map=<Z>/<LAT>/<LON>".to_string()
                    ),
                    ExecArgument::String("--query-template".to_string()),
                    ExecArgument::String(
                        "https://www.openstreetmap.org/search?query=<Q>".to_string()
                    ),
                    ExecArgument::String("--fallback".to_string()),
                    ExecArgument::String("https://www.openstreetmap.org".to_string()),
                    ExecArgument::FieldCode('u'),
                ],
            }
        );
    }

    #[test]
    fn emacsclient_mail() {
        assert_eq!(
            Exec::from_value(
                r#"sh -c "u=\\$(echo \\"\\$1\\" | sed 's/[\\\\\\"]/\\\\\\\\&/g'); exec /usr/bin/emacsclient --alternate-editor= --display=\\"\\$DISPLAY\\" --eval \\"(message-mailto \\\\\\"\\$u\\\\\\")\\"" sh %u"#
            ).unwrap(),
            Exec {
                program: "sh".to_string(),
                arguments: vec![
                    ExecArgument::String("-c".to_string()),
                    ExecArgument::String(
                        r#"u=$(echo "$1" | sed 's/[\"]/\\&/g'); exec /usr/bin/emacsclient --alternate-editor= --display="$DISPLAY" --eval "(message-mailto \"$u\")""#.to_string()
                    ),
                    ExecArgument::String("sh".to_string()),
                    ExecArgument::FieldCode('u'),
                ],
            }
        );
    }

    #[test]
    fn lone_percent() {
        assert_eq!(
            Exec::from_value("program x %% y").unwrap(),
            Exec {
                program: "program".to_string(),
                arguments: vec![
                    ExecArgument::String("x".to_string()),
                    ExecArgument::String("%".to_string()),
                    ExecArgument::String("y".to_string()),
                ],
            }
        );
    }
}
