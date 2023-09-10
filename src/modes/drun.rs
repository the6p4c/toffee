use desktop_file::desktop_entry::{Exec, ExecArgument};
use desktop_file::DesktopFile;
use eframe::egui;
use log::{info, trace};
use std::fs;
use std::process::Command;

use crate::modes::Mode;
use crate::toffee::{Toffee, ToffeeData};

struct Entry {
    name: String,
    keywords: Vec<String>,
    exec: Exec,
}

pub struct DRun {
    entries: Vec<Entry>,
}

impl DRun {
    pub fn new() -> Self {
        let mut entries = vec![];

        let desktop_files = fs::read_dir("applications/").unwrap();
        for entry in desktop_files {
            let entry = entry.unwrap();
            trace!("reading {:?}", entry);

            let contents = fs::read_to_string(&entry.path()).unwrap();
            let file = DesktopFile::parse(&contents).unwrap();

            let desktop_entry = file.group("Desktop Entry").unwrap();
            let name = desktop_entry.get_value::<String>("Name").unwrap().unwrap();
            let keywords = desktop_entry
                .get_value::<Vec<String>>("Keywords")
                .unwrap_or_else(|| Ok(vec![]))
                .unwrap();
            let exec = desktop_entry.get_value::<Exec>("Exec").unwrap().unwrap();

            entries.push(Entry {
                name,
                keywords,
                exec,
            });
        }

        Self { entries }
    }
}

impl Mode for DRun {
    fn update(&mut self, ui: &mut egui::Ui, input: &mut String) {
        let filtered_entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| {
                let input = &input.to_lowercase();

                let name_match = entry.name.to_lowercase().contains(input);
                let keyword_match = entry
                    .keywords
                    .iter()
                    .any(|k| k.to_lowercase().contains(input));

                name_match || keyword_match
            })
            .collect();
        let data = ToffeeData {
            mode: "drun",
            counter: Some((filtered_entries.len(), self.entries.len())),
            entries: filtered_entries,
        };

        let toffee = Toffee::new("toffee", data, input).show(ui, |ui, entry| {
            ui.label(&entry.name);
        });

        if toffee.input_changed() {
            eprintln!("input changed: {}", input);
        }
        if let Some(entry) = toffee.selected_entry() {
            let Exec { program, arguments } = &entry.exec;
            let arguments = arguments
                .into_iter()
                .flat_map(|argument| match argument {
                    ExecArgument::String(s) => Some(s.clone()),
                    ExecArgument::FieldCode(_) => None,
                })
                .collect::<Vec<String>>();

            info!("launching {:?} with arguments {:?}", program, arguments);

            Command::new(program).args(arguments).spawn().unwrap();
        }
    }
}
