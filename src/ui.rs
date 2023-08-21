use eframe::egui;

pub struct ToffeeData<'a, Entry: 'a, EntryIter>
where
    EntryIter: IntoIterator<Item = &'a Entry>,
{
    pub mode: &'a str,
    pub counter: Option<(usize, usize)>,
    pub entries: EntryIter,
}

pub struct Toffee<'a, Entry, EntryIter, EntryWidget>
where
    EntryIter: IntoIterator<Item = &'a Entry>,
    EntryWidget: Fn(&mut egui::Ui, &Entry) -> egui::Response,
{
    data: ToffeeData<'a, Entry, EntryIter>,
    input: &'a mut dyn egui::TextBuffer,
    entry_widget: EntryWidget,
}

impl<'a, Entry, EntryIter, EntryWidget> Toffee<'a, Entry, EntryIter, EntryWidget>
where
    EntryIter: IntoIterator<Item = &'a Entry>,
    EntryWidget: Fn(&mut egui::Ui, &Entry) -> egui::Response,
{
    pub fn new(
        data: ToffeeData<'a, Entry, EntryIter>,
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

impl<'a, Entry, EntryIter, EntryWidget> egui::Widget for Toffee<'a, Entry, EntryIter, EntryWidget>
where
    EntryIter: IntoIterator<Item = &'a Entry>,
    EntryWidget: Fn(&mut egui::Ui, &Entry) -> egui::Response,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let data = self.data;

        let selected_index_delta = ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowUp) {
                -1
            } else if i.key_pressed(egui::Key::ArrowDown) {
                1
            } else {
                0
            }
        });

        let selected_index: usize = ui
            .memory(|m| m.data.get_temp(egui::Id::new("selected_index")))
            .unwrap_or_default();
        let selected_index_changed = selected_index_delta != 0;
        if selected_index_changed {
            let selected_index = selected_index.wrapping_add_signed(selected_index_delta);
            ui.memory_mut(|m| {
                m.data
                    .insert_temp(egui::Id::new("selected_index"), selected_index)
            });
        }

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
            //.frame(egui::Frame::none()) // TODO: we want this, but it causes an overlap
            .show_inside(ui, |ui| {
                // remove vertical gaps between each result
                ui.style_mut().spacing.item_spacing.y = 0.0;

                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.max_rect().width());
                        ui.vertical(|ui| {
                            for (index, entry) in data.entries.into_iter().enumerate() {
                                let fill_style = EntryContainerFillStyle::from_selected_index(
                                    index,
                                    selected_index,
                                );
                                EntryContainer::new(fill_style).show(ui, |ui| {
                                    let widget = (self.entry_widget)(ui, entry);
                                    if selected_index_changed && selected_index == index {
                                        widget.scroll_to_me(None);
                                    }
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
            EntryContainerFillStyle::Selected => egui::Color32::from_rgb(0x10, 0x42, 0x59),
            EntryContainerFillStyle::Even => egui::Color32::from_gray(27),
            EntryContainerFillStyle::Odd => egui::Color32::from_gray(35),
        };

        egui::Frame::none()
            .inner_margin(1.0)
            .fill(fill)
            .show(ui, |ui| {
                ui.set_min_width(ui.max_rect().width());
                add_contents(ui)
            })
            .inner
    }
}
