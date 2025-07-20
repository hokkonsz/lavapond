use crate::coord_sys::{self, *};
use glam;

pub struct Camera {
    position: glam::Vec3,
    view_projection: ViewProjection,
    projection_type: ProjectionType,
    projection_params: ProjectionParams,
}

impl Camera {
    /// Creates a new [`Camera`] based on the current windows size
    pub fn new(
        window: &winit::window::Window,
        position: glam::Vec3,
        projection_type: ProjectionType,
    ) -> Self {
        let projection_params = projection_type.init_params(
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        let view_projection = ViewProjection::new(&position, &projection_type, &projection_params);

        dbg!(view_projection);

        Self {
            position,
            view_projection,
            projection_type,
            projection_params,
        }
    }

    /// Shift the camera position on the X and Y axis
    pub fn shift(&mut self, delta_pos: WorldPos2D) -> () {
        self.position.x += delta_pos.x;
        self.position.y += delta_pos.y;

        self.view_projection.view = glam::Mat4::look_at_rh(
            self.position,                                     // Camera Position
            glam::vec3(self.position.x, self.position.y, 0.0), // Camera Target
            glam::Vec3::Y,
        );
    }

    /// Updates the projection matrix of the camera
    pub fn update_projection(&mut self, window: &winit::window::Window) {
        if self.projection_params.width == window.inner_size().width as f32
            && self.projection_params.height == window.inner_size().height as f32
        {
            return;
        }

        self.projection_params.width = window.inner_size().width as f32;
        self.projection_params.height = window.inner_size().height as f32;

        self.view_projection.projection = match self.projection_type {
            ProjectionType::Orthographic => self.projection_params.orthographic(),
            ProjectionType::Perspective => self.projection_params.perspective(),
        }
    }

    /// Return a reference to the view projection matrix
    pub fn get_view_projection(&self) -> &ViewProjection {
        &self.view_projection
    }

    /// Returns a reference to the camera position
    pub fn get_position(&self) -> &glam::Vec3 {
        &self.position
    }

    /// Returns the stored window width in projection parameters
    pub fn get_width(&self) -> f32 {
        self.projection_params.width
    }

    /// Returns the stored window height in projection parameters
    pub fn get_height(&self) -> f32 {
        self.projection_params.height
    }
}

#[derive(Debug, Clone, Copy)]
struct Projection(glam::Mat4);

impl Projection {
    fn orthographic(
        left_plane: f32,
        right_plane: f32,
        bottom_plane: f32,
        top_plane: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Self {
        let projection = glam::Mat4 {
            x_axis: glam::Vec4::X * 2.0 / (right_plane - left_plane),
            y_axis: glam::Vec4::Y * 2.0 / (top_plane - bottom_plane),
            z_axis: glam::Vec4::Z * 1.0 / (near_plane - far_plane),
            w_axis: glam::vec4(
                -(right_plane + left_plane) / (right_plane - left_plane),
                -(bottom_plane + top_plane) / (bottom_plane - top_plane),
                near_plane / (near_plane - far_plane),
                1.0,
            ),
        };

        #[cfg(feature = "render_dbg")]
        {
            println!(
                "Orthographic projection changed to: (l: {:.2}, r: {:.2}, b: {:.2}, t: {:.2})",
                -left_plane, right_plane, bottom_plane, top_plane
            );

            if projection.y_axis.y >= 0.0 {
                eprintln!(
                    "WARNING: y_axis.y created with non negative value: ({})!",
                    projection.y_axis.y
                );
            }

            if near_plane >= far_plane {
                eprintln!(
                    "WARNING: near is greater than far ({} >= {})!",
                    near_plane, far_plane
                );
            }
        }

        Self(projection)
    }

    fn perspective(aspect: f32, rotation: f32, near: f32, far: f32) -> Self {
        let projection = glam::Mat4::perspective_rh_gl(aspect, rotation, near, far);

        #[cfg(feature = "render_dbg")]
        {
            println!(
                "Perspective projection changed to: (a: {:.2}, r: {:.2}, n: {:.2}, f: {:.2})",
                aspect, rotation, near, far
            );

            if projection.y_axis.y >= 0.0 {
                eprintln!(
                    "WARNING: y_axis.y created with non negative value: ({})!",
                    projection.y_axis.y
                );
            }

            if near >= far {
                eprintln!("WARNING: near is greater than far ({} >= {})!", near, far);
            }
        }

        Self(projection)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ViewProjection {
    view: glam::Mat4,
    projection: Projection,
}

impl ViewProjection {
    /// Creates a new [`ViewProjection`]
    pub fn new(
        position: &glam::Vec3,
        projection_type: &ProjectionType,
        projection_params: &ProjectionParams,
    ) -> Self {
        let view = glam::Mat4::look_at_rh(
            *position,                               // Camera Position
            glam::vec3(position.x, position.y, 0.0), // Camera Target
            glam::Vec3::Y,                           // Up Axis
        );

        let mut projection = match projection_type {
            ProjectionType::Orthographic => projection_params.orthographic(),
            ProjectionType::Perspective => projection_params.perspective(),
        };

        Self { view, projection }
    }
}

#[derive(Debug)]
pub enum ProjectionType {
    Orthographic,
    Perspective,
}

impl ProjectionType {
    fn init_params(&self, width: f32, height: f32) -> ProjectionParams {
        match self {
            ProjectionType::Orthographic => ProjectionParams {
                width,
                height,
                rotation: 60.,
                near: -5.0,
                far: 5.0,
            },
            ProjectionType::Perspective => ProjectionParams {
                width,
                height,
                rotation: 60.,
                near: 0.1,
                far: 10.0,
            },
        }
    }
}

#[derive(Debug)]
pub struct ProjectionParams {
    width: f32,
    height: f32,
    rotation: f32,
    near: f32,
    far: f32,
}

impl ProjectionParams {
    fn orthographic(&self) -> Projection {
        let (right, bottom) = coord_sys::world(self.width, self.height);
        Projection::orthographic(-right, right, bottom, -bottom, self.near, self.far)
    }

    fn perspective(&self) -> Projection {
        Projection::perspective(self.width / self.height, self.rotation, self.near, self.far)
    }
}
