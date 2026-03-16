use std::ffi::{c_char, c_void, CString};
use std::ptr;

use super::{NativeBackend, Point, Rect};

const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;
const K_CG_EVENT_MOUSE_MOVED: u32 = 5;
const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;
const K_CG_HID_EVENT_TAP: u32 = 0;

type CFAllocatorRef = *const c_void;
type CFStringRef = *const c_void;
type CGEventSourceRef = *const c_void;
type CGEventRef = *mut c_void;
type IOPMAssertionID = u32;
type IOReturn = i32;
type CGDirectDisplayID = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;

    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        assertion_level: u32,
        assertion_name: CFStringRef,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;
    fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> IOReturn;

    fn CGEventCreate(source: CGEventSourceRef) -> CGEventRef;
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventCreateMouseEvent(
        source: CGEventSourceRef,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CGMainDisplayID() -> CGDirectDisplayID;
    fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
}

pub struct PlatformBackend {
    assertion_id: Option<IOPMAssertionID>,
}

impl NativeBackend for PlatformBackend {
    fn new() -> Result<Self, String> {
        Ok(Self { assertion_id: None })
    }

    fn backend_name(&self) -> &'static str {
        "macOS / IOKit + CoreGraphics"
    }

    fn set_keep_awake(&mut self, enabled: bool) -> Result<(), String> {
        if enabled {
            if self.assertion_id.is_some() {
                return Ok(());
            }

            let assertion_type = cf_string("PreventUserIdleDisplaySleep")?;
            let assertion_name = cf_string("awake-mouse")?;
            let mut assertion_id = 0;
            let result = unsafe {
                IOPMAssertionCreateWithName(
                    assertion_type,
                    K_IOPM_ASSERTION_LEVEL_ON,
                    assertion_name,
                    &mut assertion_id,
                )
            };

            unsafe {
                CFRelease(assertion_type);
                CFRelease(assertion_name);
            }

            if result != 0 {
                return Err(format!("创建 macOS 防休眠断言失败，错误码 {result}"));
            }

            self.assertion_id = Some(assertion_id);
            Ok(())
        } else if let Some(assertion_id) = self.assertion_id.take() {
            let result = unsafe { IOPMAssertionRelease(assertion_id) };
            if result != 0 {
                return Err(format!("释放 macOS 防休眠断言失败，错误码 {result}"));
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn cursor_position(&self) -> Result<Point, String> {
        let event = unsafe { CGEventCreate(ptr::null()) };
        if event.is_null() {
            return Err("无法读取当前鼠标位置。".to_owned());
        }

        let location = unsafe { CGEventGetLocation(event) };
        unsafe { CFRelease(event.cast()) };
        Ok(Point::new(location.x, location.y))
    }

    fn screen_bounds(&self) -> Result<Rect, String> {
        let bounds = unsafe { CGDisplayBounds(CGMainDisplayID()) };
        Ok(Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        ))
    }

    fn move_cursor(&self, point: Point) -> Result<(), String> {
        let event = unsafe {
            CGEventCreateMouseEvent(
                ptr::null(),
                K_CG_EVENT_MOUSE_MOVED,
                CGPoint {
                    x: point.x,
                    y: point.y,
                },
                K_CG_MOUSE_BUTTON_LEFT,
            )
        };

        if event.is_null() {
            return Err("创建 macOS 鼠标事件失败。".to_owned());
        }

        unsafe {
            CGEventPost(K_CG_HID_EVENT_TAP, event);
            CFRelease(event.cast());
        }

        Ok(())
    }
}

impl Drop for PlatformBackend {
    fn drop(&mut self) {
        let _ = self.set_keep_awake(false);
    }
}

fn cf_string(value: &str) -> Result<CFStringRef, String> {
    let c_string = CString::new(value).map_err(|_| "CFString 内容包含非法空字符。".to_owned())?;
    let string = unsafe {
        CFStringCreateWithCString(ptr::null(), c_string.as_ptr(), K_CFSTRING_ENCODING_UTF8)
    };
    if string.is_null() {
        Err("创建 CFString 失败。".to_owned())
    } else {
        Ok(string)
    }
}
