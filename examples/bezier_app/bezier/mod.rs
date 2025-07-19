use glam::Vec2;
use utils::color::Color;

pub struct Bezier {
    pub control_points: [Vec2; 4],
    pub control_points_color: [Color; 4],
    pub dragged_point: Option<usize>,
    pub resolution: u8,
}

impl Default for Bezier {
    fn default() -> Self {
        Self {
            control_points: [
                Vec2::new(0., 0.),
                Vec2::new(0.33, 0.33),
                Vec2::new(0.66, 0.66),
                Vec2::new(1., 1.),
            ],
            control_points_color: [Color::YELLOW, Color::YELLOW, Color::BLUE, Color::BLUE],
            dragged_point: None,
            resolution: 50,
        }
    }
}

impl Bezier {
    pub const CONTROL_POINT_SIZE: f32 = 0.02;
}
