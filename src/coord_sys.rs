/// ### Screen coordinate system
///
/// - Origin = Top-left corner
/// - Right-plane = Window width
/// - Bottom-plane = Window height
///
/// ```
/// //   0,0 __________ w,0
/// //      |         |
/// //      | w/2,h/2 |
/// //  0,h |_________| w,h
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq)]
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
    pub const fn new(x: f32, y: f32) -> Self {
        Self(glam::vec2(x, y))
    }

    pub fn from_xy(
        window_size: &winit::dpi::PhysicalSize<u32>,
        world_x: f32,
        world_y: f32,
    ) -> Self {
        Self::convert(window_size, world_x, world_y)
    }

    pub fn form_vec2(
        window_size: &winit::dpi::PhysicalSize<u32>,
        world_position: glam::Vec2,
    ) -> Self {
        Self::convert(window_size, world_position.x, world_position.y)
    }

    pub fn from_world_pos(
        window_size: &winit::dpi::PhysicalSize<u32>,
        world_position: WorldPos2D,
    ) -> Self {
        Self::convert(window_size, world_position.x, world_position.y)
    }

    fn convert(window_size: &winit::dpi::PhysicalSize<u32>, world_x: f32, world_y: f32) -> Self {
        // screen origo is on the top left cornert
        // screen_left = 0, screen_top = 0
        let screen_right = window_size.width as f32;
        let screen_bottom = window_size.height as f32;

        // world origo is on the middle of the screen (NOT ALWAYS: Camera can move!)
        // world left = -world_right, world_bottom = -world_top
        let (world_right, world_top) = world(screen_right, screen_bottom);

        // screen_x = screen_left
        //           + ((world_x - world_left) / (world_right - world_left))
        //           * (screen_right - screen_left)
        let screen_x = ((world_x - -world_right) / (world_right - -world_right)) * screen_right;

        // screen_y = screen_top
        //           + ((world_y - world_top) / (world_bottom - world_top))
        //           * (screen_bottom - screen_top)
        let screen_y = ((world_y - world_top) / (-world_top - world_top)) * screen_bottom;

        Self::new(screen_x, screen_y)
    }
}

/// ### World coordinate system
///
/// Equivalent of an Y flipped Vulkan coordinate system
///
///
/// ```
/// //   -1,1 __________ 1,1
/// //       |         |
/// //       |   0,0   |
/// // -1,-1 |_________| 1,-1
///
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq)]
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
    pub const fn new(x: f32, y: f32) -> Self {
        Self(glam::vec2(x, y))
    }

    pub fn from_xy(
        window_size: &winit::dpi::PhysicalSize<u32>,
        screen_x: f32,
        screen_y: f32,
    ) -> Self {
        Self::convert(window_size, screen_x, screen_y)
    }

    pub fn from_vec2(
        window_size: &winit::dpi::PhysicalSize<u32>,
        screen_position: glam::Vec2,
    ) -> Self {
        Self::convert(window_size, screen_position.x, screen_position.y)
    }

    pub fn from_screen_pos(
        window_size: &winit::dpi::PhysicalSize<u32>,
        screen_position: ScreenPos2D,
    ) -> Self {
        Self::convert(window_size, screen_position.x, screen_position.y)
    }

    fn convert(window_size: &winit::dpi::PhysicalSize<u32>, screen_x: f32, screen_y: f32) -> Self {
        // screen origo is on the top left cornert
        // screen_left = 0, screen_top = 0
        let screen_right = window_size.width as f32;
        let screen_bottom = window_size.height as f32;

        // world origo is on the middle of the screen (NOT ALWAYS: Camera can move!)
        // world left = -world_right, world_bottom = -world_top
        let (world_right, world_top) = world(screen_right, screen_bottom);

        // world_x = world_left
        //           + ((screen_x - screen_left) / (screen_right - screen_left))
        //           * (world_right - world_left)
        let world_x = -world_right + (screen_x / screen_right) * (world_right - -world_right);

        // world_y = world_top
        //           + ((screen_y - screen_top) / (screen_bottom - screen_top))
        //           * (world_bottom - world_top)
        let world_y = world_top + (screen_y / screen_bottom) * (-world_top - world_top);

        Self::new(world_x, world_y)
    }
}

