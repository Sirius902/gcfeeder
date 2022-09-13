use eframe::epaint;
use egui::{Color32, Rgba, Rounding, Sense, Stroke, Vec2};

use crate::adapter;

pub struct Stick {
    stick: adapter::Stick,
    color: Color32,
}

impl Stick {
    const SIZE: f32 = 45.0;

    pub fn new(stick: adapter::Stick, color: Color32) -> Self {
        Self { stick, color }
    }
}

impl egui::Widget for Stick {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(Self::SIZE, Self::SIZE),
            Sense::focusable_noninteractive(),
        );

        if ui.is_rect_visible(response.rect) {
            let painter = ui.painter();
            let color = Rgba::from(self.color);
            let background_color = Color32::from(Rgba::from_rgba_unmultiplied(
                color.r(),
                color.g(),
                color.b(),
                color.a() * 0.05,
            ));
            let polygon_color = Color32::from(Rgba::from_rgba_unmultiplied(
                color.r(),
                color.g(),
                color.b(),
                color.a() * 0.35,
            ));
            let polygon_radius = Self::SIZE * 0.5 * 0.8;
            let border_color = polygon_color;

            // Add background rect.
            painter.add(epaint::RectShape {
                rect,
                rounding: Rounding::none(),
                fill: background_color,
                stroke: Stroke::new(1.0, border_color),
            });

            // Add stick polygon.
            painter.add(epaint::CircleShape {
                center: rect.center(),
                radius: polygon_radius,
                fill: polygon_color,
                stroke: Stroke::none(),
            });

            let scale_stick_coord =
                |n: u8| polygon_radius * (2.0 * n as f32 / f32::from(u8::MAX) - 1.0);

            let half_size = Self::SIZE / 18.0;
            let stick_pos = Vec2::new(
                scale_stick_coord(self.stick.x),
                -scale_stick_coord(self.stick.y),
            );
            let point_rect = epaint::Rect::from_two_pos(
                rect.center() - Vec2::new(half_size, half_size) + stick_pos,
                rect.center() + Vec2::new(half_size, half_size) + stick_pos,
            );

            // Add stick position.
            painter.add(epaint::RectShape {
                rect: point_rect,
                rounding: Rounding::same(5.0),
                fill: self.color,
                stroke: Stroke::none(),
            });
        }

        response
    }
}
