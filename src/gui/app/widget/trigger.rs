use eframe::epaint;
use egui::{Align, Color32, FontSelection, Rgba, Rounding, Sense, Stroke, Vec2, WidgetText};

pub struct Trigger<'a> {
    value: u8,
    signifier: &'a str,
    color: Color32,
    markers: Option<&'a [u8]>,
}

impl<'a> Trigger<'a> {
    const SIZE: Vec2 = Vec2::new(10.0, 45.0);
    const MARKER_HEIGHT: f32 = 2.5;

    pub fn new(value: u8, signifier: &'a str, color: Color32) -> Self {
        Self {
            value,
            signifier,
            color,
            markers: None,
        }
    }

    pub fn with_markers(mut self, markers: &'a [u8]) -> Self {
        self.markers = Some(markers);
        self
    }
}

impl egui::Widget for Trigger<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(Self::SIZE, Sense::focusable_noninteractive());

        if ui.is_rect_visible(response.rect) {
            let painter = ui.painter();
            let color = Rgba::from(self.color);
            let background_color =
                Rgba::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a() * 0.05);
            let calibration_color = Rgba::from_rgba_unmultiplied(
                1.0 - color.r(),
                1.0 - color.g(),
                1.0 - color.b(),
                color.a(),
            );
            let signifier_color = Rgba::from_rgba_unmultiplied(
                (color.r() + calibration_color.r()) * 0.5,
                (color.g() + calibration_color.g()) * 0.5,
                (color.b() + calibration_color.b()) * 0.5,
                color.a(),
            );
            let border_color =
                Rgba::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a() * 0.35);

            // Add background rect.
            painter.add(epaint::RectShape {
                rect,
                rounding: Rounding::none(),
                fill: background_color.into(),
                stroke: Stroke::new(1.0, border_color),
            });

            let scale_trigger = |n: u8| n as f32 / f32::from(u8::MAX) * Self::SIZE.y;
            let fill_top_right = rect.right_bottom() + Vec2::new(0.0, -scale_trigger(self.value));

            // Add fill value.
            painter.add(epaint::RectShape {
                rect: epaint::Rect::from_two_pos(rect.left_bottom(), fill_top_right),
                rounding: Rounding::none(),
                fill: self.color,
                stroke: Stroke::none(),
            });

            let text_job = WidgetText::from(self.signifier).into_text_job(
                ui.style(),
                FontSelection::Default,
                Align::Center,
            );
            let text_galley = text_job.into_galley(&*ui.fonts());

            // Add signifier.
            painter.add(epaint::TextShape {
                pos: rect.center() - text_galley.size() * 0.5,
                galley: text_galley.galley,
                override_text_color: Some(signifier_color.into()),
                underline: Stroke::none(),
                angle: 0.0,
            });

            let draw_market = |val: u8, color: Color32| {
                let offset = Vec2::new(0.0, -scale_trigger(val));
                let marker_rect = epaint::Rect::from_two_pos(
                    rect.left_bottom() + offset + Vec2::new(0.0, 0.5 * Self::MARKER_HEIGHT),
                    rect.right_bottom() + offset - Vec2::new(0.0, 0.5 * Self::MARKER_HEIGHT),
                );

                painter.add(epaint::RectShape {
                    rect: marker_rect,
                    rounding: Rounding::none(),
                    fill: color,
                    stroke: Stroke::none(),
                });
            };

            if let Some(markers) = self.markers {
                for marker in markers.iter() {
                    draw_market(*marker, calibration_color.into());
                }
            }
        }

        response
    }
}
