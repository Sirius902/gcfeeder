use std::convert::TryFrom;

use egui::Color32;

use crate::{
    adapter::{Input, Port},
    calibration::{StickCalibration, SticksCalibration, TriggerCalibration, TriggersCalibration},
    feeder::{Feeder, Record},
    gui::app::{widget, Usb},
};

const NOTCH_NAMES: [&str; 8] = [
    "top",
    "top-right",
    "right",
    "bottom-right",
    "bottom",
    "bottom-left",
    "left",
    "top-left",
];

pub struct CalibrationPanel<'a> {
    feeders: &'a mut [Feeder<Usb>; Port::COUNT],
    records: &'a [Option<Record>; Port::COUNT],
    state: State,
}

impl<'a> CalibrationPanel<'a> {
    pub fn new(
        feeders: &'a mut [Feeder<Usb>; Port::COUNT],
        records: &'a [Option<Record>; Port::COUNT],
        state: Option<State>,
    ) -> Self {
        Self {
            feeders,
            records,
            state: state.unwrap_or_default(),
        }
    }

    pub fn into_state(self) -> State {
        self.state
    }

    fn draw_main_stick(ui: &mut egui::Ui, input: &Input) {
        ui.add(widget::Stick::new(input.main_stick, Color32::WHITE));
    }

    fn draw_c_stick(ui: &mut egui::Ui, input: &Input) {
        ui.add(widget::Stick::new(input.c_stick, Color32::YELLOW));
    }

    fn draw_left_trigger(ui: &mut egui::Ui, input: &Input) {
        ui.add(widget::Trigger::new(
            input.left_trigger,
            "L",
            Color32::WHITE,
        ));
    }

    fn draw_right_trigger(ui: &mut egui::Ui, input: &Input) {
        ui.add(widget::Trigger::new(
            input.right_trigger,
            "R",
            Color32::WHITE,
        ));
    }

    fn controls_ui(ui: &mut egui::Ui, input: &Input) {
        ui.horizontal(|ui| {
            Self::draw_main_stick(ui, input);
            Self::draw_c_stick(ui, input);
            Self::draw_left_trigger(ui, input);
            Self::draw_right_trigger(ui, input);
        });
    }

    #[must_use]
    fn calibration_ui<R, P>(
        ui: &mut egui::Ui,
        progress: &mut P,
        add_contents: impl FnOnce(&mut egui::Ui, &mut P) -> Option<R>,
        apply_calibration: impl FnOnce(R),
    ) -> Option<Action> {
        let mut next_action = None;

        if ui.button("Cancel").clicked() {
            next_action = Some(Action::DisplayInputs);
        }

        ui.separator();
        let r = match add_contents(ui, progress) {
            Some(r) => r,
            None => return next_action,
        };
        ui.separator();

        ui.label("Calibration finished. Apply to config editor profile?");
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                apply_calibration(r);
                next_action = Some(Action::DisplayInputs);
            }

