use egui::Color32;

use crate::{
    adapter::{Input, Port},
    feeder::Feeder,
    gui::app::{widget, Usb},
};

pub struct CalibrationPanel<'a> {
    feeders: &'a mut [Feeder<Usb>; Port::COUNT],
    inputs: &'a [Option<Input>; Port::COUNT],
    state: State,
}

impl<'a> CalibrationPanel<'a> {
    pub fn new(
        feeders: &'a mut [Feeder<Usb>; Port::COUNT],
        inputs: &'a [Option<Input>; Port::COUNT],
        state: Option<State>,
    ) -> Self {
        Self {
            feeders,
            inputs,
            state: state.unwrap_or_default(),
        }
    }

    pub fn into_state(self) -> State {
        self.state
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let State { port, action } = &mut self.state;

        ui.set_min_width(200.0);
        ui.heading("Calibration");
        ui.add_space(5.0);

        match action {
            Action::DisplayInputs => {
                let index = port.index();
                let inputs = self.inputs[index].unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.add(widget::Stick::new(inputs.main_stick, Color32::WHITE));
                    ui.add(widget::Stick::new(inputs.c_stick, Color32::YELLOW));
                    ui.add(widget::Trigger::new(
                        inputs.left_trigger,
                        "L",
                        Color32::WHITE,
                    ));
                    ui.add(widget::Trigger::new(
                        inputs.right_trigger,
                        "R",
                        Color32::WHITE,
                    ));
                });
            }
            Action::CalibrateSticks(progress) => {
                todo!()
            }
            Action::CalibrateTriggers(progress) => {
                todo!()
            }
        }
    }
}

pub struct State {
    port: Port,
    action: Action,
}

impl Default for State {
    fn default() -> Self {
        Self {
            port: Port::One,
            action: Action::default(),
        }
    }
}

enum Action {
    DisplayInputs,
    CalibrateSticks(StickProgress),
    CalibrateTriggers(TriggersProgress),
}

impl Default for Action {
    fn default() -> Self {
        Self::DisplayInputs
    }
}

struct StickProgress {
    pub notch_points: [Option<[u8; 2]>; 8],
    pub center: Option<[u8; 2]>,
}

struct TriggerProgress {
    pub min: Option<u8>,
    pub max: Option<u8>,
}

struct SticksProgress {
    main_stick: StickProgress,
    c_stick: StickProgress,
}

struct TriggersProgress {
    left: TriggerProgress,
    right: TriggerProgress,
}
