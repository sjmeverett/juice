use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    sdl2::MouseButton, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use fontdue::{Font, FontSettings};
use jsui::{engine, layout, render};
use rquickjs::function::Func;
use std::collections::HashMap;

const DISPLAY_WIDTH: u32 = 480;
const DISPLAY_HEIGHT: u32 = 320;

fn make_engine(bundle: &str) -> engine::Engine {
    let eng = engine::Engine::new();
    eng.with_context(|ctx| {
        ctx.globals()
            .set(
                "nativeLog",
                Func::from(|msg: String| {
                    println!("[JS] {}", msg);
                }),
            )
            .unwrap();
    });
    eng.boot(bundle);
    eng
}

fn main() {
    // Load all fonts from assets directory
    let mut fonts = HashMap::new();
    let assets_dir = std::path::Path::new("assets");
    if let Ok(entries) = std::fs::read_dir(assets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "ttf") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let data = std::fs::read(&path).unwrap();
                let font = Font::from_bytes(data, FontSettings::default()).unwrap();
                fonts.insert(name, font);
            }
        }
    }

    let default_font = fonts
        .keys()
        .next()
        .expect("No .ttf fonts found in assets/")
        .clone();

    #[cfg(debug_assertions)]
    let bundle = std::fs::read_to_string("dist/bundle.js").expect("Run 'npm run build' first");
    #[cfg(not(debug_assertions))]
    let bundle = include_str!("../../../dist/bundle.js").to_string();

    #[cfg(not(feature = "hotreload"))]
    let engine = make_engine(&bundle);
    #[cfg(feature = "hotreload")]
    let mut engine = make_engine(&bundle);

    #[cfg(feature = "hotreload")]
    let reload_rx = jsui_dev::spawn_reload_listener();

    // Initial tree read + layout + render
    let mut layout_tree = engine::read_and_layout(
        &engine,
        &default_font,
        &fonts,
        DISPLAY_WIDTH as f32,
        DISPLAY_HEIGHT as f32,
    );

    let mut fb = render::Framebuffer::new(DISPLAY_WIDTH, DISPLAY_HEIGHT);
    let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT));
    render::render_tree(&mut fb, &layout_tree, &fonts);
    fb.flush(&mut display);

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
                    if let Some(js_node_id) =
                        layout::hit_test(&layout_tree, point.x as f32, point.y as f32)
                    {
                        engine.dispatch_event(js_node_id, "PressIn");
                    }
                }

                SimulatorEvent::MouseButtonUp {
                    point,
                    mouse_btn: MouseButton::Left,
                } => {
                    if let Some(js_node_id) =
                        layout::hit_test(&layout_tree, point.x as f32, point.y as f32)
                    {
                        engine.dispatch_event(js_node_id, "PressOut");
                    }
                }

                _ => {}
            }
        }

        engine.tick();

        #[cfg(feature = "hotreload")]
        if let Ok(new_bundle) = reload_rx.try_recv() {
            println!("[dev] reloading bundle...");
            engine = make_engine(&new_bundle);
        }

        if engine.has_update() {
            layout_tree = engine::rerender(
                &engine,
                &default_font,
                &fonts,
                &mut fb,
                &mut display,
                DISPLAY_WIDTH as f32,
                DISPLAY_HEIGHT as f32,
            );
        }
    }
}
