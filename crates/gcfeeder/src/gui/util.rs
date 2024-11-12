use std::fmt;

use egui::WidgetText;
use enum_iterator::{all, Sequence};

pub fn enum_combo_ui<T>(e: &mut T, label: impl Into<WidgetText>, ui: &mut egui::Ui)
where
    T: Sequence + Copy + fmt::Debug + Eq,
{
    egui::ComboBox::from_label(label)
        .selected_text(format!("{:?}", *e))
        .show_ui(ui, |ui| {
            for val in all::<T>() {
                ui.selectable_value(e, val, format!("{val:?}"));
            }
        });
}

pub fn enum_option_combo_ui<T>(e: &mut Option<T>, label: impl Into<WidgetText>, ui: &mut egui::Ui)
where
    T: Sequence + Copy + fmt::Debug + Eq,
{
    const NONE_STR: &str = "None";

    let selected = e
        .map(|s| format!("{s:?}"))
        .unwrap_or_else(|| NONE_STR.to_owned());

    egui::ComboBox::from_label(label)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(e, None, NONE_STR);

            for val in all::<T>() {
                ui.selectable_value(e, Some(val), format!("{val:?}"));
            }
        });
}
