use std::fmt;

use egui::{Align, Layout, Order, WidgetText};
use enum_iterator::{all, Sequence};

pub fn enum_combo_ui<T>(e: &mut T, label: impl Into<WidgetText>, ui: &mut egui::Ui)
where
    T: Sequence + Copy + fmt::Debug + Eq,
{
    egui::ComboBox::from_label(label)
        .selected_text(format!("{:?}", *e))
        .show_ui(ui, |ui| {
            for val in all::<T>() {
                ui.selectable_value(e, val, format!("{:?}", val));
            }
        });
}

pub fn enum_option_combo_ui<T>(e: &mut Option<T>, label: impl Into<WidgetText>, ui: &mut egui::Ui)
where
    T: Sequence + Copy + fmt::Debug + Eq,
{
    const NONE_STR: &str = "None";

    let selected = e
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| NONE_STR.to_owned());

    egui::ComboBox::from_label(label)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(e, None, NONE_STR);

            for val in all::<T>() {
                ui.selectable_value(e, Some(val), format!("{:?}", val));
            }
        });
}

/// Implementation based on `egui::popup_below_widget`.
pub fn no_close_popup_below_widget<R>(
    ui: &egui::Ui,
    popup_id: egui::Id,
    widget_response: &egui::Response,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    if ui.memory().is_popup_open(popup_id) {
        let response = egui::Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(widget_response.rect.left_bottom())
            .show(ui.ctx(), |ui| {
                // Note: we use a separate clip-rect for this area, so the popup can be outside the parent.
                // See https://github.com/emilk/egui/issues/825
                let frame = egui::Frame::popup(ui.style());
                let frame_margin = frame.inner_margin + frame.outer_margin;
                frame
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.set_width(widget_response.rect.width() - frame_margin.sum().x);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
            });

        if response.response.clicked_elsewhere() && !widget_response.clicked() {
            ui.memory().close_popup();
        }

        Some(response.inner)
    } else {
        None
    }
}
