use crate::coord_sys::*;
use utils::color::Color;

pub trait Shape {
    fn get_drawparams(&self) -> DrawParams;
}

#[derive(Debug, Clone)]
pub enum ShapeType {
    Circle,
    CircleBorder,
    Rectangle,
    RectangleBorder,
    RoundedRectangle,
    RoundedRectangleBorder,
}

pub struct DrawParams {
    size_x: f32,
    size_y: f32,
    rotation: f32,
    center: WorldPos2D,
    shape_type: ShapeType,
    color: Color,
}

impl DrawParams {
    pub fn size_x(&self) -> f32 {
        self.size_x
    }

    pub fn size_y(&self) -> f32 {
        self.size_y
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn center(&self) -> WorldPos2D {
        self.center
    }

    pub fn shape_type(&self) -> &ShapeType {
        &self.shape_type
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub const fn circle(radius: f32, rotation: f32, center: WorldPos2D, color: Color) -> Self {
        Self {
            size_x: radius,
            size_y: radius,
            rotation,
            center,
            shape_type: ShapeType::Circle,
            color,
        }
    }

    pub const fn ellipse(
        radius_x: f32,
        radius_y: f32,
        rotation: f32,
        center: WorldPos2D,
        color: Color,
    ) -> Self {
        Self {
            size_x: radius_x,
            size_y: radius_y,
            rotation,
            center,
            shape_type: ShapeType::Circle,
            color,
        }
    }

    pub const fn rectangle(
        length_a: f32,
        length_b: f32,
        rotation: f32,
        center: WorldPos2D,
        color: Color,
    ) -> Self {
        Self {
            size_x: length_a,
            size_y: length_b,
            rotation,
            center,
            shape_type: ShapeType::Rectangle,
            color,
        }
    }
}
