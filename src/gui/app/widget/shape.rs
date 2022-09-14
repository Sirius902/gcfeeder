use egui::Pos2;

use std::f32::consts::{PI, TAU};

pub fn ngon_points(sides: usize, radius: f32) -> Vec<Pos2> {
    let start_angle = PI * 0.5;

    (0..sides)
        .map(|i| i as f32 / sides as f32)
        .map(|t| start_angle + t * TAU)
        .map(|angle| Pos2::new(radius * angle.cos(), -radius * angle.sin()))
        .collect()
}
