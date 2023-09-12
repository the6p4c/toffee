use thiserror::Error;

use crate::{DesktopFile, FromRaw, Group, ParseError};

#[derive(Error, Debug)]
pub enum DesktopEntryError {
    #[error("parsing should succeed")]
    Parse(#[from] ParseError),
    #[error("desktop entry files must contain the [Desktop Entry] group")]
    DesktopEntryGroupMissing,
    #[error("desktop entry files require the {0} key to be present")]
    RequiredKeyMissing(&'static str),
}

trait GroupExt {
    fn get_required<V: FromRaw>(&self, key: &'static str) -> Result<V, DesktopEntryError>;
    fn get_optional<V: FromRaw>(&self, key: &str) -> Result<Option<V>, DesktopEntryError>;
}

impl GroupExt for Group<'_> {
    fn get_required<V: FromRaw>(&self, key: &'static str) -> Result<V, DesktopEntryError> {
        let value = self
            .get::<V>(key)
            .ok_or(DesktopEntryError::RequiredKeyMissing(key))??;
        Ok(value)
    }

    fn get_optional<V: FromRaw>(&self, key: &str) -> Result<Option<V>, DesktopEntryError> {
        let value = self.get::<V>(key).transpose()?;
        Ok(value)
    }
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
            version: group.get_optional("Version")?,
            name: group.get_required("Name")?,
            generic_name: group.get_optional("GenericName")?,
            no_display: group.get_optional("NoDisplay")?,
            comment: group.get_optional("Comment")?,
            icon: group.get_optional("Icon")?,
            hidden: group.get_optional("Hidden")?,
            only_show_in: group.get_optional("OnlyShowIn")?,
            not_show_in: group.get_optional("NotShowIn")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DesktopEntryApplication {
    pub try_exec: Option<String>,
    pub exec: Option<Exec>,
    pub path: Option<String>,
    pub terminal: Option<bool>,
    pub actions: Option<Vec<String>>,
    pub mime_type: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub startup_notify: Option<bool>,
    pub startup_wm_class: Option<String>,
    pub prefers_non_default_gpu: Option<bool>,
    pub single_main_window: Option<bool>,
}

impl DesktopEntryApplication {
    fn try_from_group(group: &Group) -> Result<Self, DesktopEntryError> {
        Ok(Self {
            try_exec: group.get_optional("TryExec")?,
            exec: group.get_optional("Exec")?,
            path: group.get_optional("Path")?,
            terminal: group.get_optional("Terminal")?,
            actions: group.get_optional("Actions")?,
            mime_type: group.get_optional("MimeType")?,
            categories: group.get_optional("Categories")?,
            keywords: group.get_optional("Keywords")?,
            startup_notify: group.get_optional("StartupNotify")?,
            startup_wm_class: group.get_optional("StartupWMClass")?,
            prefers_non_default_gpu: group.get_optional("PrefersNonDefaultGPU")?,
            single_main_window: group.get_optional("SingleMainWindow")?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum DesktopEntryType {
    Unknown,
    Application(DesktopEntryApplication),
}

impl DesktopEntryType {
    fn try_from_group(ty: &str, group: &Group) -> Result<Self, DesktopEntryError> {
        match ty {
            "Application" => Ok(Self::Application(DesktopEntryApplication::try_from_group(
                group,
            )?)),
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

        let ty: String = group.get_required("Type")?;
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

impl FromRaw for Exec {
    fn from_raw(value: &str) -> Result<Self, crate::ParseError> {
        let value = String::from_raw(value)?;
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
            Exec::from_raw("/usr/bin/sdrpp").unwrap(),
            Exec {
                program: "/usr/bin/sdrpp".to_string(),
                arguments: vec![],
            }
        );
    }

    #[test]
    fn ipython() {
        assert_eq!(
            Exec::from_raw("kitty python -m IPython").unwrap(),
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
            Exec::from_raw("env UBUNTU_MENUPROXY=0 audacity %F").unwrap(),
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
            Exec::from_raw("kate -b %U").unwrap(),
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
            Exec::from_raw("/usr/bin/love %f").unwrap(),
            Exec {
                program: "/usr/bin/love".to_string(),
                arguments: vec![ExecArgument::FieldCode('f')],
            }
        );
    }

    #[test]
    fn openstreetmap_geo_handler() {
        assert_eq!(
            Exec::from_raw(
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
            Exec::from_raw(
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
            Exec::from_raw("program x %% y").unwrap(),
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
