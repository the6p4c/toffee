use std::env;
use std::fs;

use desktop_entry::DesktopFile;

fn main() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    let filename = match &args[..] {
        [_, filename] => filename,
        _ => {
            return Err(format!("error: expecting the path to a .desktop file as the first and only command-line argument"));
        }
    };

    let text = fs::read_to_string(filename)
        .map_err(|err| format!("error: could not read .desktop file - {err}"))?;
    let file = DesktopFile::parse(&text)
        .map_err(|err| format!("error: could not parse .desktop file - {err}"))?;

    dbg!(file); // TODO: nicer output

    Ok(())
}
