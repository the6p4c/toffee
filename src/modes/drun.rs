use eframe::egui;
use freedesktop_desktop_entry as desktop;
use std::fs;
use std::process::Command;

use super::Mode;
use crate::toffee::{Toffee, ToffeeData};

struct Entry {
    name: String,
    exec: String,
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
            let path = entry.path();

            let contents = fs::read_to_string(&path).unwrap();
            let entry = desktop::DesktopEntry::decode(&path, &contents).unwrap();

            let name = entry.groups["Desktop Entry"]["Name"].0.to_owned();
            let exec = entry.groups["Desktop Entry"]["Exec"].0.to_owned();
            entries.push(Entry { name, exec });
        }

        Self { entries }
    }
}

impl Mode for DRun {
    fn update(&mut self, ui: &mut egui::Ui, input: &mut String) {
        let filtered_entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.name.to_lowercase().contains(&input.to_lowercase()))
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
            eprintln!("selected: {}", entry.name);
            eprintln!("    {}", entry.exec);

            // HACK: this doesn't deal with variables or quoting properly
            // see https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#exec-variables
            let mut fields = entry.exec.split(' ');
            let program = fields.next().unwrap();
            let args = fields;
            Command::new(program).args(args).spawn().unwrap();
        }
    }
}