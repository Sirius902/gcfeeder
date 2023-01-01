use crate::{
    calibration::{StickCalibration, TriggerCalibration, NOTCHES},
    config::{Config, Profile},
    gui::util::{enum_combo_ui, enum_option_combo_ui},
};

const U8_TEXT_WIDTH: f32 = 30.0;

pub struct ProfilePanel<'a> {
    config: &'a mut Config,
    profile_name: &'a str,
    state: State,
}

impl<'a> ProfilePanel<'a> {
    pub fn new(
        config: &'a mut Config,
        profile_name: &'a str,
        state: Option<State>,
    ) -> ProfilePanel<'a> {
        let state = state.map(|s| s.reset()).unwrap_or_else(|| {
            let profile = *config
                .profile
                .list
                .get(profile_name)
                .expect("Active profile exists");

            State {
                message: None,
                profile,
            }
        });

        Self {
            config,
            profile_name,
            state,
        }
    }

    #[must_use]
    pub fn into_state(mut self) -> (State, Option<Message>) {
        let message = self.state.message.take();
        (self.state, message)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Profile Editor");
            ui.separator();

            if ui.button("Save").clicked() {
                self.config
                    .profile
                    .list
                    .insert(self.profile_name.to_string(), self.state.profile);
                self.state.message = Some(Message::SaveReload);
            }

            if ui.button("Cancel").clicked() {
                self.state.message = Some(Message::Cancel);
            }
        });

        ui.separator();

        ui.label(format!("Editing profile \"{}\"", self.profile_name));

        egui::ScrollArea::vertical().show(ui, |ui| {
            let profile = &mut self.state.profile;

            enum_combo_ui(&mut profile.driver, "Driver", ui);
            enum_combo_ui(&mut profile.rumble, "Rumble", ui);

            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    ui.set_max_width(ui.available_width() * 0.33);

                    let mut buf = format!("{:.2}", profile.analog_scale);
                    if ui.text_edit_singleline(&mut buf).changed() {
                        if buf.is_empty() {
                            profile.analog_scale = 0.0;
                        } else if let Ok(scale) = buf.parse::<f64>() {
                            profile.analog_scale = scale;
                        }
                    }
                });

                ui.label("Analog Scale");
            });

            ui.group(|ui| {
                ui.label("ViGEm");

                enum_combo_ui(&mut profile.vigem_config.pad, "Pad", ui);
                enum_combo_ui(&mut profile.vigem_config.trigger_mode, "Trigger Mode", ui);
            });

            ui.group(|ui| {
                ui.label("Calibration");

                ui.checkbox(&mut profile.calibration.enabled, "Enabled");

                let stick_calibration_ui =
                    |c: &mut StickCalibration, id: usize, ui: &mut egui::Ui| {
                        const NOTCH_IDENT: [&str; NOTCHES] =
                            ["T", "TR", "R", "BR", "B", "BL", "L", "TL"];

                        ui.push_id(id, |ui| {
                            ui.label("Notch Points");
                            egui::ScrollArea::horizontal().show(ui, |ui| {
                                egui::Grid::new("notch-points").num_columns(NOTCHES).show(
                                    ui,
                                    |ui| {
                                        for row in 0..c.notch_points[0].len() + 1 {
                                            for (col, ident) in
                                                c.notch_points.iter_mut().zip(NOTCH_IDENT)
                                            {
                                                if row == 0 {
                                                    ui.label(ident);
                                                } else {
                                                    ui.scope(|ui| {
                                                        ui.set_max_width(U8_TEXT_WIDTH);
                                                        let val = &mut col[row - 1];
                                                        let mut buf = format!("{}", *val);
                                                        if ui
                                                            .text_edit_singleline(&mut buf)
                                                            .clicked()
                                                        {
                                                            if buf.is_empty() {
                                                                *val = 0;
                                                            } else if let Ok(n) = buf.parse::<u8>()
                                                            {
                                                                *val = n;
                                                            }
                                                        }
                                                    });
                                                }
                                            }

                                            ui.end_row();
                                        }
                                    },
                                );
                            });

                            ui.label("Center");
                            for val in c.center.iter_mut() {
                                ui.scope(|ui| {
                                    ui.set_max_width(U8_TEXT_WIDTH);
                                    let mut buf = format!("{}", *val);
                                    if ui.text_edit_singleline(&mut buf).clicked() {
                                        if buf.is_empty() {
                                            *val = 0;
                                        } else if let Ok(n) = buf.parse::<u8>() {
                                            *val = n;
                                        }
                                    }
                                });
                            }
                        });
                    };

                let trigger_calibration_ui =
                    |c: &mut TriggerCalibration, id: usize, ui: &mut egui::Ui| {
                        ui.push_id(id, |ui| {
                            let mut first = true;

                            loop {
                                let (val, name) = if first {
                                    (&mut c.min, "Min")
                                } else {
                                    (&mut c.max, "Max")
                                };

                                ui.horizontal(|ui| {
                                    ui.scope(|ui| {
                                        ui.set_max_width(U8_TEXT_WIDTH);
                                        let mut buf = format!("{}", *val);
                                        if ui.text_edit_singleline(&mut buf).changed() {
                                            if buf.is_empty() {
                                                *val = 0;
                                            } else if let Ok(n) = buf.parse::<u8>() {
                                                *val = n;
                                            }
                                        }
                                    });

                                    ui.label(name);
                                });

                                if first {
                                    first = false;
                                } else {
                                    break;
                                }
                            }
                        });
                    };

                if let Some(c) = profile.calibration.stick_data.as_mut() {
                    ui.label("Main Stick");
                    stick_calibration_ui(&mut c.main_stick, 0, ui);

                    ui.label("C-Stick");
                    stick_calibration_ui(&mut c.c_stick, 1, ui);
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Missing stick calibration data");
                        if ui.button("Use default").clicked() {
                            profile.calibration.stick_data = Some(Default::default());
                        }
                    });
                }

                if let Some(c) = profile.calibration.trigger_data.as_mut() {
                    ui.label("Left Trigger");
                    trigger_calibration_ui(&mut c.left_trigger, 0, ui);

                    ui.label("Right Trigger");
                    trigger_calibration_ui(&mut c.right_trigger, 1, ui);
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Missing trigger calibration data");
                        if ui.button("Use default").clicked() {
                            profile.calibration.trigger_data = Some(Default::default());
                        }
                    });
                }
            });

            enum_option_combo_ui(&mut profile.ess.inversion_mapping, "Ess Inversion", ui);
        });
    }
}

pub struct State {
    message: Option<Message>,
    profile: Profile,
}

impl State {
    fn reset(mut self) -> Self {
        self.message = None;
        self
    }
}

pub enum Message {
    SaveReload,
    Cancel,
}
