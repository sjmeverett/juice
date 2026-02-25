use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};
use std::os::unix::io::AsRawFd;

pub struct TouchState {
    pub x: i32,
    pub y: i32,
    pub pressed: bool,
}

impl Default for TouchState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            pressed: false,
        }
    }
}

/// Find the first touch device in /dev/input.
pub fn find_touch_device() -> Option<Device> {
    println!("Scanning for input devices...");

    for entry in std::fs::read_dir("/dev/input").ok()?.flatten() {
        let path = entry.path();

        if let Ok(device) = Device::open(&path) {
            let name = device.name().unwrap_or("Unknown");
            println!("  Device: {} at {:?}", name, path);

            if device.supported_absolute_axes().map_or(false, |axes| {
                (axes.contains(AbsoluteAxisCode::ABS_X)
                    && axes.contains(AbsoluteAxisCode::ABS_Y))
                    || (axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_X)
                        && axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_Y))
            }) {
                println!("  -> Selected as touchscreen device!");
                unsafe {
                    let flags = libc::fcntl(device.as_raw_fd(), libc::F_GETFL, 0);
                    libc::fcntl(device.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
                return Some(device);
            }
        }
    }

    None
}

/// Drain all available events from the device, updating the persistent state.
pub fn read_touch(device: &mut Device, state: &mut TouchState) {
    while let Ok(events) = device.fetch_events() {
        for event in events {
            match event.destructure() {
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_X, val) => {
                    state.x = val;
                }
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_Y, val) => {
                    state.y = val;
                }
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_X, val) => {
                    state.x = val;
                }
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_Y, val) => {
                    state.y = val;
                }
                EventSummary::Key(_, KeyCode::BTN_TOUCH, val)
                | EventSummary::Key(_, KeyCode::BTN_TOOL_FINGER, val) => {
                    state.pressed = val != 0;
                }
                _ => {}
            }
        }
    }
}
