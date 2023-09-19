use std::fs;
use std::path::Path;
use std::process::Command;

use desktop_file::desktop_entry::{DesktopEntry, DesktopEntryType, Exec, ExecArgument};
use desktop_file::DesktopFile;
use eframe::egui;
use log::{info, trace, warn};
use serde::Deserialize;

use crate::backends::{Backend, NewBackend};

#[derive(Deserialize)]
pub struct Config {
    path: String,
}

pub struct Entry {
    name: String,
    keywords: Vec<String>,
    exec: Exec,
}

pub struct DRun {
    entries: Vec<Entry>,
}

impl NewBackend for DRun {
    type Config = Config;

    fn new(config: Self::Config) -> Self {
        let entries = match Self::read_entries(config.path) {
            Ok(entries) => entries,
            Err(err) => {
                warn!("failed to read entries - {}", err);
                vec![]
            }
        };

        Self { entries }
    }
}

impl<'entry> Backend<'entry> for DRun {
    type Entry = &'entry Entry;

    fn entries(&'entry self, query: &str) -> Vec<Self::Entry> {
        self.entries
            .iter()
            .filter(|entry| {
                let query = &query.to_lowercase();

                let name_match = entry.name.to_lowercase().contains(query);
                let keyword_match = entry
                    .keywords
                    .iter()
                    .any(|k| k.to_lowercase().contains(query));

                name_match || keyword_match
            })
            .collect()
    }

    fn entry_contents(&self, ui: &mut egui::Ui, entry: Self::Entry) {
        ui.label(&entry.name);
    }

    fn on_selected(&self, entry: Self::Entry) {
        let Exec { program, arguments } = &entry.exec;
        let arguments = arguments
            .iter()
            .flat_map(|argument| match argument {
                ExecArgument::String(s) => Some(s.clone()),
                ExecArgument::FieldCode(_) => None,
            })
            .collect::<Vec<String>>();

        info!("launching {:?} with arguments {:?}", program, arguments);

        Command::new(program).args(arguments).spawn().unwrap();
    }
}

impl DRun {
    fn read_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Entry>, String> {
        let path = path.as_ref();

        trace!("reading entries from {:?}", path);

        let dir = fs::read_dir(path)
            .map_err(|err| format!("couldn't read directory {:?} - {}", path, err))?;

        let entries = dir
            // Unwrap directory entries, reporting errors
            .flat_map(|dir_entry| match dir_entry {
                Ok(dir_entry) => {
                    trace!("reading {:?}", dir_entry);
                    Some(dir_entry)
                }
                Err(err) => {
                    warn!("reading directory entry failed - {}", err);
                    None
                }
            })
            // Read each file, reporting entries ignored due to errors
            .flat_map(|dir_entry| {
                let entry = Self::read_entry(dir_entry.path());
                match entry {
                    Ok(Some(entry)) => Some(entry),
                    Ok(None) => {
                        trace!("ignoring {:?}", dir_entry);
                        None
                    }
                    Err(err) => {
                        warn!("ignoring {:?} due to error - {}", dir_entry, err);
                        None
                    }
                }
            })
            .collect();

        Ok(entries)
    }

    fn read_entry<P: AsRef<Path>>(path: P) -> Result<Option<Entry>, String> {
        let path = path.as_ref();

        let contents = fs::read_to_string(path)
            .map_err(|err| format!("failed to read desktop file {:?} - {}", path, err))?;
        let file = DesktopFile::parse(&contents)
            .map_err(|err| format!("failed to parse desktop file {:?} - {}", path, err))?;
        let desktop_entry = DesktopEntry::try_from_file(&file)
            .map_err(|err| format!("failed to parse desktop entry {:?} - {}", path, err))?;

        let common = desktop_entry.common;
        let app = match desktop_entry.for_type {
            DesktopEntryType::Application(app) => app,
            _ => return Ok(None),
        };

        // HACK: handle entries we can't run
        if app.exec.is_none() {
            return Ok(None);
        }

        Ok(Some(Entry {
            name: common.name,
            keywords: app.keywords.unwrap_or_default(),
            exec: app.exec.unwrap(),
        }))
    }
}
