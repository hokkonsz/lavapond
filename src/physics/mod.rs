extern crate nalgebra_glm as glm;

pub struct PhysicsSystem {
    pub models: Vec<Model>,
    instant: Instant,
    simulation_state: SimulationState,
}

impl PhysicsSystem {
    /// Creates a new [`PhysicsSystem`]
    pub fn new() -> Self {
        Self {
            models: vec![],
            instant: Instant::now(),
            simulation_state: SimulationState::Run,
        }
    }

    /// Adds a `Circle` model to [`PhysicsSystem`]
    pub fn circle(&mut self, r: f32, pos_x: f32, pos_y: f32, velo_x: f32, velo_y: f32) -> () {
        self.models.push(Model {
            position: glm::vec2(pos_x, pos_y),
            velocity: glm::vec2(velo_x, velo_y),
            acceleration: glm::vec2(0.0, 0.0),
            model_type: ModelType::Circle(r),
        });
    }

    /// Adds a `Rectangle` model to [`PhysicsSystem`]
    pub fn rectangle(
        &mut self,
        a: f32,
        b: f32,
        pos_x: f32,
        pos_y: f32,
        velo_x: f32,
        velo_y: f32,
    ) -> () {
        self.models.push(Model {
            position: glm::vec2(pos_x, pos_y),
            velocity: glm::vec2(velo_x, velo_y),
            acceleration: glm::vec2(0.0, 0.0),
            model_type: ModelType::Rectangle(a, b),
        });
    }

    /// Adds a `Arena` model to [`PhysicsSystem`]
    pub fn arena(
        &mut self,
        a: f32,
        b: f32,
        pos_x: f32,
        pos_y: f32,
        velo_x: f32,
        velo_y: f32,
    ) -> () {
        self.models.push(Model {
            position: glm::vec2(pos_x, pos_y),
            velocity: glm::vec2(velo_x, velo_y),
            acceleration: glm::vec2(0.0, 0.0),
            model_type: ModelType::Arena(a, b),
        });
    }

    /// Updates the models in the [`PhysicsSystem`] based on the elapsed time
    pub fn update(&mut self) -> () {
        if self.simulation_state == SimulationState::Paused {
            return;
        }

        for model in self.models.as_mut_slice() {
            model.position += model.velocity * self.instant.elapsed().as_secs_f32();
        }

        self.instant = Instant::now();
    }

    /// Switches the [`SimulationState`] to `Run`
    pub fn run(&mut self) -> () {
        self.simulation_state = SimulationState::Run;
    }

    /// Switches the [`SimulationState`] to `Paused`
    pub fn pause(&mut self) -> () {
        self.simulation_state = SimulationState::Run;
    }
}

pub struct Model {
    pub position: glm::Vec2,
    pub velocity: glm::Vec2,
    pub acceleration: glm::Vec2,
    pub model_type: ModelType,
}

use std::time::Instant;

use f32 as radius;
use f32 as a_side;
use f32 as b_side;

pub enum ModelType {
    Circle(radius),
    Rectangle(a_side, b_side),
    Arena(a_side, b_side),
}

#[derive(PartialEq)]
pub enum SimulationState {
    Run,
    Paused,
}
