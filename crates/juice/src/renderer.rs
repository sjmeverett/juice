use base64::engine::general_purpose;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle},
};
use fontdue::{Font, FontSettings};
use resvg::{tiny_skia::Pixmap, usvg::Tree};
use rquickjs::{
    CatchResultExt, Ctx, Function, Object, Persistent,
    prelude::{Func, MutFn},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use taffy::NodeId;

use crate::{
    canvas::Canvas,
    dom::{Dom, NodeKind},
    engine::{Engine, JsModule},
    inherited_style::InheritedStyle,
};

pub struct Renderer {
    pub engine: Engine,
    pub canvas: Canvas,
    pub dom: Rc<RefCell<Dom>>,

    modules: Vec<Box<dyn JsModule>>,
    fonts: Rc<RefCell<HashMap<String, Font>>>,
    event_callback: Rc<RefCell<Option<Persistent<Function<'static>>>>>,
    should_update: Rc<RefCell<bool>>,
}

impl Renderer {
    pub async fn new(
        canvas: Canvas,
        fonts: HashMap<String, Font>,
        base_style: InheritedStyle,
        modules: Vec<Box<dyn JsModule>>,
    ) -> Self {
        let renderer = Self {
            engine: Engine::new(&modules).await,
            canvas,
            fonts: Rc::new(RefCell::new(fonts)),
            dom: Rc::new(RefCell::new(Dom::new(base_style))),
            event_callback: Rc::new(RefCell::new(None)),
            should_update: Rc::new(RefCell::new(false)),
            modules,
        };

        renderer
            .engine
            .with_context(|ctx| {
                renderer.register(&ctx);
                renderer.dom.register(&ctx);
            })
            .await;

        renderer
    }

    pub async fn tick(&self) {
        self.engine.tick().await;
    }

    pub fn flush(&mut self, display: &mut impl DrawTarget<Color = Rgb888>) {
        self.canvas.draw_to_drawtarget(display);
    }

    pub fn render(&mut self) -> bool {
        if *self.should_update.borrow() {
            *self.should_update.borrow_mut() = false;

            let mut dom = self.dom.borrow_mut();

            if let Some(root) = dom.root_node_id {
                render_node(
                    &mut dom,
                    &mut self.canvas,
                    &*self.fonts.borrow(),
                    root,
                    0.0,
                    0.0,
                );

                return true;
            }
        }

        false
    }

    pub async fn dispatch_event(
        &self,
        node_id: u64,
        event_name: &str,
        build_details: impl FnOnce(Ctx, &Object),
    ) {
        let Some(callback) = self.event_callback.borrow().clone() else {
            eprintln!("Could not borrow callback");
            return;
        };

        self.engine
            .with_context(|ctx| {
                let event = Object::new(ctx.clone()).unwrap();
                event.set("type", event_name.to_string()).unwrap();

                let details = Object::new(ctx.clone()).unwrap();
                build_details(ctx.clone(), &details);
                event.set("details", details).unwrap();

                let callback = callback.restore(&ctx).unwrap();

                if let Err(err) = callback.call::<_, ()>((node_id, event)).catch(&ctx) {
                    eprintln!("Error calling event callback: {}", err)
                }

                while ctx.execute_pending_job() {}
            })
            .await;
    }

    pub async fn dispatch_xy_event(&self, event_name: &str, x: f32, y: f32) {
        let node_id = self.dom.borrow().node_at_point(x, y);

        let Some(node_id) = node_id else {
            return;
        };

        self.dispatch_event(node_id, event_name, |_ctx, details| {
            details.set("x", x).unwrap();
            details.set("y", y).unwrap();
        })
        .await;
    }

    pub async fn reload(&mut self, js: &str) {
        self.event_callback.borrow_mut().take();

        self.engine = Engine::new(&self.modules).await;

        self.engine
            .with_context(|ctx| {
                self.register(&ctx);
                self.dom.register(&ctx);
            })
            .await;

        self.engine.load(js).await;
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.event_callback.borrow_mut().take();
    }
}

fn render_node(
    dom: &mut Dom,
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

    let Some(ctx) = dom.get_node_mut(node_id) else {
        return;
    };

    let render_w = w as u32;
    let render_h = h as u32;

    match &mut ctx.kind {
        NodeKind::Element {
            background: Some(bg),
            border_radius,
            ..
        } => {
            let color = Rgb888::new(bg.r, bg.g, bg.b);
            let style = PrimitiveStyle::with_fill(color);

            let rect = Rectangle::new(
                Point::new(x as i32, y as i32),
                Size::new(render_w, render_h),
            );

            if *border_radius > 0.0 {
                let r = *border_radius as u32;
                let _ = RoundedRectangle::new(rect, CornerRadii::new(Size::new(r, r)))
                    .into_styled(style)
                    .draw(canvas);
            } else {
                let _ = rect.into_styled(style).draw(canvas);
            }
            ctx.render_dirty = false;
        }

        NodeKind::Text { text, wrap_width } => {
            if let Some(font) = fonts.get(&ctx.resolved_style.font_name) {
                canvas.draw_text(
                    font,
                    text,
                    ctx.resolved_style.font_size,
                    ctx.resolved_style.color,
                    x,
                    y,
                    *wrap_width,
                    ctx.resolved_style.text_align,
                    w,
                );
            }
            ctx.render_dirty = false;
        }

        NodeKind::Svg { markup, .. } => {
            if render_w > 0 && render_h > 0 {
                // Use cached raster if available and not dirty
                let needs_rasterize = ctx.render_dirty
                    || ctx
                        .cached_raster
                        .as_ref()
                        .map_or(true, |c| c.width != render_w || c.height != render_h);

                if needs_rasterize {
                    let current_color = ctx.resolved_style.with_overrides(&ctx.overrides).color;
                    let color_hex = format!(
                        "#{:02x}{:02x}{:02x}",
                        current_color.r, current_color.g, current_color.b
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

                                let data = pixmap.data().to_vec();
                                canvas.blit_premultiplied_rgba(
                                    &data, render_w, render_h, x as i32, y as i32,
                                );
                                ctx.cached_raster = Some(crate::dom::CachedRaster {
                                    data,
                                    width: render_w,
                                    height: render_h,
                                });
                            }
                        }
                        Err(err) => {
                            println!("Error parsing SVG: {:?}", err);
                        }
                    }
                } else if let Some(cache) = &ctx.cached_raster {
                    canvas.blit_premultiplied_rgba(
                        &cache.data,
                        cache.width,
                        cache.height,
                        x as i32,
                        y as i32,
                    );
                }
            }
            ctx.render_dirty = false;
        }

        NodeKind::Image {
            data,
            img_width,
            img_height,
            ..
        } => {
            if !data.is_empty() && *img_width > 0 && *img_height > 0 && render_w > 0 && render_h > 0
            {
                // Use cached raster if available and not dirty
                let needs_rasterize = ctx.render_dirty
                    || ctx
                        .cached_raster
                        .as_ref()
                        .map_or(true, |c| c.width != render_w || c.height != render_h);

                if needs_rasterize {
                    if *img_width == render_w && *img_height == render_h {
                        // No resize needed, blit directly and cache the raw data
                        canvas.blit_rgba(data, *img_width, *img_height, x as i32, y as i32);
                        ctx.cached_raster = Some(crate::dom::CachedRaster {
                            data: data.clone(),
                            width: render_w,
                            height: render_h,
                        });
                    } else if let Some(src_img) =
                        image::RgbaImage::from_raw(*img_width, *img_height, data.clone())
                    {
                        let resized = image::imageops::resize(
                            &src_img,
                            render_w,
                            render_h,
                            image::imageops::FilterType::Triangle,
                        );
                        let resized_data = resized.into_raw();
                        canvas.blit_rgba(&resized_data, render_w, render_h, x as i32, y as i32);
                        ctx.cached_raster = Some(crate::dom::CachedRaster {
                            data: resized_data,
                            width: render_w,
                            height: render_h,
                        });
                    }
                } else if let Some(cache) = &ctx.cached_raster {
                    canvas.blit_rgba(&cache.data, cache.width, cache.height, x as i32, y as i32);
                }
            }
            ctx.render_dirty = false;
        }

        _ => {}
    }

    if let Some(children) = dom.get_children(node_id) {
        for child_id in children {
            render_node(dom, canvas, fonts, child_id, x, y);
        }
    }
}

