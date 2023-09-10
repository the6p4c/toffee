use std::env;
use std::fs;

use desktop_file::{desktop_entry, DesktopFile};

fn main() -> Result<(), String> {
    let mut args = env::args().fuse();
    args.next(); // skip argv[0]
    let filename = args
        .next()
        .ok_or("expected the path to a .desktop file as the first command-line argument")?;
    let group_name = args.next();
    let key = args.next();
    let value_type = args.next();
    if args.next().is_some() {
        return Err("too many command-line arguments".to_string());
    }

    let text = fs::read_to_string(filename)
        .map_err(|err| format!("could not read .desktop file - {err}"))?;
    let file = DesktopFile::parse(&text)
        .map_err(|err| format!("could not parse .desktop file - {err}"))?;

    let (group, group_name) = match group_name {
        None => {
            println!("{file:#?}");
            return Ok(());
        }
        Some(group_name) => (
            file.group(&group_name)
                .ok_or_else(|| format!("could not find group {group_name}"))?,
            group_name,
        ),
    };

    let (key, value, meta) = match key.as_deref() {
        None => {
            println!("[{group_name}]");
            println!("{group:#?}");
            return Ok(());
        }
        Some(key) => {
            enum Error<'a> {
                UnknownValueType(&'a str),
                NotFound,
                Parse(&'a str),
            }

            let value_type = value_type.as_deref().unwrap_or("RAW_QUOTED");
            let value = || match value_type {
                "RAW_QUOTED" => {
                    let value = group.get(key).ok_or(Error::NotFound)?;

                    Ok((format!("{value:#?}"), "raw (quoted)".to_string()))
                }
                "RAW" => {
                    let value = group.get(key).ok_or(Error::NotFound)?;

                    Ok((value.to_string(), "raw".to_string()))
                }
                "string" | "localestring" | "iconstring" => {
                    let value: String = group
                        .get_value(key)
                        .ok_or(Error::NotFound)?
                        .map_err(|_| Error::Parse("String"))?;

                    Ok((value.to_string(), value_type.to_string()))
                }
                "strings" | "localestrings" | "iconstrings" => {
                    let value: Vec<String> = group
                        .get_value(key)
                        .ok_or(Error::NotFound)?
                        .map_err(|_| Error::Parse("Vec<String>"))?;

                    let len = value.len();
                    let word = if len == 1 { "item" } else { "items" };
                    Ok((
                        value.join("\n===\n"),
                        format!("{value_type} ({len} {word})"),
                    ))
                }
                "boolean" => {
                    let value: bool = group
                        .get_value(key)
                        .ok_or(Error::NotFound)?
                        .map_err(|_| Error::Parse("bool"))?;

                    Ok((value.to_string(), "boolean".to_string()))
                }
                "DE_Exec" => {
                    let desktop_entry::Exec { program, arguments } = group
                        .get_value(key)
                        .ok_or(Error::NotFound)?
                        .map_err(|_| Error::Parse("desktop_entry::Exec"))?;

                    let num_args = arguments.len();
                    let word = if num_args == 1 { "arg" } else { "args" };

                    let value = format!(
                        "{program}{}",
                        arguments
                            .into_iter()
                            .enumerate()
                            .map(|(i, argument)| {
                                let prefix = format!("\n=== ${}", i + 1);
                                match argument {
                                    desktop_entry::ExecArgument::String(s) => {
                                        format!("{prefix} -- string\n{s}")
                                    }
                                    desktop_entry::ExecArgument::FieldCode(fc) => {
                                        format!("{prefix} -- field code\n%{fc}")
                                    }
                                }
                            })
                            .collect::<Vec<String>>()
                            .join("")
                    );

                    Ok((value, format!("{value_type} ({num_args} {word})")))
                }
                value_type => Err(Error::UnknownValueType(value_type)),
            };

            let (value, meta) = value().map_err(|err| match err {
                Error::UnknownValueType(value_type) => format!("unknown value type {value_type}"),
                Error::NotFound => format!("key {key} not found in group {group_name}"),
                Error::Parse(rust_type) => {
                    format!("could not parse key {key} from group {group_name} as {value_type} -> {rust_type}")
                }
            })?;

            (key, value, meta)
        }
    };

    println!("[{group_name}].{key} -- {meta}");
    println!("{value}");

    Ok(())
}