            if ui.button("Discard").clicked() {
                next_action = Some(Action::DisplayInputs);
            }
        });

        next_action
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let State {
            port,
            action,
            was_a_pressed,
        } = &mut self.state;

        let index = port.index();
        let (raw, mapped) = self.records[index]
            .as_ref()
            .and_then(|r| r.raw_input.zip(r.layered_input))
            .unwrap_or_default();

        let confirm = if raw.button_a && !*was_a_pressed {
            *was_a_pressed = true;
            true
        } else if !raw.button_a {
            *was_a_pressed = false;
            false
        } else {
            false
        };

        ui.set_min_width(200.0);
        ui.heading("Calibration");
        ui.add_space(5.0);

        match action {
            Action::DisplayInputs => {
                ui.horizontal(|ui| {
                    if ui.button("Calibrate Sticks").clicked() {
                        *action = Action::CalibrateSticks(Default::default());
                    }

                    if ui.button("Calibrate Triggers").clicked() {
                        *action = Action::CalibrateTriggers(Default::default());
                    }
                });

                ui.separator();

                ui.label("Raw");
                Self::controls_ui(ui, &raw);

                ui.label("Mapped");
                Self::controls_ui(ui, &mapped);
            }
            Action::CalibrateSticks(progress) => {
                let next_action = Self::calibration_ui(
                    ui,
                    progress,
                    |ui, progress| {
                        ui.label("Calibrating sticks...");

                        let mut is_main_stick = true;
                        loop {
                            if is_main_stick {
                                Self::draw_main_stick(ui, &raw);
                            } else {
                                Self::draw_c_stick(ui, &raw);
                            }

                            let (stick, pstick, name) = if is_main_stick {
                                (&raw.main_stick, &mut progress.main_stick, "main ")
                            } else {
                                (&raw.c_stick, &mut progress.c_stick, "C-")
                            };

                            if pstick.center.is_none() {
                                ui.label(format!("Center {}stick then press A", name));
                                if confirm {
                                    pstick.center = Some((*stick).into());
                                }
                                return None;
                            }

                            for (i, point) in pstick.notch_points.iter_mut().enumerate() {
                                if point.is_none() {
                                    ui.label(format!(
                                        "Move {}stick to center then to {} then press A",
                                        name, NOTCH_NAMES[i]
                                    ));
                                    if confirm {
                                        *point = Some((*stick).into());
                                    }
                                    return None;
                                }
                            }

                            if is_main_stick {
                                is_main_stick = false;
                            } else {
                                break;
                            }
                        }

                        Some(SticksCalibration::try_from(*progress).unwrap())
                    },
                    |calibration| {
                        log::debug!("Stick calibration finished! {:?}", calibration);
                    },
                );

                if let Some(next_action) = next_action {
                    *action = next_action;
                }
            }
            Action::CalibrateTriggers(progress) => {
                let next_action = Self::calibration_ui(
                    ui,
                    progress,
                    |ui, progress| {
                        ui.label("Calibrating triggers...");

                        let mut is_left_trigger = true;
                        loop {
                            if is_left_trigger {
                                Self::draw_left_trigger(ui, &raw);
                            } else {
                                Self::draw_right_trigger(ui, &raw);
                            }

                            let (trigger, ptrigger, name) = if is_left_trigger {
                                (&raw.left_trigger, &mut progress.left_trigger, "left")
                            } else {
                                (&raw.right_trigger, &mut progress.right_trigger, "right")
                            };

                            if ptrigger.min.is_none() {
                                ui.label(format!(
                                    "Completely release {} trigger then press A",
                                    name
                                ));
                                if confirm {
                                    ptrigger.min = Some(*trigger);
                                }
                                return None;
                            }

                            if ptrigger.max.is_none() {
                                ui.label(format!(
                                    "Press {} trigger all the way in then press A",
                                    name
                                ));
                                if confirm {
                                    ptrigger.max = Some(*trigger);
                                }
                                return None;
                            }

                            if is_left_trigger {
                                is_left_trigger = false;
                            } else {
                                break;
                            }
                        }

                        Some(TriggersCalibration::try_from(*progress).unwrap())
                    },
                    |calibration| {
                        log::debug!("Trigger calibration finished! {:?}", calibration);
                    },
                );

                if let Some(next_action) = next_action {
                    *action = next_action;
                }
            }
        }
    }
}

pub struct State {
    port: Port,
    action: Action,
    was_a_pressed: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            port: Port::One,
            action: Action::default(),
            was_a_pressed: false,
        }
    }
}

enum Action {
    DisplayInputs,
    CalibrateSticks(SticksProgress),
    CalibrateTriggers(TriggersProgress),
}

impl Default for Action {
    fn default() -> Self {
        Self::DisplayInputs
    }
}

#[derive(Default, Copy, Clone)]
struct StickProgress {
    pub notch_points: [Option<[u8; 2]>; 8],
    pub center: Option<[u8; 2]>,
}

impl TryFrom<StickProgress> for StickCalibration {
    type Error = ();

    fn try_from(value: StickProgress) -> Result<Self, Self::Error> {
        let notch_points = value
            .notch_points
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(|v| <[[u8; 2]; 8]>::try_from(v).unwrap());

        notch_points
            .and_then(|notch_points| {
                value.center.map(|center| Self {
                    notch_points,
                    center,
                })
            })
            .ok_or(())
    }
}

#[derive(Default, Copy, Clone)]
struct TriggerProgress {
    pub min: Option<u8>,
    pub max: Option<u8>,
}

impl TryFrom<TriggerProgress> for TriggerCalibration {
    type Error = ();

    fn try_from(value: TriggerProgress) -> Result<Self, Self::Error> {
        value
            .min
            .and_then(|min| value.max.map(|max| Self { min, max }))
            .ok_or(())
    }
}

#[derive(Default, Copy, Clone)]
struct SticksProgress {
    main_stick: StickProgress,
    c_stick: StickProgress,
}

impl TryFrom<SticksProgress> for SticksCalibration {
    type Error = ();

    fn try_from(value: SticksProgress) -> Result<Self, Self::Error> {
        let main_stick = StickCalibration::try_from(value.main_stick);
        let c_stick = StickCalibration::try_from(value.c_stick);

        main_stick.and_then(|main_stick| {
            c_stick.map(|c_stick| Self {
                main_stick,
                c_stick,
            })
        })
    }
}

#[derive(Default, Copy, Clone)]
struct TriggersProgress {
    left_trigger: TriggerProgress,
    right_trigger: TriggerProgress,
}

impl TryFrom<TriggersProgress> for TriggersCalibration {
    type Error = ();

    fn try_from(value: TriggersProgress) -> Result<Self, Self::Error> {
        let left_trigger = TriggerCalibration::try_from(value.left_trigger);
        let right_trigger = TriggerCalibration::try_from(value.right_trigger);

        left_trigger.and_then(|left_trigger| {
            right_trigger.map(|right_trigger| Self {
                left_trigger,
                right_trigger,
            })
        })
    }
}
