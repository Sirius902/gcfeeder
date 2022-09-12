use crate::{adapter::Port, feeder::Feeder, gui::app::Usb};

pub struct CalibrationPanel<'a> {
    feeders: &'a mut [Feeder<Usb>; Port::COUNT],
}

impl<'a> CalibrationPanel<'a> {
    pub fn new(feeders: &'a mut [Feeder<Usb>; Port::COUNT]) -> Self {
        Self { feeders }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);
        ui.heading("Calibration");
        ui.add_space(5.0);

        // TODO: Implement.
        let _ = self.feeders;
    }
}
