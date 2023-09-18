mod drun;

use eframe::egui;

pub use drun::DRun;

pub trait NewMode {
    type Config;

    fn new(config: Self::Config) -> Self;
}

pub trait Mode<'entry> {
    type Entry: Copy;

    fn entries(&'entry self, query: &str) -> Vec<Self::Entry>;
    fn entry_contents(&self, ui: &mut egui::Ui, entry: Self::Entry);

    fn on_selected(&self, entry: Self::Entry);
}
