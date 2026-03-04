use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::MouseButton,
};
use juice::canvas::{Canvas, RgbColor};
use juice::inherited_style::{InheritedStyle, TextAlign};
use juice::renderer::Renderer;
use rquickjs::Object;
use rquickjs::prelude::Func;
use std::collections::HashMap;
use std::time::Duration;

const DISPLAY_WIDTH: u32 = 800;
const DISPLAY_HEIGHT: u32 = 800;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let canvas = Canvas::new(DISPLAY_WIDTH, DISPLAY_HEIGHT);
    let fonts = HashMap::new();
    let default_font = "Roboto-Regular";

    let reload_rx = juice_dev::spawn_reload_listener();

    // create the juice renderer
    let mut renderer = Renderer::new(
        move |ctx| {
            let console = Object::new(ctx.clone()).unwrap();

            console
                .set(
                    "log",
                    Func::from(|msg: String| {
                        println!("[JS] {}", msg);
                    }),
                )
                .unwrap();

            console
                .set(
                    "error",
                    Func::from(|msg: String| {
                        eprintln!("[JS] {}", msg);
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
            text_align: TextAlign::Left,
        },
    )
    .await;

    println!("Created renderer");

    let bundle = std::fs::read_to_string("dist/bundle.js").expect("Run 'npm run build' first");
    renderer.engine.load(&bundle).await;

    let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT));
    renderer.flush(&mut display);

    let output_settings = OutputSettingsBuilder::new().build();
    let mut window = Window::new("Preact Embedded", &output_settings);

    let mut frame_interval = tokio::time::interval(Duration::from_millis(16));

    // main event loop

    loop {
        frame_interval.tick().await;
        window.update(&display);

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => return Ok(()),

                SimulatorEvent::MouseButtonDown {
                    point,
                    mouse_btn: MouseButton::Left,
                } => {
                    renderer
                        .dispatch_xy_event("PressIn", point.x as f32, point.y as f32)
                        .await;
                }

                SimulatorEvent::MouseButtonUp {
                    point,
                    mouse_btn: MouseButton::Left,
                } => {
                    renderer
                        .dispatch_xy_event("PressOut", point.x as f32, point.y as f32)
                        .await;
                }

                _ => {}
            }
        }

        renderer.tick().await;

        if renderer.render() {
            renderer.flush(&mut display);
        }

        if let Ok(new_bundle) = reload_rx.try_recv() {
            println!("[dev] reloading bundle...");
            renderer.reload(&new_bundle).await;
        }
    }
}
