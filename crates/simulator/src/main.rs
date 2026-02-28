use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::MouseButton,
};
use fontdue::{Font, FontSettings};
use juice::{
    canvas::{Canvas, RgbColor},
    inherited_style::InheritedStyle,
    renderer::Renderer,
};
use rquickjs::{Object, function::Func};
use std::collections::HashMap;

const DISPLAY_WIDTH: u32 = 800;
const DISPLAY_HEIGHT: u32 = 800;

fn main() {
    let mut fonts = HashMap::new();
    let reload_rx = juice_dev::spawn_reload_listener();
    let canvas = Canvas::new(DISPLAY_WIDTH, DISPLAY_HEIGHT);
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

    let bundle = std::fs::read_to_string("dist/bundle.js").expect("Run 'npm run build' first");
    renderer.engine.load(&bundle);

    let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT));
    renderer.flush(&mut display);

    let output_settings = OutputSettingsBuilder::new().build();
    let mut window = Window::new("Preact Embedded", &output_settings);

    loop {
        window.update(&display);

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => return,

                SimulatorEvent::MouseButtonDown {
                    point,
                    mouse_btn: MouseButton::Left,
                } => {
                    renderer.press_event(
                        point.x as f32,
                        point.y as f32,
                        juice::renderer::EventName::PressIn,
                    );
                }

                SimulatorEvent::MouseButtonUp {
                    point,
                    mouse_btn: MouseButton::Left,
                } => {
                    renderer.press_event(
                        point.x as f32,
                        point.y as f32,
                        juice::renderer::EventName::PressOut,
                    );
                }

                _ => {}
            }
        }

        renderer.tick();

        if renderer.render() {
            renderer.flush(&mut display);
        }

        if let Ok(new_bundle) = reload_rx.try_recv() {
            println!("[dev] reloading bundle...");
            renderer.reload(&new_bundle);
        }
    }
}
