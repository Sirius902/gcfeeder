use std::{convert::TryFrom, iter};

use egui::Color32;

use crate::{
    adapter::{Input, Port},
    calibration::{StickCalibration, SticksCalibration, TriggerCalibration, TriggersCalibration},
    config::Config,
    feeder::{CalibrationReceiver, Feeder, Record},
    gui::{
        app::{widget, Usb},
        util::enum_combo_ui,
    },
    util::recent_channel as recent,
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
    config: &'a Config,
    state: State,
}

impl<'a> CalibrationPanel<'a> {
    pub fn new(
        feeders: &'a mut [Feeder<Usb>; Port::COUNT],
        records: &'a [Option<Record>; Port::COUNT],
        config: &'a Config,
        state: Option<State>,
    ) -> Self {
        Self {
            feeders,
            records,
            config,
            state: state.unwrap_or_default(),
        }
    }

    #[must_use]
    pub fn into_state(mut self) -> (State, Option<ConfigUpdate>) {
        let update = self.state.config_update.take();
        (self.state, update)
    }

    #[must_use]
    fn main_stick<'b>(input: &Input) -> widget::Stick<'b> {
        widget::Stick::new(input.main_stick, Color32::WHITE)
    }

    #[must_use]
    fn c_stick<'b>(input: &Input) -> widget::Stick<'b> {
        widget::Stick::new(input.c_stick, Color32::YELLOW)
    }

    #[must_use]
    fn left_trigger<'b>(input: &Input) -> widget::Trigger<'b> {
        widget::Trigger::new(input.left_trigger, "L", Color32::WHITE)
    }

    #[must_use]
    fn right_trigger<'b>(input: &Input) -> widget::Trigger<'b> {
        widget::Trigger::new(input.right_trigger, "R", Color32::WHITE)
    }

