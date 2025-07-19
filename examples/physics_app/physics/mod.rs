use glam;
use lavapond::coord_sys::WorldPos2D;
use lavapond::shapes::{DrawParams, Shape};
use std::time::Instant;
use utils::color::Color;

pub struct PhysicsSystem {
    pub models: Vec<Model>,
    pub bounding_box: BoundingBox,
    time: Instant,
    simulation_state: SimulationState,
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self {
            models: vec![],
            bounding_box: BoundingBox::new(WorldPos2D::new(0.0, 0.0), 0.0, 1.0, 1.0),
            time: Instant::now(),
            simulation_state: SimulationState::Pause,
        }
    }
}

impl PhysicsSystem {
    /// Change the bounding box properties
    pub fn bounding_box(&mut self, center: WorldPos2D, rotation: f32, size_a: f32, size_b: f32) {
        self.bounding_box.center = center;
        self.bounding_box.rotation = rotation;
        self.bounding_box.size_a = size_a;
        self.bounding_box.size_b = size_b;
    }

    /// Adds a `Circle` model to [`PhysicsSystem`]
    pub fn add_circle(
        &mut self,
        radius: f32,
        position: WorldPos2D,
        velocity: glam::Vec2,
        color: Color,
    ) -> () {
        self.models.push(Model {
            position,
            rotation: 0.0,
            velocity,
            // acceleration: glam::vec2(0.0, 0.0),
            model_type: ModelType::Circle(radius),
            color,
        });
    }

    /// Adds a `Circle` model to [`PhysicsSystem`]
    ///
    /// Color and velocity randomised
    pub fn add_circle2(&mut self, radius: f32, position: WorldPos2D) -> () {
        self.models.push(Model {
            position,
            rotation: 0.0,
            velocity: glam::vec2(rand::random_range(-0.5..0.5), rand::random_range(-0.5..0.5)),
            model_type: ModelType::Circle(radius),
            color: Color::random(),
        });
    }

    /// Adds a `Circle` model to [`PhysicsSystem`]
    ///
    /// Values randomised based on input ranges
    pub fn add_circle3(
        &mut self,
        radius: std::ops::Range<f32>,
        position_x: std::ops::Range<f32>,
        position_y: std::ops::Range<f32>,
        velocity: std::ops::Range<f32>,
    ) -> () {
        self.models.push(Model {
            position: WorldPos2D::new(
                rand::random_range(position_x),
                rand::random_range(position_y),
            ),
            rotation: 0.0,
            velocity: glam::vec2(
                rand::random_range(velocity.clone()),
                rand::random_range(velocity),
            ),
            model_type: ModelType::Circle(rand::random_range(radius)),
            color: Color::random(),
        });
    }

    /// Adds a `Rectangle` model to [`PhysicsSystem`]
    pub fn rectangle(
        &mut self,
        length_a: f32,
        length_b: f32,
        rotation: f32,
        position: WorldPos2D,
        velocity: glam::Vec2,
        color: Color,
    ) -> () {
        self.models.push(Model {
            position,
            rotation,
            velocity,
            // acceleration: glam::vec2(0.0, 0.0),
            model_type: ModelType::Rectangle(length_a, length_b),
            color,
        });
    }

