#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::PlatformBackend;
#[cfg(target_os = "macos")]
pub use macos::PlatformBackend;
#[cfg(target_os = "windows")]
pub use windows::PlatformBackend;

pub trait NativeBackend {
    fn new() -> Result<Self, String>
    where
        Self: Sized;

    fn backend_name(&self) -> &'static str;
    fn set_keep_awake(&mut self, enabled: bool) -> Result<(), String>;
    fn cursor_position(&self) -> Result<Point, String>;
    fn screen_bounds(&self) -> Result<Rect, String>;
    fn move_cursor(&self, point: Point) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub const fn offset(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    pub fn distance_to(self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(min_x: f64, min_y: f64, width: f64, height: f64) -> Self {
        Self {
            min_x,
            min_y,
            width,
            height,
        }
    }

    pub fn max_x(self) -> f64 {
        self.min_x + self.width
    }

    pub fn max_y(self) -> f64 {
        self.min_y + self.height
    }
}
