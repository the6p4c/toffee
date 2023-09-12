use clap::ValueEnum;
use std::{fs, string};

use crate::CliError;
use desktop_file::{desktop_entry, DesktopFile, FromRaw, Group};

#[derive(ValueEnum, Debug, Clone, Copy)]
#[value(rename_all = "PascalCase")]
enum ValueType {
    Raw,
    RawQuoted,
    #[value(name = "string", alias("localestring"), alias("iconstring"))]
    String,
    #[value(name = "strings", alias("localestrings"), alias("iconstrings"))]
    Strings,
    #[value(name = "boolean")]
    Boolean,
    DesktopEntryExec,
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Path to a desktop file to read
    path: std::path::PathBuf,
    /// Group name to retrieve
    group_name: Option<String>,
    /// Key within group to retrieve
    key: Option<String>,
    /// Value type to interpret value as
    #[arg(default_value = "RawQuoted")]
    value_type: ValueType,
}

fn print_file(file: DesktopFile) -> Result<(), CliError> {
    println!("{:#?}", file);

    Ok(())
}

fn print_group(group_name: &str, group: &Group) -> Result<(), CliError> {
    println!("[{group_name}]");
    println!("{group:#?}");

    Ok(())
}

fn print_value(
    group_name: &str,
    group: &Group,
    key: &str,
    value_type: ValueType,
) -> Result<(), CliError> {
    trait ToStringFromStr {
        fn to_string(
            group_name: &str,
            group: &Group,
            key: &str,
        ) -> Result<(String, Option<String>), CliError> {
            let value = group
                .get_raw(key.as_ref())
                .ok_or_else(|| format!("could not find [{group_name}].{key}"))?;

            Self::to_string_from_str(value)
        }

        fn to_string_from_str(s: &str) -> Result<(String, Option<String>), CliError>;
    }

    trait ToStringFromValue {
        type Value: FromRaw;

        fn to_string(
            group_name: &str,
            group: &Group,
            key: &str,
        ) -> Result<(String, Option<String>), CliError> {
            let value = group
                .get(key.as_ref())
                .ok_or_else(|| format!("could not find [{group_name}].{key}"))?
                .map_err(|err| {
                    CliError::new(
                        format!("could not parse [{group_name}].{key} as string"),
                        err.to_string(),
                    )
                })?;

            Self::to_string_from_value(value)
        }

        fn to_string_from_value(value: Self::Value) -> Result<(String, Option<String>), CliError>;
    }

    struct ValueTypeRaw;

    impl ToStringFromStr for ValueTypeRaw {
        fn to_string_from_str(s: &str) -> Result<(String, Option<String>), CliError> {
            Ok((s.to_string(), None))
        }
    }

    struct ValueTypeRawQuoted;

    impl ToStringFromStr for ValueTypeRawQuoted {
        fn to_string_from_str(s: &str) -> Result<(String, Option<String>), CliError> {
            Ok((format!("{s:#?}"), None))
        }
    }

    struct ValueTypeString;

    impl ToStringFromValue for ValueTypeString {
        type Value = String;

        fn to_string_from_value(value: Self::Value) -> Result<(String, Option<String>), CliError> {
            Ok((value, None))
        }
    }

    struct ValueTypeStrings;

    impl ToStringFromValue for ValueTypeStrings {
        type Value = Vec<String>;

        fn to_string_from_value(value: Self::Value) -> Result<(String, Option<String>), CliError> {
            let len = value.len();
            let word = if len != 1 { "items" } else { "item" };

            Ok((value.join("\n===\n"), Some(format!("{len} {word}"))))
        }
    }

    struct ValueTypeBoolean;

    impl ToStringFromValue for ValueTypeBoolean {
        type Value = bool;

        fn to_string_from_value(value: Self::Value) -> Result<(String, Option<String>), CliError> {
            Ok((value.to_string(), None))
        }
    }

    struct ValueTypeDesktopEntryExec;

    impl ToStringFromValue for ValueTypeDesktopEntryExec {
        type Value = desktop_entry::Exec;

        fn to_string_from_value(
            desktop_entry::Exec { program, arguments }: Self::Value,
        ) -> Result<(String, Option<String>), CliError> {
            struct Arg(usize, desktop_entry::ExecArgument);

            impl string::ToString for Arg {
                fn to_string(&self) -> String {
                    use desktop_entry::ExecArgument::*;
                    match self {
                        Arg(i, String(s)) => format!("\n=== ${} -- string\n{s}", i + 1),
                        Arg(i, FieldCode(fc)) => {
                            format!("\n=== ${} -- field code\n%{fc}", i + 1)
                        }
                    }
                }
            }

            let num_args = arguments.len();
            let word = if num_args == 1 { "arg" } else { "args" };

            let value = format!(
                "{program}{}",
                arguments
                    .into_iter()
                    .enumerate()
                    .map(|(i, argument)| Arg(i, argument).to_string())
                    .collect::<Vec<String>>()
                    .join("")
            );
            let meta = format!("desktop entry Exec ({num_args} {word})");

            Ok((value, Some(meta)))
        }
    }

    let (value, meta) = match value_type {
        ValueType::Raw => ValueTypeRaw::to_string(&group_name, group, &key)?,
        ValueType::RawQuoted => ValueTypeRawQuoted::to_string(&group_name, group, &key)?,
        ValueType::String => ValueTypeString::to_string(&group_name, group, &key)?,
        ValueType::Strings => ValueTypeStrings::to_string(&group_name, group, &key)?,
        ValueType::Boolean => ValueTypeBoolean::to_string(&group_name, group, &key)?,
        ValueType::DesktopEntryExec => {
            ValueTypeDesktopEntryExec::to_string(&group_name, group, &key)?
        }
    };

    let meta = match meta {
        Some(meta) => format!(" --- {meta}"),
        None => "".to_string(),
    };

    println!("[{group_name}].{key}{meta}");
    println!("{value}");

    Ok(())
}

pub fn main(args: Args) -> Result<(), CliError> {
    let contents = fs::read_to_string(args.path)
        .map_err(|err| CliError::new("could not read .desktop file", err.to_string()))?;
    let file = DesktopFile::parse(&contents)
        .map_err(|err| CliError::new("could not parse .desktop file", err.to_string()))?;

    let (group, group_name) = match args.group_name {
        None => {
            return print_file(file);
        }
        Some(group_name) => {
            let group = file
                .group(&group_name)
                .ok_or_else(|| format!("could not find [{group_name}]"))?;

            (group, group_name)
        }
    };

    match args.key {
        None => {
            return print_group(&group_name, group);
        }
        Some(key) => {
            return print_value(&group_name, group, &key, args.value_type);
        }
    }
}
