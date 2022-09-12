use crate::gui::log::Message;
use crossbeam::channel;
use egui::Align;

pub struct LogPanel {
    log_receiver: channel::Receiver<Message>,
    log_messages: Vec<Message>,
    auto_scroll: bool,
}

impl LogPanel {
    pub fn new(log_receiver: channel::Receiver<Message>) -> Self {
        Self {
            log_receiver,
            log_messages: Vec::new(),
            auto_scroll: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let mut received_message = false;

        while let Ok(message) = self.log_receiver.try_recv() {
            self.log_messages.push(message);
            received_message = true
        }

        ui.set_max_height(150.0);

        ui.horizontal(|ui| {
            ui.heading("Log");
            ui.separator();
            ui.checkbox(&mut self.auto_scroll, "Auto Scroll");
        });

        ui.separator();

        egui::ScrollArea::both().show(ui, |ui| {
            let grid = egui::Grid::new("messages").num_columns(3);
            grid.show(ui, |ui| {
                for message in self.log_messages.iter() {
                    message.draw(ui);
                }

                if self.auto_scroll && received_message {
                    ui.scroll_to_cursor(Some(Align::BOTTOM));
                }
            });
        });
    }
}
