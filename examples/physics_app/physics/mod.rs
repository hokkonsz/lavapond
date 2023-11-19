// std
use std::time::Instant;

// extern
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
            simulation_state: SimulationState::Paused,
        }
    }

    /// Adds a `Circle` model to [`PhysicsSystem`]
    pub fn circle(
        &mut self,
        radius: f32,
        position: glm::Vec2,
        velocity: glm::Vec2,
        color: glm::Vec3,
    ) -> () {
        self.models.push(Model {
            position,
            velocity,
            acceleration: glm::vec2(0.0, 0.0),
            model_type: ModelType::Circle(radius, color),
        });
    }

    /// Adds a `Arena` model to [`PhysicsSystem`]
    pub fn arena(
        &mut self,
        sides: glm::Vec2,
        position: glm::Vec2,
        velocity: glm::Vec2,
        color: glm::Vec3,
    ) -> () {
        self.models.push(Model {
            position,
            velocity,
            acceleration: glm::vec2(0.0, 0.0),
            model_type: ModelType::Arena(sides.x, sides.y, color),
        });
    }

    /// Updates the models in the [`PhysicsSystem`] based on the elapsed time
    pub fn update(&mut self) -> () {
        if self.simulation_state == SimulationState::Paused {
            self.instant = Instant::now();
            return;
        }

        for model in self.models.as_mut_slice() {
            // X Axis
            if (model.position.x - model.x_range() <= -1.0)
                || model.position.x + model.x_range() >= 1.0
            {
                model.velocity.x *= -1.0;
            }

            // Y Axis
            if (model.position.y - model.y_range() <= -1.0)
                || model.position.y + model.y_range() >= 1.0
            {
                model.velocity.y *= -1.0;
            }

            model.position += model.velocity * self.instant.elapsed().as_secs_f32();
        }

        self.instant = Instant::now();
    }

    /// Switches the [`SimulationState`] to `Run`
    pub fn set_run(&mut self) -> () {
        self.simulation_state = SimulationState::Run;
    }

    /// Switches the [`SimulationState`] to `Paused`
    pub fn set_pause(&mut self) -> () {
        self.simulation_state = SimulationState::Run;
    }

    /// Switches between `Paused` and `Run` [`SimulationState`]s
    pub fn switch_state(&mut self) -> () {
        match self.simulation_state {
            SimulationState::Run => self.simulation_state = SimulationState::Paused,
            SimulationState::Paused => self.simulation_state = SimulationState::Run,
        }
    }
}

#[derive(PartialEq)]
pub enum SimulationState {
    Run,
    Paused,
}

//==================================================
//=== Model
//==================================================

use f32 as Radius;
use f32 as X_side;
use f32 as Y_side;
use glm::Vec3 as Color;

pub struct Model {
    pub position: glm::Vec2,
    pub velocity: glm::Vec2,
    pub acceleration: glm::Vec2,
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
    Arena(X_side, Y_side, Color),
}
