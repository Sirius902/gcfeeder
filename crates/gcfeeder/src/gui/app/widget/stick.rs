use eframe::epaint;
use egui::epaint::PathStroke;
use egui::{Color32, CornerRadius, Pos2, Rgba, Sense, Stroke, StrokeKind, Vec2};
use gcfeeder_core::calibration::NOTCHES;

use super::shape::ngon_points;

pub struct Stick<'a> {
    stick: gcinput::Stick,
    color: Color32,
    points: Option<&'a [[u8; 2]]>,
}

impl<'a> Stick<'a> {
    const SIZE: f32 = 45.0;

    pub fn new(stick: gcinput::Stick, color: Color32) -> Self {
        Self {
            stick,
            color,
            points: None,
        }
    }

    pub fn with_points(mut self, points: &'a [[u8; 2]]) -> Self {
        self.points = Some(points);
        self
    }
}

impl egui::Widget for Stick<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(Self::SIZE, Self::SIZE),
            Sense::focusable_noninteractive(),
        );

        if ui.is_rect_visible(response.rect) {
            let painter = ui.painter();
            let color = Rgba::from(self.color);
            let background_color =
                Rgba::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a() * 0.03);
            let polygon_color =
                Rgba::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a() * 0.35);
            let calibration_color = Rgba::from_rgba_unmultiplied(
                1.0 - color.r(),
                1.0 - color.g(),
                1.0 - color.b(),
                color.a(),
            );
            let border_color = polygon_color;

            // Add background rect.
            painter.rect_filled(rect, CornerRadius::ZERO, background_color);
            painter.rect_stroke(
                rect,
                CornerRadius::ZERO,
                Stroke::new(1.0, border_color),
                StrokeKind::Inside,
            );

            let polygon_radius = Self::SIZE * 0.5 * 0.8;
            let polygon_points = ngon_points(NOTCHES, polygon_radius)
                .into_iter()
                .map(|p| {
                    let c = rect.center();
                    Pos2::new(p.x + c.x, p.y + c.y)
                })
                .collect();

            // Add stick polygon.
            painter.add(egui::Shape::Path(epaint::PathShape {
                points: polygon_points,
                closed: true,
                fill: polygon_color.into(),
                stroke: PathStroke::NONE,
            }));

            let draw_point = |p: [u8; 2], color: Color32| {
                let scale_stick_coord =
                    |n: u8| polygon_radius * (2.0 * n as f32 / f32::from(u8::MAX) - 1.0);

                let half_size = Self::SIZE / 18.0;
                let stick_pos = Vec2::new(scale_stick_coord(p[0]), -scale_stick_coord(p[1]));
                let point_rect = epaint::Rect::from_two_pos(
                    rect.center() - Vec2::new(half_size, half_size) + stick_pos,
                    rect.center() + Vec2::new(half_size, half_size) + stick_pos,
                );

                // Add stick position.
                painter.rect_filled(point_rect, CornerRadius::same(5), color);
            };

            if let Some(points) = self.points {
                for point in points.iter() {
                    draw_point(*point, calibration_color.into());
                }
            }

            draw_point(self.stick.into(), self.color);
        }

        response
    }
}
