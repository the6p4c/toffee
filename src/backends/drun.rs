use std::env::{self, VarError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{ensure, eyre, Context, Report, Result};
use desktop_file::desktop_entry::{DesktopEntry, DesktopEntryType, Exec, ExecArgument};
use desktop_file::DesktopFile;
use eframe::egui;
use itertools::chain;
use log::{error, info, trace, warn};
use serde::Deserialize;

use crate::backends::{Backend, Entries, EntriesCounter, NewBackend};

pub struct DRun {
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Paths {
    None,
    One(PathBuf),
    Many(Vec<PathBuf>),
}

impl Paths {
    fn into_vec(self) -> Vec<PathBuf> {
        match self {
            Self::None => vec![],
            Self::One(path) => vec![path],
            Self::Many(paths) => paths,
        }
    }
}

impl Default for Paths {
    fn default() -> Self {
        Self::None
    }
}

// why is serde like this
fn bool_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "bool_true")]
    include_system: bool,
    #[serde(default = "bool_true")]
    include_user: bool,
    #[serde(default)]
    include: Paths,
}

fn xdg(name: &str, default: &[impl AsRef<Path>]) -> Result<Vec<PathBuf>> {
    let value = match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(err) => Err(err).wrap_err_with(|| format!("failed to read ${name}")),
    }?;

    let paths: Vec<PathBuf> = match value {
        Some(value) => value.split(':').map(|path| path.into()).collect(),
        None => default.iter().map(|path| path.as_ref().into()).collect(),
    };

    let paths = paths
        .iter()
        .map(|path| {
            ensure!(
                path.is_absolute(),
                "path {path:?} in ${name} should be absolute"
            );

            Ok(path.join("applications"))
        })
        .collect::<Result<_, _>>()?;

    Ok(paths)
}

impl NewBackend for DRun {
    type Config = Config;

    fn new(_cc: &eframe::CreationContext<'_>, config: Self::Config) -> Self {
        let include_system = config
            .include_system
            .then(|| xdg("XDG_DATA_DIRS", &["/usr/local/share", "/usr/share"]))
            .transpose()
            .expect("include_system paths to be ok") // TODO: report error properly
            .unwrap_or_default();
        let include_user = config
            .include_user
            .then(|| {
                let home = env::var("HOME").wrap_err("$HOME should be set")?;
                let default = PathBuf::from(home).join(".local/share");

                xdg("XDG_DATA_HOME", &[default])
            })
            .transpose()
            .expect("include-user paths to be ok") // TODO: report error properly
            .unwrap_or_default();

        let include = chain!(include_system, include_user, config.include.into_vec());
        let entries = include
            .flat_map(|path| {
                Self::read_entries(path).unwrap_or_else(|err| {
                    warn!("failed to read entries - {}", err);

                    vec![]
                })
            })
            .collect();

        Self { entries }
    }
}

impl<'entry> Backend<'entry> for DRun {
    type Entry = &'entry Entry;

    fn entries(&'entry self, query: &str) -> Entries<Self::Entry> {
        let entries = self
            .entries
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
            .collect::<Vec<_>>();

        Entries {
            counter: Some(EntriesCounter {
                visible: entries.len(),
                total: self.entries.len(),
            }),
            entries,
        }
    }

    fn entry_contents(&self, ui: &mut egui::Ui, entry: Self::Entry) {
        ui.label(&entry.name);
    }

    fn on_selected(&self, entry: Self::Entry) {
        match entry.launch() {
            Ok(_) => {}
            Err(err) => {
                error!("launch failed - {}", err);
            }
        }
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
            .flat_map(|dir_entry| match Entry::read(dir_entry.path()) {
                EntryResult::Ok(entry) => Some(entry),
                EntryResult::Ignored => {
                    trace!("ignoring {:?}", dir_entry);
                    None
                }
                EntryResult::Err(err) => {
                    warn!("ignoring {:?} due to error - {}", dir_entry, err);
                    None
                }
            })
            .collect();

        Ok(entries)
    }
}

pub enum EntryResult<T, E> {
    Ok(T),
    Ignored,
    Err(E),
}

impl<T, E> From<Result<Option<T>, E>> for EntryResult<T, E> {
    fn from(value: Result<Option<T>, E>) -> Self {
        match value {
            Ok(Some(entry)) => EntryResult::Ok(entry),
            Ok(None) => EntryResult::Ignored,
            Err(err) => EntryResult::Err(err),
        }
    }
}

pub struct Entry {
    name: String,
    keywords: Vec<String>,
    exec: Exec,
}

impl Entry {
    fn read<P: AsRef<Path>>(path: P) -> EntryResult<Self, Report> {
        fn read(path: &Path) -> Result<Option<Entry>> {
            // Hack to avoid having to move to nightly to implement Try for EntryResult
            #[allow(non_snake_case)]
            let Ignored = Ok(None);

            let contents = fs::read_to_string(path)
                .wrap_err_with(|| format!("failed to read desktop file {path:?}"))?;
            let file = DesktopFile::parse(&contents)
                .map_err(|_| eyre!("TODO: fix errors from desktop-file"))
                .wrap_err_with(|| format!("failed to parse desktop file {path:?}"))?;
            let desktop_entry = DesktopEntry::try_from_file(&file)
                .wrap_err_with(|| format!("failed to parse desktop entry {path:?}"))?;

            let common = desktop_entry.common;
            let app = match desktop_entry.for_type {
                DesktopEntryType::Application(app) => app,
                _ => return Ignored,
            };

            let name = common.name;
            let keywords = app.keywords.unwrap_or_default();
            let exec = match app.exec {
                Some(exec) => exec,
                None => return Ignored,
            };

            Ok(Some(Entry {
                name,
                keywords,
                exec,
            }))
        }

        read(path.as_ref()).into()
    }

    fn launch(&self) -> Result<()> {
        let Exec { program, arguments } = &self.exec;
        let arguments = arguments
            .iter()
            .flat_map(|argument| match argument {
                ExecArgument::String(s) => Some(s.clone()),
                ExecArgument::FieldCode(_) => None,
            })
            .collect::<Vec<String>>();

        info!("launching {:?} with arguments {:?}", program, arguments);

        Command::new(program)
            .args(arguments)
            .spawn()
            .wrap_err("spawn failed")?;

        Ok(())
    }
}
