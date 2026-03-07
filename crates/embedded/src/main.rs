mod console;
mod drm;
mod input;

use juice::canvas::{Canvas, RgbColor};
use juice::inherited_style::{InheritedStyle, TextAlign};
use juice::renderer::Renderer;
use std::collections::HashMap;
use std::time::Duration;

use crate::console::Console;
use crate::input::{InputDevice, TouchEvent};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = HashMap::new();

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
        canvas,
        fonts,
        InheritedStyle {
            color: RgbColor::from_array([255, 255, 255]),
            font_name: default_font.to_string(),
            font_size: 24.0,
            text_align: TextAlign::Left,
        },
        vec![Box::new(Console {})],
    )
    .await;

    let bundle = include_str!("../../../dist/bundle.js").to_string();

    renderer.engine.load(&bundle).await;

    // set up touchscreen input
    let mut touch_device = InputDevice::get_touchscreen_device();

    if touch_device.is_none() {
        println!("Warning: No touchscreen device found");
    }

    let mut frame_interval = tokio::time::interval(Duration::from_millis(16));

    // Event loop
    loop {
        // Wait for a frame tick, WS message, or touch event
        tokio::select! {
            _ = frame_interval.tick() => {}

            event = async { touch_device.as_mut().unwrap().next_event().await }, if touch_device.is_some() => {
                match event {
                    TouchEvent::PressIn { x, y } => {
                        renderer.dispatch_xy_event("PressIn", x as f32, y as f32).await;
                    }
                    TouchEvent::PressOut { x, y } => {
                        renderer.dispatch_xy_event("PressOut", x as f32, y as f32).await;
                    }
                    _ => {}
                }
            }
        }

        renderer.tick().await;

        if renderer.render() {
            display.blit_from(&renderer.canvas);
        }

        #[cfg(feature = "hotreload")]
        if let Ok(new_bundle) = reload_rx.try_recv() {
            println!("[dev] reloading bundle...");
            renderer.reload(&new_bundle).await;
        }
    }
}