impl std::ops::AddAssign<glam::Vec2> for WorldPos2D {
    fn add_assign(&mut self, rhs: glam::Vec2) {
        self.0 += rhs;
    }
}

type Right = f32;
type Top = f32;
fn world(width: f32, height: f32) -> (Right, Top) {
    if width > height {
        (width / height, 1.0)
    } else {
        (1.0, height / width)
    }
}

#[test]
fn test_world_pos_2d() {
    use crate::WorldPos2D;
    use winit::dpi::PhysicalSize;

    let width: f32 = 800.;
    let height: f32 = 600.;
    let world_right = width / height;
    let window_size: PhysicalSize<u32> = PhysicalSize::new(width as u32, height as u32);

    // Top left corner
    let world_pos = WorldPos2D::from_xy(&window_size, 0., 0.);
    assert_eq!(world_pos.x, -world_right);
    assert_eq!(world_pos.y, -1.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 0.);
    assert_eq!(screen_pos.y, 0.);

    // Top side middle
    let world_pos = WorldPos2D::from_xy(&window_size, 400., 0.);
    assert_eq!(world_pos.x, 0.);
    assert_eq!(world_pos.y, -1.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 400.);
    assert_eq!(screen_pos.y, 0.);

    // Top right corner
    let world_pos = WorldPos2D::from_xy(&window_size, 800., 0.);
    assert_eq!(world_pos.x, world_right);
    assert_eq!(world_pos.y, -1.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 800.);
    assert_eq!(screen_pos.y, 0.);

    // Left side middle
    let world_pos = WorldPos2D::from_xy(&window_size, 0., 300.);
    assert_eq!(world_pos.x, -world_right);
    assert_eq!(world_pos.y, 0.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 0.);
    assert_eq!(screen_pos.y, 300.);

    // Center
    let world_pos = WorldPos2D::from_xy(&window_size, 400., 300.);
    assert_eq!(world_pos.x, 0.);
    assert_eq!(world_pos.y, 0.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 400.);
    assert_eq!(screen_pos.y, 300.);

    // Right side middle
    let world_pos = WorldPos2D::from_xy(&window_size, 800., 300.);
    assert_eq!(world_pos.x, world_right);
    assert_eq!(world_pos.y, 0.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 800.);
    assert_eq!(screen_pos.y, 300.);

    // Bottom left corner
    let world_pos = WorldPos2D::from_xy(&window_size, 0., 600.);
    assert_eq!(world_pos.x, -world_right);
    assert_eq!(world_pos.y, 1.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 0.);
    assert_eq!(screen_pos.y, 600.);

    // Bottom side middle
    let world_pos = WorldPos2D::from_xy(&window_size, 400., 600.);
    assert_eq!(world_pos.x, 0.);
    assert_eq!(world_pos.y, 1.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 400.);
    assert_eq!(screen_pos.y, 600.);

    // Bottom right corner
    let world_pos = WorldPos2D::from_xy(&window_size, 800., 600.);
    assert_eq!(world_pos.x, world_right);
    assert_eq!(world_pos.y, 1.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert_eq!(screen_pos.x, 800.);
    assert_eq!(screen_pos.y, 600.);

    // Negative Point
    let world_pos = WorldPos2D::from_xy(&window_size, -800., -300.);
    assert_eq!(world_pos.x, -3. * world_right);
    assert_eq!(world_pos.y, -2.);
    let screen_pos = ScreenPos2D::from_world_pos(&window_size, world_pos);
    assert!(-800. - screen_pos.x >= -0.00007 && -800. - screen_pos.x < 0.0);
    assert_eq!(screen_pos.y, -300.);
}
