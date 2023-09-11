use std::fmt;

use crate::{DesktopFile, FromValue, Group, ParseError};

#[derive(Debug)]
pub enum DesktopEntryError {
    Parse(ParseError),
    DesktopEntryGroupMissing,
    RequiredKeyMissing(&'static str),
}

impl fmt::Display for DesktopEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "parse: {err}"),
            Self::DesktopEntryGroupMissing => {
                write!(f, "[Desktop Entry] group not found")
            }
            Self::RequiredKeyMissing(key) => {
                write!(f, "required key \"{key}\"")
            }
        }
    }
}

impl From<ParseError> for DesktopEntryError {
    fn from(err: ParseError) -> Self {
        DesktopEntryError::Parse(err)
    }
}

fn get_required<V: FromValue>(group: &Group, key: &'static str) -> Result<V, DesktopEntryError> {
    let value = group
        .get_value::<V>(key)
        .ok_or(DesktopEntryError::RequiredKeyMissing(key))??;
    Ok(value)
}

fn get_optional<V: FromValue>(group: &Group, key: &str) -> Result<Option<V>, DesktopEntryError> {
    let value = group.get_value::<V>(key).transpose()?;
    Ok(value)
}

#[derive(Debug, Clone)]
pub struct DesktopEntryCommon {
    pub version: Option<String>,
    pub name: String,
    pub generic_name: Option<String>,
    pub no_display: Option<bool>,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub hidden: Option<bool>,
    pub only_show_in: Option<Vec<String>>,
    pub not_show_in: Option<Vec<String>>,
}

impl DesktopEntryCommon {
    fn try_from_group(group: &Group) -> Result<Self, DesktopEntryError> {
        Ok(Self {
            version: get_optional(group, "Version")?,
            name: get_required(group, "Name")?,
            generic_name: get_optional(group, "GenericName")?,
            no_display: get_optional(group, "NoDisplay")?,
            comment: get_optional(group, "Comment")?,
            icon: get_optional(group, "Icon")?,
            hidden: get_optional(group, "Hidden")?,
            only_show_in: get_optional(group, "OnlyShowIn")?,
            not_show_in: get_optional(group, "NotShowIn")?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum DesktopEntryType {
    Unknown,
    Application {
        try_exec: Option<String>,
        exec: Option<Exec>,
        path: Option<String>,
        terminal: Option<bool>,
        actions: Option<Vec<String>>,
        mime_type: Option<Vec<String>>,
        categories: Option<Vec<String>>,
        keywords: Option<Vec<String>>,
        startup_notify: Option<bool>,
        startup_wm_class: Option<String>,
        prefers_non_default_gpu: Option<bool>,
        single_main_window: Option<bool>,
    },
}

impl DesktopEntryType {
    fn try_from_group(ty: &str, group: &Group) -> Result<Self, DesktopEntryError> {
        match ty {
            "Application" => Ok(Self::Application {
                try_exec: get_optional(group, "TryExec")?,
                exec: get_optional(group, "Exec")?,
                path: get_optional(group, "Path")?,
                terminal: get_optional(group, "Terminal")?,
                actions: get_optional(group, "Actions")?,
                mime_type: get_optional(group, "MimeType")?,
                categories: get_optional(group, "Categories")?,
                keywords: get_optional(group, "Keywords")?,
                startup_notify: get_optional(group, "StartupNotify")?,
                startup_wm_class: get_optional(group, "StartupWMClass")?,
                prefers_non_default_gpu: get_optional(group, "PrefersNonDefaultGPU")?,
                single_main_window: get_optional(group, "SingleMainWindow")?,
            }),
            _ => Ok(Self::Unknown),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesktopEntry<'file, 'input> {
    pub group: &'file Group<'input>,
    pub common: DesktopEntryCommon,
    pub for_type: DesktopEntryType,
}

impl<'file: 'input, 'input> DesktopEntry<'file, 'input> {
    pub fn try_from_file(file: &'file DesktopFile<'input>) -> Result<Self, DesktopEntryError> {
        let group = file
            .group("Desktop Entry")
            .ok_or(DesktopEntryError::DesktopEntryGroupMissing)?;

        let ty: String = get_required(group, "Type")?;
        let common = DesktopEntryCommon::try_from_group(group)?;
        let for_type = DesktopEntryType::try_from_group(&ty, group)?;

        Ok(Self {
            group,
            common,
            for_type,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecArgument {
    String(String),
    FieldCode(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    pub program: String,
    pub arguments: Vec<ExecArgument>,
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
