mod drm;
mod input;

use juice::canvas::{Canvas, RgbColor};
use juice::inherited_style::InheritedStyle;
use juice::renderer::{EventName, Renderer};
use rquickjs::Object;
use rquickjs::function::Func;
use std::collections::HashMap;
use std::time::Duration;

use crate::input::{InputDevice, TouchEvent};

fn main() {
    let mut fonts = HashMap::new();

    #[cfg(feature = "hotreload")]
    let reload_rx = juice_dev::spawn_reload_listener();

    // Hardware init
    let mut display =
        drm::DrmDisplay::new("/dev/dri/card0").expect("Failed to initialize DRM display");
    let display_width = display.width();
    let display_height = display.height();

    println!("Display: {}x{}", display_width, display_height);

    let canvas = Canvas::new(display_width, display_height);
    let default_font = "Roboto-Regular";

    let mut renderer = Renderer::new(
        |ctx| {
            let console = Object::new(ctx.clone()).unwrap();

            console
                .set(
                    "log",
                    Func::from(|msg: String| {
                        println!("[JS] {}", msg);
                    }),
                )
                .unwrap();

            ctx.globals().set("console", console).unwrap();
        },
        canvas,
        fonts,
        InheritedStyle {
            color: RgbColor::from_array([255, 255, 255]),
            font_name: default_font.to_string(),
            font_size: 24.0,
        },
    );

    #[cfg(debug_assertions)]
    let bundle = std::fs::read_to_string("dist/bundle.js").expect("Run 'npm run build' first");
    #[cfg(not(debug_assertions))]
    let bundle = include_str!("../../../dist/bundle.js").to_string();

    renderer.engine.load(&bundle);

    // Touch input
    let mut touch_device = InputDevice::get_touchscreen_device();

    if let Some(ref mut touch_device) = touch_device {
        touch_device.set_nonblocking();
    } else {
        println!("Warning: No touchscreen device found");
    }

    // Event loop
    loop {
        if let Some(ref mut touch_device) = touch_device {
            match touch_device.read_touch_event() {
                Some(TouchEvent::PressIn { x, y }) => {
                    renderer.press_event(x as f32, y as f32, EventName::PressIn);
                }
                Some(TouchEvent::PressOut { x, y }) => {
                    renderer.press_event(x as f32, y as f32, EventName::PressOut);
                }
                _ => {}
            }
        }

        renderer.tick();

        if renderer.render() {
            display.blit_from(&renderer.canvas);
        }

        #[cfg(feature = "hotreload")]
        if let Ok(new_bundle) = reload_rx.try_recv() {
            println!("[dev] reloading bundle...");
            renderer.reload(&new_bundle);
        }

        std::thread::sleep(Duration::from_millis(16));
    }
}
