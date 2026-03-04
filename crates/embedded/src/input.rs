use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};
use std::{fs::read_dir, os::unix::io::AsRawFd};
use tokio::io::unix::AsyncFd;

#[derive(Clone, Copy, Debug)]
pub struct TouchState {
    pub x: i32,
    pub y: i32,
    pub pressed: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum TouchEvent {
    PressIn { x: i32, y: i32 },
    PressOut { x: i32, y: i32 },
    Move { x: i32, y: i32 },
}

pub struct InputDevice {
    async_fd: AsyncFd<Device>,
    pub touch_state: TouchState,
}

impl InputDevice {
    pub fn new(device: Device) -> Self {
        set_nonblocking(&device);

        Self {
            async_fd: AsyncFd::new(device).unwrap(),
            touch_state: TouchState {
                x: 0,
                y: 0,
                pressed: false,
            },
        }
    }

    pub fn get_touchscreen_device() -> Option<Self> {
        // Check for touchscreen capability before wrapping in AsyncFd,
        // since we need to inspect the device first
        read_dir("/dev/input")
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let device = Device::open(&path).ok()?;
                let name = device.name().unwrap_or("Unknown");
                println!("  Device: {} at {:?}", name, path);

                if is_touchscreen(&device) {
                    Some(Self::new(device))
                } else {
                    None
                }
            })
            .next()
    }

    pub async fn next_event(&mut self) -> TouchEvent {
        loop {
            self.async_fd.readable().await.unwrap().clear_ready();

            if let Some(event) = self.read_touch_event() {
                return event;
            }
        }
    }

    fn read_touch_state(&mut self) -> Option<TouchState> {
        let mut touch_state = self.touch_state;
        let mut has_event = false;

        while let Ok(events) = self.async_fd.get_mut().fetch_events() {
            for event in events {
                match event.destructure() {
                    EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_X, val) => {
                        touch_state.x = val;
                        has_event = true;
                    }
                    EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_Y, val) => {
                        touch_state.y = val;
                        has_event = true;
                    }
                    EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_X, val) => {
                        touch_state.x = val;
                        has_event = true;
                    }
                    EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_Y, val) => {
                        touch_state.y = val;
                        has_event = true;
                    }
                    EventSummary::Key(_, KeyCode::BTN_TOUCH, val)
                    | EventSummary::Key(_, KeyCode::BTN_TOOL_FINGER, val) => {
                        touch_state.pressed = val != 0;
                        has_event = true;
                    }
                    _ => {}
                }
            }
        }

        if has_event { Some(touch_state) } else { None }
    }

    fn read_touch_event(&mut self) -> Option<TouchEvent> {
        let touch_state = self.read_touch_state()?;

        let result = if touch_state.pressed && !self.touch_state.pressed {
            Some(TouchEvent::PressIn {
                x: touch_state.x,
                y: touch_state.y,
            })
        } else if !touch_state.pressed && self.touch_state.pressed {
            Some(TouchEvent::PressOut {
                x: touch_state.x,
                y: touch_state.y,
            })
        } else if self.touch_state.x != touch_state.x || self.touch_state.y != touch_state.y {
            Some(TouchEvent::Move {
                x: touch_state.x,
                y: touch_state.y,
            })
        } else {
            None
        };

        self.touch_state = touch_state;
        result
    }
}

fn set_nonblocking(device: &Device) {
    unsafe {
        let flags = libc::fcntl(device.as_raw_fd(), libc::F_GETFL, 0);
        libc::fcntl(device.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

fn is_touchscreen(device: &Device) -> bool {
    if let Some(axes) = device.supported_absolute_axes() {
        (axes.contains(AbsoluteAxisCode::ABS_X) && axes.contains(AbsoluteAxisCode::ABS_Y))
            || (axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_X)
                && axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_Y))
    } else {
        false
    }
}
