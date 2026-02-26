mod drm;
mod input;

use fontdue::{Font, FontSettings};
use jsui::{engine, layout, render};
use rquickjs::function::Func;
use std::collections::HashMap;
use std::time::Duration;

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

    let bundle = std::fs::read_to_string("dist/bundle.js").expect("Run 'npm run build' first");

    // Boot QuickJS engine
    let engine = engine::Engine::new();

    // Register native functions
    engine.with_context(|ctx| {
        ctx.globals()
            .set(
                "nativeLog",
                Func::from(|msg: String| {
                    println!("[JS] {}", msg);
                }),
            )
            .unwrap();
    });

    engine.boot(&bundle);

    // Hardware init
    let mut display =
        drm::DrmDisplay::new("/dev/dri/card0").expect("Failed to initialize DRM display");
    let display_width = display.width();
    let display_height = display.height();

    println!("Display: {}x{}", display_width, display_height);

    // Initial tree read + layout + render
    let mut layout_tree = engine::read_and_layout(
        &engine,
        &default_font,
        &fonts,
        display_width as f32,
        display_height as f32,
    );

    let mut fb = render::Framebuffer::new(display_width, display_height);
    render::render_tree(&mut fb, &layout_tree, &fonts);
    fb.flush(&mut display);

    // Touch input
    let mut touch_device = input::find_touch_device();
    let mut touch_state = input::TouchState::default();
    let mut was_pressed = false;

    if touch_device.is_none() {
        println!("Warning: No touchscreen device found");
    }

    // Event loop
    loop {
        if let Some(ref mut dev) = touch_device {
            input::read_touch(dev, &mut touch_state);
        }

        if touch_state.pressed && !was_pressed {
            if let Some(js_node_id) = layout::hit_test(
                &layout_tree,
                touch_state.x as f32,
                touch_state.y as f32,
            ) {
                engine.dispatch_event(js_node_id, "PressIn");
            }
        }

        if !touch_state.pressed && was_pressed {
            if let Some(js_node_id) = layout::hit_test(
                &layout_tree,
                touch_state.x as f32,
                touch_state.y as f32,
            ) {
                engine.dispatch_event(js_node_id, "PressOut");
            }
        }

        was_pressed = touch_state.pressed;
        engine.tick();

        if engine.has_update() {
            layout_tree = engine::rerender(
                &engine,
                &default_font,
                &fonts,
                &mut fb,
                &mut display,
                display_width as f32,
                display_height as f32,
            );
        }

        std::thread::sleep(Duration::from_millis(16));
    }
}
