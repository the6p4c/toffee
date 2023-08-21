use eframe::egui;
use freedesktop_desktop_entry as desktop;
use std::fs;

use super::Mode;
use crate::toffee::{Toffee, ToffeeData};

pub struct DRun {
    entries: Vec<String>,
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

            entries.push(entry.groups["Desktop Entry"]["Name"].0.to_owned());
        }

        Self { entries }
    }
}

impl Mode for DRun {
    fn update(&mut self, ui: &mut egui::Ui, input: &mut String) {
        let filtered_entries: Vec<_> = self
            .entries
            .iter()
            .map(|e| &e as &str)
            .filter(|e| e.to_lowercase().contains(&input.to_lowercase()))
            .collect();
        let data = ToffeeData {
            mode: "drun",
            counter: Some((filtered_entries.len(), self.entries.len())),
            entries: filtered_entries,
        };

        let toffee = Toffee::new("toffee", data, input).show(ui, |ui, entry| {
            ui.label(entry);
        });

        if toffee.input_changed() {
            eprintln!("input changed: {}", input);
        }
        if let Some(selected_entry) = toffee.selected_entry() {
            eprintln!("selected: {}", selected_entry);
        }
    }
}
