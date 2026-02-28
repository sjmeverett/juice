use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle},
};
use fontdue::Font;
use resvg::{tiny_skia::Pixmap, usvg::Tree};
use rquickjs::{
    CatchResultExt, Ctx, Function, Object, Persistent,
    prelude::{Func, MutFn},
};
use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};
use taffy::NodeId;

use crate::{
    canvas::Canvas,
    dom::{Dom, NodeContext},
    engine::Engine,
    inherited_style::InheritedStyle,
};

pub struct Renderer {
    pub engine: Engine,
    pub canvas: Canvas,
    fonts: HashMap<String, Font>,
    base_style: InheritedStyle,
    event_callback: Rc<RefCell<Option<Persistent<Function<'static>>>>>,
    should_update: Rc<RefCell<bool>>,
    pub dom: Rc<RefCell<Option<Dom>>>,
    setup: Rc<dyn Fn(Ctx)>,
}

pub enum EventName {
    PressIn,
    PressOut,
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EventName::PressIn => "PressIn",
                EventName::PressOut => "PressOut",
            }
        )
    }
}

impl Renderer {
    pub fn new(
        setup: impl Fn(Ctx) + 'static,
        canvas: Canvas,
        fonts: HashMap<String, Font>,
        base_style: InheritedStyle,
    ) -> Self {
        let setup: Rc<dyn Fn(Ctx)> = Rc::new(setup);
        let setup2 = setup.clone();

        let renderer = Self {
            engine: Engine::new(move |ctx| setup2(ctx)),
            canvas,
            fonts,
            base_style,
            dom: Rc::new(RefCell::new(None)),
            event_callback: Rc::new(RefCell::new(None)),
            should_update: Rc::new(RefCell::new(false)),
            setup,
        };

        renderer.engine.with_context(|ctx| {
            renderer.register(&ctx);
        });

        renderer
    }

    pub fn tick(&self) {
        self.engine.tick();
    }

    pub fn flush(&self, display: &mut impl DrawTarget<Color = Rgb888>) {
        self.canvas.flush(display);
    }

    fn register(&self, ctx: &Ctx<'_>) {
        let renderer = Object::new(ctx.clone()).unwrap();

        let dom_cell = self.dom.clone();
        let should_update_cell = self.should_update.clone();
        let event_callback_cell = self.event_callback.clone();
        let base_style = self.base_style.clone();
        let fonts = Rc::new(self.fonts.clone());
        let canvas_width = self.canvas.width as f32;
        let canvas_height = self.canvas.height as f32;

        // register the globalThis.renderer object
        // it has a single method: update(content: string, eventCallback: (nodeId, event) => void)
        renderer
            .set(
                "update",
                Func::from(MutFn::from(
                    move |content: String, event_callback: Persistent<Function<'static>>| {
                        match Dom::new(
                            &content,
                            base_style.clone(),
                            &fonts,
                            canvas_width,
                            canvas_height,
                        ) {
                            Ok(dom) => {
                                *dom_cell.borrow_mut() = Some(dom);
                                *should_update_cell.borrow_mut() = true;
                                *event_callback_cell.borrow_mut() = Some(event_callback);
                            }
                            Err(err) => {
                                let col = err.column();
                                let start = col.saturating_sub(40);
                                let end = (col + 40).min(content.len());
                                let snippet = &content[start..end];
                                let pointer = " ".repeat(col - start) + "^";
                                println!(
                                    "Error creating DOM: {}\nNear: {}\n      {}",
                                    err, snippet, pointer
                                );
                            }
                        }
                    },
                )),
            )
            .unwrap();

