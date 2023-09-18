mod drun;

pub use drun::DRun;

use eframe::egui;

pub trait Mode<'entry> {
    type Entry: Copy;
    type Config;

    fn new(config: Self::Config) -> Self;

    fn entries(&'entry self, query: &str) -> Vec<Self::Entry>;
    fn entry_contents(&self, ui: &mut egui::Ui, entry: Self::Entry);

    fn on_selected(&self, entry: Self::Entry);
}
