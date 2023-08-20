use eframe::egui;

pub struct ToffeeData<'a, Entry> {
    pub mode: &'a str,
    pub counter: Option<(usize, usize)>,
    pub entries: &'a [Entry],
}

pub struct Toffee<'a, Entry, EntryWidget>
where
    EntryWidget: Fn(&mut egui::Ui, &Entry),
{
    data: ToffeeData<'a, Entry>,
    input: &'a mut dyn egui::TextBuffer,
    entry_widget: EntryWidget,
}

impl<'a, Entry, EntryWidget> Toffee<'a, Entry, EntryWidget>
where
    EntryWidget: Fn(&mut egui::Ui, &Entry),
{
    pub fn new(
        data: ToffeeData<'a, Entry>,
        input: &'a mut dyn egui::TextBuffer,
        entry_widget: EntryWidget,
    ) -> Self {
        Self {
            data,
            input,
            entry_widget,
        }
    }
}

impl<Entry, EntryWidget> egui::Widget for Toffee<'_, Entry, EntryWidget>
where
    EntryWidget: Fn(&mut egui::Ui, &Entry),
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let data = self.data;

        let resp = egui::TopBottomPanel::top("query")
            .frame(egui::Frame::none())
            .show_inside(ui, |ui| {
                egui::SidePanel::left("query_mode")
                    .min_width(0.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        ui.add(egui::Label::new(data.mode).wrap(false));
                    });

                if let Some(counter) = data.counter {
                    egui::SidePanel::right("query_counter")
                        .min_width(0.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            ui.add(
                                egui::Label::new(format!("{}/{}", counter.0, counter.1))
                                    .wrap(false),
                            );
                        });
                }

                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::singleline(self.input).frame(false),
                )
            })
            .inner;

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show_inside(ui, |ui| {
                // remove vertical gaps between each result
                ui.style_mut().spacing.item_spacing.y = 0.0;

                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for (index, entry) in data.entries.iter().enumerate() {
                                let fill_style =
                                    EntryContainerFillStyle::from_selected_index(index, 1);
                                EntryContainer::new(fill_style).show(ui, |ui| {
                                    (self.entry_widget)(ui, entry);
                                });
                            }
                        });
                    });
            });

        resp
    }
}

enum EntryContainerFillStyle {
    Selected,
    Even,
    Odd,
}

impl EntryContainerFillStyle {
    fn from_selected_index(index: usize, selected_index: usize) -> Self {
        if index == selected_index {
            EntryContainerFillStyle::Selected
        } else if index % 2 == 0 {
            EntryContainerFillStyle::Even
        } else {
            EntryContainerFillStyle::Odd
        }
    }
}

struct EntryContainer {
    fill_style: EntryContainerFillStyle,
}

impl EntryContainer {
    fn new(fill_style: EntryContainerFillStyle) -> Self {
        Self { fill_style }
    }

    fn show<R>(self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let fill = match self.fill_style {
            EntryContainerFillStyle::Selected => egui::Color32::BLUE,
            EntryContainerFillStyle::Even => egui::Color32::RED,
            EntryContainerFillStyle::Odd => egui::Color32::GREEN,
        };

        egui::Frame::none()
            .fill(fill)
            .show(ui, |ui| {
                ui.set_min_width(ui.max_rect().width());
                add_contents(ui)
            })
            .inner
    }
}