        ctx.globals().set("renderer", renderer).unwrap();
    }

    pub fn render(&mut self) -> bool {
        if *self.should_update.borrow() {
            let dom_ref = self.dom.borrow();

            if let Some(dom) = dom_ref.as_ref() {
                render_node(dom, &mut self.canvas, &self.fonts, dom.root_id, 0.0, 0.0);
            }

            *self.should_update.borrow_mut() = false;
            true
        } else {
            false
        }
    }

    pub fn press_event(&self, x: f32, y: f32, event_name: EventName) {
        let callback = self.event_callback.borrow().clone();
        let node_id = self
            .dom
            .borrow()
            .as_ref()
            .and_then(|dom| dom.node_at_point(x, y));

        if let Some(callback) = callback
            && let Some(node_id) = node_id
        {
            self.engine.with_context(|ctx| {
                let event = Object::new(ctx.clone()).unwrap();
                event.set("type", event_name.to_string()).unwrap();

                let details = Object::new(ctx.clone()).unwrap();
                details.set("x", x).unwrap();
                details.set("y", y).unwrap();

                event.set("details", details).unwrap();

                let callback = callback.restore(&ctx).unwrap();
                let _ = callback.call::<_, ()>((node_id, event)).catch(&ctx);

                while ctx.execute_pending_job() {}
            });
        }
    }

    pub fn reload(&mut self, js: &str) {
        self.event_callback.borrow_mut().take();

        let setup = self.setup.clone();
        self.engine = Engine::new(move |ctx| setup(ctx));

        self.engine.with_context(|ctx| {
            self.register(&ctx);
        });

        self.engine.load(js);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.event_callback.borrow_mut().take();
    }
}

fn render_node(
    dom: &Dom,
    canvas: &mut Canvas,
    fonts: &HashMap<String, Font>,
    node_id: NodeId,
    parent_x: f32,
    parent_y: f32,
) {
    let layout = dom.get_layout(node_id).unwrap();

    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    let context = dom.get_context(node_id);

    match context {
        Some(NodeContext::Element {
            background: Some(bg),
            border_radius,
            ..
        }) => {
            let color = Rgb888::new(bg.r, bg.g, bg.b);
            let style = PrimitiveStyle::with_fill(color);

            let rect = Rectangle::new(
                Point::new(x as i32, y as i32),
                Size::new(w as u32, h as u32),
            );

            if *border_radius > 0.0 {
                let r = *border_radius as u32;

                let _ = RoundedRectangle::new(rect, CornerRadii::new(Size::new(r, r)))
                    .into_styled(style)
                    .draw(canvas);
            } else {
                let _ = rect.into_styled(style).draw(canvas);
            }
        }

        Some(NodeContext::Text {
            content,
            color,
            font_name,
            font_size,
        }) => {
            if let Some(font) = fonts.get(font_name) {
                canvas.draw_text(font, content, *font_size, *color, x, y);
            }
        }

        Some(NodeContext::Svg {
            markup,
            inherited_color,
            ..
        }) => {
            let render_w = w as u32;
            let render_h = h as u32;

            if render_w > 0 && render_h > 0 {
                let color_hex = format!(
                    "#{:02x}{:02x}{:02x}",
                    inherited_color.r, inherited_color.g, inherited_color.b
                );

                let resolved = markup.replace("currentColor", &color_hex);
                let options = resvg::usvg::Options::default();

                match Tree::from_str(&resolved, &options) {
                    Ok(tree) => {
                        if let Some(mut pixmap) = Pixmap::new(render_w, render_h) {
                            let svg_size = tree.size();
                            let sx = render_w as f32 / svg_size.width();
                            let sy = render_h as f32 / svg_size.height();
                            let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);

                            resvg::render(&tree, transform, &mut pixmap.as_mut());

                            canvas.blit_premultiplied_rgba(
                                pixmap.data(),
                                render_w,
                                render_h,
                                x as i32,
                                y as i32,
                            );
                        }
                    }
                    Err(err) => {
                        println!("Error parsing SVG: {:?}", err);
                    }
                }
            }
        }

        Some(NodeContext::Image {
            data,
            img_width,
            img_height,
            ..
        }) => {
            let render_w = w as u32;
            let render_h = h as u32;

            if !data.is_empty() && *img_width > 0 && *img_height > 0 && render_w > 0 && render_h > 0
            {
                // If dimensions match, blit directly; otherwise resize
                if *img_width == render_w && *img_height == render_h {
                    canvas.blit_rgba(data, *img_width, *img_height, x as i32, y as i32);
                } else if let Some(src_img) =
                    image::RgbaImage::from_raw(*img_width, *img_height, data.clone())
                {
                    let resized = image::imageops::resize(
                        &src_img,
                        render_w,
                        render_h,
                        image::imageops::FilterType::Triangle,
                    );
                    canvas.blit_rgba(resized.as_raw(), render_w, render_h, x as i32, y as i32);
                }
            }
        }

        _ => {}
    }

    if let Some(children) = dom.get_children(node_id) {
        for child_id in children {
            render_node(dom, canvas, fonts, child_id, x, y);
        }
    }
}
