use eframe::epaint;
use egui::{Color32, Pos2, Rgba, Rounding, Sense, Stroke, Vec2};

use crate::{adapter, calibration::NOTCHES};

use super::shape::ngon_points;

pub struct Stick<'a> {
    stick: adapter::Stick,
    color: Color32,
    points: Option<&'a [[u8; 2]]>,
}

impl<'a> Stick<'a> {
    const SIZE: f32 = 45.0;

    pub fn new(stick: adapter::Stick, color: Color32) -> Self {
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
            painter.add(epaint::RectShape {
                rect,
                rounding: Rounding::none(),
                fill: background_color.into(),
                stroke: Stroke::new(1.0, border_color),
            });

            let polygon_radius = Self::SIZE * 0.5 * 0.8;
            let polygon_points = ngon_points(NOTCHES, polygon_radius)
                .into_iter()
                .map(|p| {
                    let c = rect.center();
                    Pos2::new(p.x + c.x, p.y + c.y)
                })
                .collect();

            // Add stick polygon.
            painter.add(epaint::PathShape {
                points: polygon_points,
                closed: true,
                fill: polygon_color.into(),
                stroke: Stroke::NONE,
            });

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
                painter.add(epaint::RectShape {
                    rect: point_rect,
                    rounding: Rounding::same(5.0),
                    fill: color,
                    stroke: Stroke::NONE,
                });
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
