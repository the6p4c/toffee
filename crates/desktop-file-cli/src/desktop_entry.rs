use std::fs;

use crate::CliError;
use desktop_file::desktop_entry::DesktopEntry;
use desktop_file::DesktopFile;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Path to a desktop file to read
    path: std::path::PathBuf,
}

pub fn main(args: Args) -> Result<(), CliError> {
    let contents = fs::read_to_string(args.path)
        .map_err(|err| CliError::new("could not read .desktop file", err.to_string()))?;
    let file = DesktopFile::parse(&contents)
        .map_err(|err| CliError::new("could not parse .desktop file", err.to_string()))?;

    let desktop_entry = DesktopEntry::try_from_file(&file).map_err(|err| {
        CliError::new(
            "could not parse .desktop file as desktop entry",
            err.to_string(),
        )
    })?;

    println!("Common keys");
    println!("===========");
    println!("{:#?}", desktop_entry.common);
    println!();
    println!("Type-specific");
    println!("=============");
    println!("{:#?}", desktop_entry.for_type);

    Ok(())
}