    #[must_use]
    fn calibration_ui<R, P>(
        ui: &mut egui::Ui,
        progress: &mut P,
        connected: bool,
        add_contents: impl FnOnce(&mut egui::Ui, &mut P) -> Option<R>,
        apply_calibration: impl FnOnce(R),
    ) -> Option<Action> {
        let mut next_action = None;

        if ui.button("Cancel").clicked() {
            next_action = Some(Action::DisplayInputs);
        }

        ui.separator();

        if !connected {
            ui.label("Please reconnect the controller");
            return next_action;
        }

        let r = match add_contents(ui, progress) {
            Some(r) => r,
            None => return next_action,
        };

        ui.separator();

        ui.label("Calibration finished. Apply to profile editor?");
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

    fn should_confirm(was_a_pressed: &mut bool, input: &Input) -> bool {
        if input.button_a && !*was_a_pressed {
            *was_a_pressed = true;
            true
        } else if !input.button_a {
            *was_a_pressed = false;
            false
        } else {
            false
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let State {
            port,
            action,
            config_update,
            view_calibration,
            was_a_pressed,
        } = &mut self.state;

        ui.set_min_width(200.0);
        ui.heading("Calibration");
        ui.add_space(5.0);

        let index = port.index();
        let feeder = &self.feeders[index];

        let connected = self.records[index]
            .as_ref()
            .map(|r| r.raw_input.is_some())
            .unwrap_or(false);

        ui.add_enabled_ui(matches!(action, Action::DisplayInputs), |ui| {
            enum_combo_ui(port, "Port", ui);
        });

        ui.separator();

        let scroll_contents = |ui: &mut egui::Ui| match action {
            Action::DisplayInputs => {
                let (raw, mapped) = self.records[index]
                    .as_ref()
                    .and_then(|r| r.raw_input.zip(r.layered_input))
                    .unwrap_or_default();

                if !connected {
                    ui.label("Controller disconnected");
                    return;
                }

                ui.horizontal(|ui| {
                    if ui.button("Calibrate Sticks").clicked() {
                        let (tx, rx) = recent::channel();
                        feeder.start_calibration(tx);
                        *action = Action::CalibrateSticks(Default::default(), rx);
                    }

                    if ui.button("Calibrate Triggers").clicked() {
                        let (tx, rx) = recent::channel();
                        feeder.start_calibration(tx);
                        *action = Action::CalibrateTriggers(Default::default(), rx);
                    }
                });

                ui.checkbox(view_calibration, "View Calibration");

                let stick_to_points = |s: Option<StickCalibration>| -> Vec<[u8; 2]> {
                    s.map(|s| iter::once(s.center).chain(s.notch_points).collect())
                        .unwrap_or_default()
                };

                let trigger_to_points = |t: Option<TriggerCalibration>| -> Vec<u8> {
                    t.map(|t| vec![t.min, t.max]).unwrap_or_default()
                };

                ui.label("Raw");
                ui.horizontal(|ui| {
                    let (sticks, triggers) = self
                        .config
                        .profile
                        .selected(*port)
                        .filter(|_| *view_calibration)
                        .map(|profile| {
                            (
                                profile.calibration.stick_data,
                                profile.calibration.trigger_data,
                            )
                        })
                        .unwrap_or_default();

                    ui.add(
                        Self::main_stick(&raw)
                            .with_points(&stick_to_points(sticks.map(|s| s.main_stick))),
                    );
                    ui.add(
                        Self::c_stick(&raw)
                            .with_points(&stick_to_points(sticks.map(|s| s.c_stick))),
                    );
                    ui.add(
                        Self::left_trigger(&raw)
                            .with_markers(&trigger_to_points(triggers.map(|t| t.left_trigger))),
                    );
                    ui.add(
                        Self::right_trigger(&raw)
                            .with_markers(&trigger_to_points(triggers.map(|t| t.right_trigger))),
                    );
                });

                ui.label("Mapped");
                ui.horizontal(|ui| {
                    let stick_points =
                        stick_to_points(Some(Default::default()).filter(|_| *view_calibration));
                    let trigger_points =
                        trigger_to_points(Some(Default::default()).filter(|_| *view_calibration));

                    ui.add(Self::main_stick(&mapped).with_points(&stick_points));
                    ui.add(Self::c_stick(&mapped).with_points(&stick_points));
                    ui.add(Self::left_trigger(&mapped).with_markers(&trigger_points));
                    ui.add(Self::right_trigger(&mapped).with_markers(&trigger_points));
                });
            }
            Action::CalibrateSticks(progress, rx) => {
                let record = rx.try_recv().ok().flatten();
                let raw = record.unwrap_or_default();
                let confirm = Self::should_confirm(was_a_pressed, &raw);
                let connected = record.is_some();

                let next_action = Self::calibration_ui(
                    ui,
                    progress,
                    connected,
                    |ui, progress| {
                        ui.label("Calibrating sticks...");

                        let mut is_main_stick = true;
                        loop {
                            let (stick, pstick, name) = if is_main_stick {
                                (&raw.main_stick, &mut progress.main_stick, "main ")
                            } else {
                                (&raw.c_stick, &mut progress.c_stick, "C-")
                            };

                            let points = iter::once(pstick.center)
                                .chain(pstick.notch_points)
                                .flatten()
                                .collect::<Vec<_>>();

                            if is_main_stick {
                                ui.add(Self::main_stick(&raw).with_points(&points));
                            } else {
                                ui.add(Self::c_stick(&raw).with_points(&points));
                            }

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
                        *config_update = Some(ConfigUpdate::SticksCalibration(calibration));
                    },
                );

                if let Some(next_action) = next_action {
                    *action = next_action;
                }
            }
            Action::CalibrateTriggers(progress, rx) => {
                let record = rx.try_recv().ok().flatten();
                let raw = record.unwrap_or_default();
                let confirm = Self::should_confirm(was_a_pressed, &raw);
                let connected = record.is_some();

                let next_action = Self::calibration_ui(
                    ui,
                    progress,
                    connected,
                    |ui, progress| {
                        ui.label("Calibrating triggers...");

                        let mut is_left_trigger = true;
                        loop {
                            let (trigger, ptrigger, name) = if is_left_trigger {
                                (&raw.left_trigger, &mut progress.left_trigger, "left")
                            } else {
                                (&raw.right_trigger, &mut progress.right_trigger, "right")
                            };

                            let markers = iter::once(ptrigger.min)
                                .chain(iter::once(ptrigger.max))
                                .flatten()
                                .collect::<Vec<_>>();

                            if is_left_trigger {
                                ui.add(Self::left_trigger(&raw).with_markers(&markers));
                            } else {
                                ui.add(Self::right_trigger(&raw).with_markers(&markers));
                            }

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
                        *config_update = Some(ConfigUpdate::TriggersCalibration(calibration));
                    },
                );

                if let Some(next_action) = next_action {
                    *action = next_action;
                }
            }
        };

        egui::ScrollArea::vertical().show(ui, scroll_contents);
    }
}

pub struct State {
    port: Port,
    action: Action,
    config_update: Option<ConfigUpdate>,
    view_calibration: bool,
    was_a_pressed: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            port: Port::One,
            action: Action::default(),
            config_update: None,
            view_calibration: false,
            was_a_pressed: false,
        }
    }
}

enum Action {
    DisplayInputs,
    CalibrateSticks(SticksProgress, CalibrationReceiver),
    CalibrateTriggers(TriggersProgress, CalibrationReceiver),
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

#[derive(Debug)]
pub enum ConfigUpdate {
    SticksCalibration(SticksCalibration),
    TriggersCalibration(TriggersCalibration),
}
