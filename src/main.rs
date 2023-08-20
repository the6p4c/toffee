use std::fs;

use eframe::egui;
use freedesktop_desktop_entry as desktop;

struct QueryEdit<'m, 't> {
    mode: &'m str,
    query: &'t mut dyn egui::TextBuffer,
}

impl<'m, 't> QueryEdit<'m, 't> {
    fn new(mode: &'m str, query: &'t mut dyn egui::TextBuffer) -> Self {
        Self { mode, query }
    }
}

impl<'m, 't> egui::Widget for QueryEdit<'m, 't> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(self.mode);
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(self.query).frame(false),
            )
        })
        .inner
    }
}

#[derive(Default)]
struct QueryResult<'s> {
    entry: &'s str,
    is_even: bool,
}

impl<'s> QueryResult<'s> {
    fn new(entry: &'s str, is_even: bool) -> Self {
        Self { entry, is_even }
    }
}

impl<'s> egui::Widget for QueryResult<'s> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let fill = if self.is_even {
            egui::Color32::RED
        } else {
            egui::Color32::GREEN
        };

        egui::Frame::none()
            .fill(fill)
            .show(ui, |ui| {
                ui.set_min_width(ui.max_rect().width());
                ui.label(self.entry)
            })
            .inner
    }
}

#[derive(Default)]
struct App {
    query: String,
    entries: Vec<String>,
}

impl App {
    fn new(entries: Vec<String>, cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;

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
            query: String::new(),
            entries,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_query = self.query.clone().to_ascii_lowercase();
        let filtered_entries = self
            .entries
            .iter()
            .filter(|e| e.to_lowercase().contains(&current_query));

        egui::TopBottomPanel::top("query")
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.add(QueryEdit::new("drun", &mut self.query));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                // remove vertical gaps between each result
                ui.style_mut().spacing.item_spacing.y = 0.0;

                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for (i, entry) in filtered_entries.enumerate() {
                                ui.add(QueryResult::new(entry, i % 2 == 0));
                            }
                        });
                    });
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
