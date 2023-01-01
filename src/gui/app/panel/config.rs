use enum_iterator::all;

use crate::{
    adapter::Port,
    config::Config,
    gui::{util::no_close_popup_below_widget, ERROR_COLOR},
};

pub struct ConfigEditor<'a> {
    config: &'a mut Config,
    state: State,
}

impl<'a> ConfigEditor<'a> {
    pub fn new(config: &'a mut Config, state: Option<State>) -> Self {
        Self {
            config,
            state: state.map(|s| s.reset()).unwrap_or_default(),
        }
    }

    pub fn into_state(self) -> State {
        self.state
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let State {
            message,
            dirty,
            add_state,
        } = &mut self.state;

        ui.horizontal(|ui| {
            if *dirty {
                ui.heading("Config*");
            } else {
                ui.heading("Config");
            }

            ui.separator();

            if ui.button("Save").clicked() {
                *message = Some(Message::Save);
                *dirty = false;
            }

            if ui.button("Reload").clicked() {
                *message = Some(Message::Reload);
                *dirty = false;
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Active Profile");

            let grid = egui::Grid::new("active-profiles").num_columns(5);
            grid.show(ui, |ui| {
                for p in all::<Port>() {
                    if ui.button("Edit").clicked() {
                        *message = Some(Message::EditProfile {
                            name: self.config.profile.selected[p.index()].clone(),
                        });
                    }

                    let action_profile_name = self.config.profile.selected[p.index()].clone();
                    let mut profile_action: Option<ProfileAction> = None;

                    let add_popup_id = ui.make_persistent_id(format!("add {:?}", p));
                    let remove_popup_id = ui.make_persistent_id(format!("remove {:?}", p));

                    let add_response = ui.button("New");
                    if add_response.clicked() {
                        ui.memory().toggle_popup(add_popup_id);
                    }

                    no_close_popup_below_widget(ui, add_popup_id, &add_response, |ui| {
                        let mut add_popup =
                            AddPopup::new(add_state.take().unwrap_or_default(), self.config);
                        profile_action = add_popup.update(ui);
                        *add_state = Some(add_popup.into_state());
                    });

                    let remove_response = ui.button("Delete");
                    if remove_response.clicked() {
                        ui.memory().toggle_popup(remove_popup_id);
                    }

                    no_close_popup_below_widget(ui, remove_popup_id, &remove_response, |ui| {
                        let mut remove_popup = RemovePopup::new(
                            &action_profile_name,
                            self.config.profile.list.len() <= 1,
                        );
                        profile_action = remove_popup.update(ui);
                    });

                    egui::ComboBox::from_id_source(format!("Port {:?}", p))
                        .selected_text(&self.config.profile.selected[p.index()])
                        .show_ui(ui, |ui| {
                            for (name, _) in self.config.profile.list.iter() {
                                let res = ui.selectable_value(
                                    &mut self.config.profile.selected[p.index()],
                                    name.clone(),
                                    name.as_str(),
                                );

                                if res.changed() {
                                    *dirty = true;
                                }
                            }
                        });

                    match profile_action {
                        Some(ProfileAction::Add(name)) => {
                            let action_profile = self
                                .config
                                .profile
                                .list
                                .get(&action_profile_name)
                                .cloned()
                                .expect("Active profile exists");

                            *dirty = true;
                            self.config.profile.selected[p.index()] = name.clone();
                            self.config.profile.list.insert(name, action_profile);
                        }
                        Some(ProfileAction::Remove(name)) => {
                            self.config.profile.list.remove(&name);
                            *dirty = true;

                            let other_name = self
                                .config
                                .profile
                                .list
                                .keys()
                                .next()
                                .expect("At least two profiles exist");

                            for selected in self.config.profile.selected.iter_mut() {
                                if *selected == name {
                                    *selected = other_name.clone();
                                }
                            }
                        }
                        None => {}
                    }

                    ui.end_row();
                }
            });

            ui.separator();

            ui.label("Input Server");
            egui::Grid::new("input-server")
                .num_columns(4)
                .show(ui, |ui| {
                    for p in all::<Port>() {
                        let input_server = &mut self.config.input_server[p.index()];
                        ui.label(format!("Port {:?}", p));
                        if ui.checkbox(&mut input_server.enabled, "Enabled").changed() {
                            *dirty = true;
                        }

                        let mut port_buf = format!("{}", input_server.port);
                        if ui.text_edit_singleline(&mut port_buf).changed() {
                            if port_buf.is_empty() {
                                input_server.port = 0;
                            } else if let Ok(p) = port_buf.parse::<u16>() {
                                input_server.port = p;
                            }
                            *dirty = true;
                        }

                        ui.label("UDP Port");
                        ui.end_row();
                    }
                });
        });
    }
}

#[derive(Default)]
pub struct State {
    message: Option<Message>,
    dirty: bool,
    add_state: Option<AddState>,
}

impl State {
    pub fn message(&mut self) -> Option<Message> {
        self.message.take()
    }

