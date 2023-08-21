mod ui;

use eframe::egui;
use freedesktop_desktop_entry as desktop;
use std::fs;
use ui::*;

#[derive(Default)]
struct App {
    input: String,
    entries: Vec<String>,
}

impl App {
    fn new(entries: Vec<String>, cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.memory_mut(|m| m.data.insert_temp(egui::Id::new("meow"), 0usize));

        // scale the ui up a bit
        ctx.set_pixels_per_point(1.5);

        // add monofur, use as default proportional font
        // TODO: can we do this without explicit paths and using the system's fonts?
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "monofur".to_owned(),
            egui::FontData::from_static(include_bytes!("/usr/share/fonts/TTF/monof55.ttf")),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "monofur".to_owned());
        ctx.set_fonts(fonts);

        Self {
            input: String::new(),
            entries,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_input = self.input.clone();
        let filtered_entries = self
            .entries
            .iter()
            .filter(|e| {
                e.to_ascii_lowercase()
                    .contains(&current_input.to_ascii_lowercase())
            })
            .collect::<Vec<_>>();

        let data = ToffeeData {
            mode: "drun",
            counter: Some((filtered_entries.len(), self.entries.len())),
            entries: &filtered_entries,
        };
        let mut toffee = Toffee::new("toffee", data, &mut self.input, |ui, entry| ui.label(entry));

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.add(&mut toffee);
            });
    }
}

fn main() {
    let mut entries = vec![];

    let desktop_files = fs::read_dir("applications/").unwrap();
    for entry in desktop_files {
        let entry = entry.unwrap();
        let path = entry.path();

        let contents = fs::read_to_string(&path).unwrap();
        let entry = desktop::DesktopEntry::decode(&path, &contents).unwrap();

        entries.push(entry.groups["Desktop Entry"]["Name"].0.to_owned());
    }

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::emath::Vec2::new(500.0, 200.0)),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "toffee",
        native_options,
        Box::new(|cc| Box::new(App::new(entries, cc))),
    )
    .expect("app run failed");
}
