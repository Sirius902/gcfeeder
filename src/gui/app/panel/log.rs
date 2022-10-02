use crate::gui::log::Message;
use crossbeam::channel;

pub struct LogPanel {
    receiver: channel::Receiver<Message>,
    messages: Vec<(Message, usize)>,
    auto_scroll: bool,
}

impl LogPanel {
    pub fn new(receiver: channel::Receiver<Message>) -> Self {
        Self {
            receiver,
            messages: Vec::new(),
            auto_scroll: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        while let Ok(message) = self.receiver.try_recv() {
            if let Some((last_message, last_count)) = self.messages.last_mut() {
                if message == *last_message {
                    if let Some(count) = last_count.checked_add(1) {
                        *last_count = count;
                        continue;
                    }
                }
            }

            self.messages.push((message, 1));
        }

        ui.set_height(125.0);

        ui.horizontal(|ui| {
            ui.heading("Log");
            ui.separator();
            ui.checkbox(&mut self.auto_scroll, "Auto Scroll")
                .on_hover_text(
                "Automatically scroll to view new messages when the scroll bar is at the bottom.",
            );
        });

        ui.separator();

        let row_height = ui.text_style_height(&egui::TextStyle::Body);

        egui::ScrollArea::both()
            .stick_to_bottom(self.auto_scroll)
            .show_rows(ui, row_height, self.messages.len(), |ui, rows| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                let grid = egui::Grid::new("messages").num_columns(3);
                grid.show(ui, |ui| {
                    for (message, count) in rows.map(|i| &self.messages[i]) {
                        ui.horizontal(|ui| {
                            message.draw(ui);
                            if *count != 1 {
                                ui.label(format!("(x{})", *count));
                            }
                        });

                        ui.end_row();
                    }
                });
            });
    }
}