    fn reset(mut self) -> Self {
        self.message = None;
        self
    }
}

#[derive(Eq, PartialEq)]
pub enum Message {
    Reload,
    Save,
    EditProfile { name: String },
}

pub enum ProfileAction {
    Add(String),
    Remove(String),
}

struct AddPopup<'a> {
    state: AddState,
    profile_action: Option<ProfileAction>,
    config: &'a Config,
}

impl<'a> AddPopup<'a> {
    pub fn new(state: AddState, config: &'a Config) -> Self {
        Self {
            state,
            profile_action: None,
            config,
        }
    }

    pub fn into_state(self) -> AddState {
        self.state
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let AddPopup {
            state: AddState { name, action },
            profile_action,
            config,
        } = self;

        let mut next_action = None;

        let set_profile_action = |profile_action: &mut Option<ProfileAction>, name: &str| {
            *profile_action = Some(ProfileAction::Add(name.to_owned()));
        };

        ui.set_min_width(200.0);

        match action {
            AddAction::Input { err } => {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(name);
                    ui.label("Name");
                });

                if let Some(err) = err.as_ref() {
                    ui.colored_label(ERROR_COLOR, err.as_str());
                }

                ui.horizontal(|ui| {
                    let add = ui.button("Add");
                    let cancel = ui.button("Cancel");

                    if add.clicked() {
                        if name.is_empty() {
                            *err = Some("Profile name must not be empty".to_owned());
                        } else if config.profile.list.contains_key(name) {
                            next_action = Some(AddAction::AlreadyExists);
                        } else {
                            set_profile_action(profile_action, name);
                        }
                    }

                    if (profile_action.is_some() && add.clicked()) || cancel.clicked() {
                        name.clear();
                        ui.memory().close_popup();
                    }
                });
            }
            AddAction::AlreadyExists => {
                ui.label(format!("Profile '{}' already exists", name));

                if ui.button("Ok").clicked() {
                    name.clear();
                    ui.memory().close_popup();
                }
            }
        }

        if let Some(next) = next_action {
            *action = next;
        }
    }

    #[must_use]
    pub fn update(&mut self, ui: &mut egui::Ui) -> Option<ProfileAction> {
        self.ui(ui);
        self.profile_action.take()
    }
}

#[derive(Default)]
struct AddState {
    name: String,
    action: AddAction,
}

enum AddAction {
    Input { err: Option<String> },
    AlreadyExists,
}

impl Default for AddAction {
    fn default() -> Self {
        Self::Input { err: None }
    }
}

struct RemovePopup<'a> {
    name: &'a str,
    is_last_profile: bool,
    profile_action: Option<ProfileAction>,
}

impl<'a> RemovePopup<'a> {
    pub fn new(name: &'a str, is_last_profile: bool) -> Self {
        Self {
            name,
            is_last_profile,
            profile_action: None,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);

        if self.is_last_profile {
            ui.label("Cannot remove all profiles.");

            ui.horizontal(|ui| {
                if ui.button("Ok").clicked() {
                    ui.memory().close_popup();
                }
            });
        } else {
            ui.label(format!("Remove profile '{}'?", self.name));
            ui.colored_label(ERROR_COLOR, "The profile will be lost.");

            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    self.profile_action = Some(ProfileAction::Remove(self.name.to_owned()));
                    ui.memory().close_popup();
                }

                if ui.button("No").clicked() {
                    ui.memory().close_popup();
                }
            });
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> Option<ProfileAction> {
        self.ui(ui);
        self.profile_action.take()
    }
}
