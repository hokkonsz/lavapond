// std
use std::time::Instant;

// extern
use glam;
use lavapond::camera::WorldPos2D;

pub struct PhysicsSystem {
    pub models: Vec<Model>,
    instant: Instant,
    simulation_state: SimulationState,
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self {
            models: vec![],
            instant: Instant::now(),
            simulation_state: SimulationState::Pause,
        }
    }
}

impl PhysicsSystem {
    /// Adds a `Circle` model to [`PhysicsSystem`]
    pub fn add_circle(
        &mut self,
        radius: f32,
        position: WorldPos2D,
        velocity: glam::Vec2,
        color: glam::Vec3,
    ) -> () {
        self.models.push(Model {
            position,
            velocity,
            // acceleration: glam::vec2(0.0, 0.0),
            model_type: ModelType::Circle(radius, Color(color)),
        });
    }

    /// Adds a `Circle` model to [`PhysicsSystem`]
    ///
    /// Color and velocity randomised
    pub fn add_circle2(&mut self, radius: f32, position: WorldPos2D) -> () {
        self.models.push(Model {
            position,
            velocity: glam::vec2(rand::random_range(-0.5..0.5), rand::random_range(-0.5..0.5)),
            model_type: ModelType::Circle(radius, Color::random()),
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
            velocity: glam::vec2(
                rand::random_range(velocity.clone()),
                rand::random_range(velocity),
            ),
            model_type: ModelType::Circle(rand::random_range(radius), Color::random()),
        });
    }

    /// Adds a `Arena` model to [`PhysicsSystem`]
    pub fn arena(
        &mut self,
        sides: glam::Vec2,
        position: WorldPos2D,
        velocity: glam::Vec2,
        color: glam::Vec3,
    ) -> () {
        self.models.push(Model {
            position,
            velocity,
            // acceleration: glam::vec2(0.0, 0.0),
            model_type: ModelType::Arena(sides.x, sides.y, Color(color)),
        });
    }

    /// Updates the models in the [`PhysicsSystem`] based on the elapsed time
    pub fn update(&mut self) -> () {
        if self.simulation_state == SimulationState::Pause {
            self.instant = Instant::now();
            return;
        }

        for model in self.models.as_mut_slice() {
            // Skip if its an arena
            if matches!(model.model_type, ModelType::Arena(..)) {
                continue;
            }

            // New position
            model.position += model.velocity * self.instant.elapsed().as_secs_f32();

            // Check area limits and invert velocity
            if model.position.x - model.x_range() <= -1.0 {
                model.position.x = -1.0 + model.x_range();
                model.velocity.x *= -1.0;
            } else if model.position.x + model.x_range() >= 1.0 {
                model.position.x = 1.0 - model.x_range();
                model.velocity.x *= -1.0;
            }

            if model.position.y - model.y_range() <= -1.0 {
                model.position.y = -1.0 + model.y_range();
                model.velocity.y *= -1.0;
            } else if model.position.y + model.y_range() >= 1.0 {
                model.position.y = 1.0 - model.y_range();
                model.velocity.y *= -1.0;
            }
        }

        self.instant = Instant::now();
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

type Radius = f32;
type SideA = f32;
type SideB = f32;

#[derive(Clone, Copy, PartialEq)]
pub struct Color(pub glam::Vec3);

impl Color {
    fn new(r: f32, g: f32, b: f32) -> Self {
        Color(glam::vec3(r, g, b))
    }

    fn random() -> Self {
        Color(glam::vec3(
            rand::random_range(0.0..1.0),
            rand::random_range(0.0..1.0),
            rand::random_range(0.0..1.0),
        ))
    }
}

pub struct Model {
    pub position: lavapond::camera::WorldPos2D,
    pub velocity: glam::Vec2,
    // pub acceleration: glam::Vec2,
    pub model_type: ModelType,
}

impl Model {
    pub fn x_range(&self) -> f32 {
        match self.model_type {
            ModelType::Circle(r, ..) => r * 0.1,
            ModelType::Arena(x, ..) => x * 0.5 * 0.1,
        }
    }

    pub fn y_range(&self) -> f32 {
        match self.model_type {
            ModelType::Circle(r, ..) => r * 0.1,
            ModelType::Arena(_, y, _) => y / 2.0,
        }
    }
}

pub enum ModelType {
    Circle(Radius, Color),
    Arena(SideA, SideB, Color),
}