    /// Updates the models in the [`PhysicsSystem`] based on the elapsed time
    pub fn update(&mut self) -> () {
        // if self.simulation_state == SimulationState::Pause {
        //     self.time = Instant::now();
        //     return;
        // }

        // for model in self.models.as_mut_slice() {
        //     // Skip if its an rectangle
        //     if matches!(model.model_type, ModelType::Rectangle(..)) {
        //         continue;
        //     }

        //     // New position
        //     let x_pos = model.position.x + model.velocity.x * self.time.elapsed().as_secs_f32();
        //     let y_pos = model.position.y + model.velocity.y * self.time.elapsed().as_secs_f32();

        //     // Check area limits and invert velocity
        //     if x_pos - model.x_range() <= self.bounding_box.left() {
        //         model.position.y = self.bounding_box.left() + model.x_range();
        //         model.velocity.x *= -0.8;
        //     } else if x_pos + model.x_range() >= self.bounding_box.right() {
        //         model.position.y = self.bounding_box.right() - model.x_range();
        //         model.velocity.x *= -0.8;
        //     } else {
        //         model.position.x += model.velocity.x * self.time.elapsed().as_secs_f32();
        //     }

        //     if y_pos - model.y_range() <= self.bounding_box.top() {
        //         model.position.y = self.bounding_box.top() + model.y_range();
        //         model.velocity.y *= -0.8;
        //     } else if y_pos + model.y_range() >= self.bounding_box.bottom() {
        //         model.position.y = self.bounding_box.bottom() - model.y_range();
        //         model.velocity.y *= -0.8;
        //     } else {
        //         model.position.y += model.velocity.y * self.time.elapsed().as_secs_f32();
        //     }
        // }

        // self.time = Instant::now();
    }

    /// Switches the [`SimulationState`] to `Run`
    pub fn set_run(&mut self) -> () {
        self.simulation_state = SimulationState::Run;
        dbg!("State Run");
    }

    /// Switches the [`SimulationState`] to `Paused`
    pub fn set_pause(&mut self) -> () {
        self.simulation_state = SimulationState::Pause;
        dbg!("State Pause");
    }

    /// Switches between `Paused` and `Run` [`SimulationState`]s
    pub fn switch_state(&mut self) -> () {
        match self.simulation_state {
            SimulationState::Run => self.set_pause(),
            SimulationState::Pause => self.set_run(),
        }
    }
}

#[derive(PartialEq)]
pub enum SimulationState {
    Run,
    Pause,
}

//==================================================
//=== Model
//==================================================

pub struct Model {
    position: WorldPos2D,
    rotation: f32,
    velocity: glam::Vec2,
    // acceleration: glam::Vec2,
    model_type: ModelType,
    color: Color,
}

impl Shape for Model {
    fn get_drawparams(&self) -> DrawParams {
        match self.model_type {
            ModelType::Circle(radius) => {
                DrawParams::circle(radius, self.rotation, self.position, self.color)
            }
            ModelType::Rectangle(length_a, length_b) => {
                DrawParams::rectangle(length_a, length_b, self.rotation, self.position, self.color)
            }
        }
    }
}

type Radius = f32;
type LengthA = f32;
type LengthB = f32;

pub enum ModelType {
    Circle(Radius),
    Rectangle(LengthA, LengthB),
}

// impl Model {
// pub fn x_range(&self) -> f32 {
//     match self.model_type {
//         ModelType::Circle(r, ..) => r * 0.1,
//         ModelType::Rectangle(x, ..) => x * 0.5 * 0.1,
//     }
// }

// pub fn y_range(&self) -> f32 {
//     match self.model_type {
//         ModelType::Circle(r, ..) => r * 0.1,
//         ModelType::Rectangle(_, y, _) => y / 2.0,
//     }
// }
// }

// Simulation Area
pub struct BoundingBox {
    pub center: WorldPos2D,
    pub rotation: f32,
    pub size_a: f32,
    pub size_b: f32,
}

impl BoundingBox {
    pub fn new(center: WorldPos2D, rotation: f32, size_a: f32, size_b: f32) -> Self {
        Self {
            center,
            rotation,
            size_a,
            size_b,
        }
    }

    fn left(&self) -> f32 {
        -self.size_a * 0.1 + self.center.x
    }

    fn right(&self) -> f32 {
        self.size_a * 0.1 + self.center.x
    }

    fn top(&self) -> f32 {
        -self.size_b * 0.1 + self.center.x
    }

    fn bottom(&self) -> f32 {
        self.size_b * 0.1 + self.center.x
    }
}

impl Shape for BoundingBox {
    fn get_drawparams(&self) -> DrawParams {
        DrawParams::rectangle(
            self.size_a,
            self.size_b,
            self.rotation,
            self.center,
            Color::ONYX,
        )
    }
}
