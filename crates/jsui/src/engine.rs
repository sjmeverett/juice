use crate::{layout, render, tree};
use fontdue::Font;
use rquickjs::function::{Func, MutFn};
use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const TIMER_SHIM: &str = r#"
(() => {
    const timers = new Map();
    let nextId = 1;

    globalThis.setTimeout = (fn, ms) => {
        const id = nextId++;
        timers.set(id, { fn, fire: Date.now() + (ms || 0) });
        return id;
    };

    globalThis.clearTimeout = (id) => {
        timers.delete(id);
    };

    globalThis.__tickTimers__ = () => {
        const now = Date.now();
        for (const [id, timer] of timers) {
            if (timer.fire <= now) {
                timers.delete(id);
                timer.fn();
            }
        }
    };
})();
"#;

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
            ctx.eval::<(), _>(TIMER_SHIM).catch(&ctx).unwrap();

            // Register renderer global object
            let renderer = Object::new(ctx.clone()).unwrap();

            let tree_cell = tree_json.clone();
            renderer
                .set(
                    "setContents",
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
            let doc: Object = ctx.globals().get("document").unwrap();
            let on_event: Function = doc.get("onEvent").unwrap();
            on_event
                .call::<_, ()>((node_id, event_type))
                .catch(&ctx)
                .unwrap();

            // Flush microtasks (Preact schedules re-renders via promises)
            while ctx.execute_pending_job() {}
        });
    }

    /// Run any timers that have expired. Call once per frame from your event loop.
    pub fn tick(&self) {
        self.ctx.with(|ctx| {
            ctx.eval::<(), _>("__tickTimers__()").catch(&ctx).unwrap();
            while ctx.execute_pending_job() {}
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
    let layout_tree = read_and_layout(engine, default_font, fonts, width, height);

    fb.clear(layout::RgbColor { r: 0, g: 0, b: 0 });
    render::render_tree(fb, &layout_tree, fonts);
    fb.flush(display);

    layout_tree
}
