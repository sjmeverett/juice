use crate::{layout, render, tree};
use fontdue::Font;
use rquickjs::function::{Func, MutFn};
use rquickjs::{CatchResultExt, Context, Ctx, Object, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Engine {
    _rt: Runtime,
    ctx: Context,
    tree_json: Rc<RefCell<String>>,
}

impl Engine {
    pub fn new() -> Self {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let tree_json = Rc::new(RefCell::new(String::new()));

        ctx.with(|ctx| {
            ctx.eval::<(), _>("globalThis.setTimeout = (fn, ms) => { fn(); };")
                .catch(&ctx)
                .unwrap();

            // Register renderer global object
            let renderer = Object::new(ctx.clone()).unwrap();

            let tree_cell = tree_json.clone();
            renderer
                .set(
                    "setTree",
                    Func::from(MutFn::from(move |json: String| {
                        *tree_cell.borrow_mut() = json;
                    })),
                )
                .unwrap();

            ctx.globals().set("renderer", renderer).unwrap();
        });

        Engine {
            _rt: rt,
            ctx,
            tree_json,
        }
    }

    /// Access the JS context directly for registering native functions.
    /// Call before boot().
    pub fn with_context<R>(&self, f: impl FnOnce(Ctx) -> R) -> R {
        self.ctx.with(f)
    }

    /// Evaluate the JS bundle. Call after registering native functions.
    pub fn boot(&self, bundle: &str) {
        self.ctx.with(|ctx| {
            ctx.eval::<(), _>(bundle).catch(&ctx).unwrap();
        });
    }

    pub fn dispatch_event(&self, node_id: u32, event_type: &str) {
        self.ctx.with(|ctx| {
            let script = format!(
                "globalThis.__dispatchEvent__({}, '{}');",
                node_id, event_type
            );
            ctx.eval::<(), _>(script.as_str()).catch(&ctx).unwrap();

            // Flush microtasks (Preact schedules re-renders via promises)
            while ctx.execute_pending_job() {}
        });
    }

    pub fn refresh_tree(&self) {
        self.ctx.with(|ctx| {
            ctx.eval::<(), _>("globalThis.__refreshTree__();")
                .catch(&ctx)
                .unwrap();
        });
    }

    pub fn read_tree(&self) -> String {
        self.tree_json.borrow().clone()
    }
}

pub fn read_and_layout(
    engine: &Engine,
    default_font: &str,
    fonts: &HashMap<String, Font>,
    width: f32,
    height: f32,
) -> layout::LayoutTree {
    let tree_json = engine.read_tree();
    let widget_tree = tree::parse_tree(&tree_json).expect("Failed to parse widget tree");
    let mut layout_tree = layout::build_layout_tree(&widget_tree, default_font);
    layout::compute_layout(&mut layout_tree, fonts, width, height);
    layout_tree
}

pub fn rerender(
    engine: &Engine,
    default_font: &str,
    fonts: &HashMap<String, Font>,
    fb: &mut render::Framebuffer,
    display: &mut impl embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb888>,
    width: f32,
    height: f32,
) -> layout::LayoutTree {
    engine.refresh_tree();
    let layout_tree = read_and_layout(engine, default_font, fonts, width, height);

    fb.clear(layout::RgbColor { r: 0, g: 0, b: 0 });
    render::render_tree(fb, &layout_tree, fonts);
    fb.flush(display);

    layout_tree
}