impl JsModule for Renderer {
    fn register(&self, ctx: &Ctx<'_>) {
        let renderer = Object::new(ctx.clone()).unwrap();

        let dom_cell = self.dom.clone();
        let should_update_cell = self.should_update.clone();
        let event_callback_cell = self.event_callback.clone();
        let fonts_cell = self.fonts.clone();
        let fonts_for_add = self.fonts.clone();
        let canvas_width = self.canvas.width as f32;
        let canvas_height = self.canvas.height as f32;

        renderer
            .set(
                "update",
                Func::from(MutFn::from(
                    move |event_callback: Persistent<Function<'static>>| {
                        let mut dom = dom_cell.borrow_mut();
                        dom.compute_layout(&*fonts_cell.borrow(), canvas_width, canvas_height);
                        *should_update_cell.borrow_mut() = true;
                        *event_callback_cell.borrow_mut() = Some(event_callback);
                    },
                )),
            )
            .unwrap();

        renderer
            .set(
                "addFont",
                Func::from(MutFn::from(move |name: String, src: String| {
                    match src.split(',').nth(1).and_then(|str| {
                        base64::Engine::decode(&general_purpose::STANDARD, str).ok()
                    }) {
                        Some(data) => {
                            let font = Font::from_bytes(data, FontSettings::default()).unwrap();
                            fonts_for_add.borrow_mut().insert(name, font);
                        }
                        None => {
                            println!("addFont: font not a valid base64 URL");
                        }
                    }
                })),
            )
            .unwrap();

        ctx.globals().set("renderer", renderer).unwrap();
    }
}
