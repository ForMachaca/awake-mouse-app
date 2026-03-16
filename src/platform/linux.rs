use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
use std::ptr;

use super::{NativeBackend, Point, Rect};

#[repr(C)]
struct Display(c_void);

#[link(name = "X11")]
#[link(name = "Xtst")]
extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
    fn XDefaultScreen(display: *mut Display) -> c_int;
    fn XDisplayWidth(display: *mut Display, screen_number: c_int) -> c_int;
    fn XDisplayHeight(display: *mut Display, screen_number: c_int) -> c_int;
    fn XRootWindow(display: *mut Display, screen_number: c_int) -> c_ulong;
    fn XQueryPointer(
        display: *mut Display,
        window: c_ulong,
        root_return: *mut c_ulong,
        child_return: *mut c_ulong,
        root_x_return: *mut c_int,
        root_y_return: *mut c_int,
        win_x_return: *mut c_int,
        win_y_return: *mut c_int,
        mask_return: *mut c_uint,
    ) -> c_int;
    fn XTestFakeMotionEvent(
        display: *mut Display,
        screen_number: c_int,
        x: c_int,
        y: c_int,
        delay: c_ulong,
    ) -> c_int;
    fn XResetScreenSaver(display: *mut Display) -> c_int;
    fn XFlush(display: *mut Display) -> c_int;
}

pub struct PlatformBackend {
    display: *mut Display,
    screen_number: c_int,
    keep_awake: bool,
}

impl NativeBackend for PlatformBackend {
    fn new() -> Result<Self, String> {
        let display = unsafe { XOpenDisplay(ptr::null()) };
        if display.is_null() {
            return Err("无法连接到 X11 显示服务器。Wayland 默认不允许全局模拟鼠标。".to_owned());
        }

        let screen_number = unsafe { XDefaultScreen(display) };
        Ok(Self {
            display,
            screen_number,
            keep_awake: false,
        })
    }

    fn backend_name(&self) -> &'static str {
        "Linux / X11 XTest"
    }

    fn set_keep_awake(&mut self, enabled: bool) -> Result<(), String> {
        self.keep_awake = enabled;
        if enabled {
            unsafe {
                XResetScreenSaver(self.display);
                XFlush(self.display);
            }
        }
        Ok(())
    }

    fn cursor_position(&self) -> Result<Point, String> {
        let root = unsafe { XRootWindow(self.display, self.screen_number) };
        let mut root_return = 0;
        let mut child_return = 0;
        let mut root_x = 0;
        let mut root_y = 0;
        let mut win_x = 0;
        let mut win_y = 0;
        let mut mask = 0;

        let success = unsafe {
            XQueryPointer(
                self.display,
                root,
                &mut root_return,
                &mut child_return,
                &mut root_x,
                &mut root_y,
                &mut win_x,
                &mut win_y,
                &mut mask,
            )
        };

        if success == 0 {
            Err("读取 X11 鼠标位置失败。".to_owned())
        } else {
            Ok(Point::new(root_x as f64, root_y as f64))
        }
    }

    fn screen_bounds(&self) -> Result<Rect, String> {
        let width = unsafe { XDisplayWidth(self.display, self.screen_number) } as f64;
        let height = unsafe { XDisplayHeight(self.display, self.screen_number) } as f64;
        Ok(Rect::new(0.0, 0.0, width, height))
    }

    fn move_cursor(&self, point: Point) -> Result<(), String> {
        let result = unsafe {
            XTestFakeMotionEvent(
                self.display,
                self.screen_number,
                point.x.round() as c_int,
                point.y.round() as c_int,
                0,
            )
        };

        if result == 0 {
            return Err("发送 X11 鼠标事件失败。".to_owned());
        }

        unsafe {
            if self.keep_awake {
                XResetScreenSaver(self.display);
            }
            XFlush(self.display);
        }

        Ok(())
    }
}

impl Drop for PlatformBackend {
    fn drop(&mut self) {
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}
