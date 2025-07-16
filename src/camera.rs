use std::ops::{Add, AddAssign};

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

        Self {
            position,
            view_projection,
            projection_type,
            projection_params,
        }
    }

    /// Shift the camera position on the X and Y axis
    pub fn shift(&mut self, delta_x: f32, delta_y: f32) -> () {
        self.position = glam::vec3(
            self.position.x + delta_x,
            self.position.y - delta_y,
            self.position.z,
        );

        self.view_projection.view = glam::Mat4::look_at_rh(
            self.position,                                     // Camera Position
            glam::vec3(self.position.x, self.position.y, 0.0), // Camera Target
            glam::vec3(0.0, 1.0, 0.0),
        );
    }

    /// Updates the projection matrix of the camera
    ///
    /// If the camera is fix then we do not need to call this function
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
        if projection.y_axis.y >= 0.0 {
            eprintln!(
                "WARNING: y_axis.y created with non negative value: ({})!",
                projection.y_axis.y
            );
        }

        #[cfg(feature = "render_dbg")]
        if near_plane >= far_plane {
            eprintln!(
                "WARNING: near is smaller than far ({} >= {})!",
                near_plane, far_plane
            );
        }

        Self(projection)
    }

    fn perspective(aspect: f32, rotation: f32, near: f32, far: f32) -> Self {
        let projection = glam::Mat4::perspective_rh_gl(aspect, rotation, near, far);

        #[cfg(feature = "render_dbg")]
        if projection.y_axis.y >= 0.0 {
            eprintln!(
                "WARNING: y_axis.y created with non negative value: ({})!",
                projection.y_axis.y
            );
        }

        #[cfg(feature = "render_dbg")]
        if near >= far {
            eprintln!("WARNING: near is smaller than far ({} >= {})!", near, far);
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
        if self.width > self.height {
            Projection::orthographic(
                -1.,
                1.,
                self.height / self.width,
                -self.height / self.width,
                self.near,
                self.far,
            )
        } else {
            Projection::orthographic(
                -self.width / self.height,
                self.width / self.height,
                1.,
                -1.,
                self.near,
                self.far,
            )
        }
    }

    fn perspective(&self) -> Projection {
        Projection::perspective(self.width / self.height, self.rotation, self.near, self.far)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScreenPos2D(glam::Vec2);

impl std::ops::Deref for ScreenPos2D {
    type Target = glam::Vec2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ScreenPos2D {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ScreenPos2D {
    pub fn new(x: f32, y: f32) -> Self {
        Self(glam::vec2(x, y))
    }

    pub fn from_world(window_size: &winit::dpi::PhysicalSize<u32>, x: f32, y: f32) -> Self {
        let half_width = window_size.width as f32 * 0.5;
        let half_height = window_size.height as f32 * 0.5;
        let x = (x * half_width) + half_width;
        let y = (y * half_height) + half_height;

        Self::new(x, y)
    }

    pub fn from_world2(window_size: &winit::dpi::PhysicalSize<u32>, position: glam::Vec2) -> Self {
        let half_width = window_size.width as f32 * 0.5;
        let half_height = window_size.height as f32 * 0.5;
        let x = (position.x * half_width) + half_width;
        let y = (position.y * half_height) + half_height;

        Self::new(x, y)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WorldPos2D(glam::Vec2);

impl std::ops::Deref for WorldPos2D {
    type Target = glam::Vec2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for WorldPos2D {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl WorldPos2D {
    pub fn new(x: f32, y: f32) -> Self {
        Self(glam::vec2(x, y))
    }

    pub fn from_screen(window_size: &winit::dpi::PhysicalSize<u32>, x: f32, y: f32) -> Self {
        let half_width = window_size.width as f32 * 0.5;
        let half_height = window_size.height as f32 * 0.5;
        let x = (x - half_width) / half_width;
        let y = (y - half_height) / half_height;

        Self::new(x, y)
    }

    pub fn from_screen2(window_size: &winit::dpi::PhysicalSize<u32>, position: glam::Vec2) -> Self {
        let half_width = window_size.width as f32 * 0.5;
        let half_height = window_size.height as f32 * 0.5;
        let x = (position.x - half_width) / half_width;
        let y = (position.y - half_height) / half_height;

        Self::new(x, y)
    }
}

impl AddAssign<glam::Vec2> for WorldPos2D {
    fn add_assign(&mut self, rhs: glam::Vec2) {
        self.0 += rhs;
    }
}
