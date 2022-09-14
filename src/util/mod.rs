use std::fmt;

pub use average_timer::AverageTimer;
use egui::WidgetText;
use enum_iterator::{all, Sequence};

pub mod average_timer;
pub mod recent_channel;

macro_rules! packed_bools {
    ( ($t:ty) $($b:expr,)* ) => { {
        let mut result: $t = 0;
        let mut _i = 0_u32;

        $(
            if $b { result |= <$t>::checked_shl(1, _i).unwrap(); }
            _i += 1;
        )*

        result
    } };
}

pub(crate) use packed_bools;

pub trait EnumComboUi: Sequence {
    fn enum_combo_ui(&mut self, label: impl Into<WidgetText>, ui: &mut egui::Ui);
}

impl<T> EnumComboUi for T
where
    T: Sequence + Copy + fmt::Debug + Eq,
{
    fn enum_combo_ui(&mut self, label: impl Into<WidgetText>, ui: &mut egui::Ui) {
        egui::ComboBox::from_label(label)
            .selected_text(format!("{:?}", *self))
            .show_ui(ui, |ui| {
                for val in all::<T>() {
                    ui.selectable_value(self, val, format!("{:?}", val));
                }
            });
    }
}
