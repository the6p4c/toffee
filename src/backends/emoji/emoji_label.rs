use egui::{Image, Vec2, Widget};

pub struct EmojiLabel<'a> {
    svg_path: &'a str,
    emoji: &'a str,
}

impl<'a> EmojiLabel<'a> {
    pub fn new(svg_path: &'a str, emoji: &'a str) -> Self {
        Self { svg_path, emoji }
    }
}

impl<'a> Widget for EmojiLabel<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let filename = self
            .emoji
            .chars()
            .map(|c| format!("{:x}", c as u32))
            .collect::<Vec<_>>()
            .join("-");
        let uri = format!("file://{}/{}.svg", self.svg_path, filename);

        ui.add(Image::new(uri).fit_to_exact_size(Vec2::new(16.0, 16.0)))
    }
}
