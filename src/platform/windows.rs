use super::{NativeBackend, Point, Rect};

const ES_CONTINUOUS: u32 = 0x8000_0000;
const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;

const INPUT_MOUSE: u32 = 0;
const MOUSEEVENTF_MOVE: u32 = 0x0001;
const MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;
const MOUSEEVENTF_VIRTUALDESK: u32 = 0x4000;

const SM_XVIRTUALSCREEN: i32 = 76;
const SM_YVIRTUALSCREEN: i32 = 77;
const SM_CXVIRTUALSCREEN: i32 = 78;
const SM_CYVIRTUALSCREEN: i32 = 79;

#[repr(C)]
#[derive(Default)]
struct PointRaw {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
union InputUnion {
    mi: MouseInput,
}

#[repr(C)]
struct Input {
    input_type: u32,
    input: InputUnion,
}

#[link(name = "user32")]
#[link(name = "kernel32")]
extern "system" {
    fn GetCursorPos(point: *mut PointRaw) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn SendInput(input_count: u32, inputs: *const Input, size: i32) -> u32;
    fn SetThreadExecutionState(flags: u32) -> u32;
}

pub struct PlatformBackend;

impl NativeBackend for PlatformBackend {
    fn new() -> Result<Self, String> {
        Ok(Self)
    }

    fn backend_name(&self) -> &'static str {
        "Windows / SendInput + SetThreadExecutionState"
    }

    fn set_keep_awake(&mut self, enabled: bool) -> Result<(), String> {
        let flags = if enabled {
            ES_CONTINUOUS | ES_DISPLAY_REQUIRED
        } else {
            ES_CONTINUOUS
        };

        let result = unsafe { SetThreadExecutionState(flags) };
        if result == 0 {
            Err("调用 Windows 防休眠 API 失败。".to_owned())
        } else {
            Ok(())
        }
    }

    fn cursor_position(&self) -> Result<Point, String> {
        let mut point = PointRaw::default();
        let success = unsafe { GetCursorPos(&mut point) };
        if success == 0 {
            Err("读取 Windows 鼠标位置失败。".to_owned())
        } else {
            Ok(Point::new(point.x as f64, point.y as f64))
        }
    }

    fn screen_bounds(&self) -> Result<Rect, String> {
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) } as f64;
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) } as f64;
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } as f64;
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } as f64;
        Ok(Rect::new(x, y, width, height))
    }

    fn move_cursor(&self, point: Point) -> Result<(), String> {
        let bounds = self.screen_bounds()?;
        let width = bounds.width.max(1.0);
        let height = bounds.height.max(1.0);

        let normalized_x =
            (((point.x - bounds.min_x) * 65535.0) / (width - 1.0).max(1.0)).round() as i32;
        let normalized_y =
            (((point.y - bounds.min_y) * 65535.0) / (height - 1.0).max(1.0)).round() as i32;

        let input = Input {
            input_type: INPUT_MOUSE,
            input: InputUnion {
                mi: MouseInput {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouse_data: 0,
                    dw_flags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        };

        let sent = unsafe { SendInput(1, &input, std::mem::size_of::<Input>() as i32) };
        if sent == 0 {
            Err("发送 Windows 鼠标事件失败。".to_owned())
        } else {
            Ok(())
        }
    }
}
