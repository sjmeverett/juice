use crate::{layout, render, tree};
use fontdue::Font;
use rquickjs::function::{Func, MutFn};
use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Persistent, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Timer {
    id: u32,
    callback: Persistent<Function<'static>>,
    fire_at: Instant,
}

struct Listener {
    event: String,
    callback: Persistent<Function<'static>>,
}

pub struct Engine {
    _rt: Runtime,
    ctx: Context,
    tree_json: Rc<RefCell<String>>,
    dirty: Rc<RefCell<bool>>,
    timers: Rc<RefCell<Vec<Timer>>>,
    listeners: Rc<RefCell<Vec<Listener>>>,
}

impl Engine {
    pub fn new() -> Self {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let tree_json = Rc::new(RefCell::new(String::new()));
        let dirty = Rc::new(RefCell::new(false));
        let timers: Rc<RefCell<Vec<Timer>>> = Rc::new(RefCell::new(Vec::new()));
        let next_timer_id = Rc::new(RefCell::new(1u32));
        let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));

        ctx.with(|ctx| {
            // Register setTimeout
            let timers_cell = timers.clone();
            let id_cell = next_timer_id.clone();
            ctx.globals()
                .set(
                    "setTimeout",
                    Func::from(MutFn::from(
                        move |callback: Persistent<Function<'static>>, ms: Option<f64>| -> u32 {
                            let id = {
                                let mut id_ref = id_cell.borrow_mut();
                                let id = *id_ref;
                                *id_ref += 1;
                                id
                            };
                            let delay_ms = ms.unwrap_or(0.0).max(0.0) as u64;
                            timers_cell.borrow_mut().push(Timer {
                                id,
                                callback,
                                fire_at: Instant::now() + Duration::from_millis(delay_ms),
                            });
                            id
                        },
                    )),
                )
                .unwrap();

            // Register clearTimeout
            let timers_cell = timers.clone();
            ctx.globals()
                .set(
                    "clearTimeout",
                    Func::from(MutFn::from(move |id: u32| {
                        timers_cell.borrow_mut().retain(|t| t.id != id);
                    })),
                )
                .unwrap();

            // Create document object (JS fake-dom will add createElement etc. to it)
            let doc = Object::new(ctx.clone()).unwrap();
            ctx.globals().set("document", doc.clone()).unwrap();

            let listeners_cell = listeners.clone();
            doc.set(
                "addEventListener",
                Func::from(MutFn::from(
                    move |event: String, callback: Persistent<Function<'static>>| {
                        listeners_cell
                            .borrow_mut()
                            .push(Listener { event, callback });
                    },
                )),
            )
            .unwrap();

            let listeners_cell = listeners.clone();
            doc.set(
                "removeEventListener",
                Func::from(MutFn::from(
                    move |event: String, callback: Persistent<Function<'static>>| {
                        listeners_cell
                            .borrow_mut()
                            .retain(|l| l.event != event || l.callback != callback);
                    },
                )),
            )
            .unwrap();

            // Register renderer global object
            let renderer = Object::new(ctx.clone()).unwrap();

            let tree_cell = tree_json.clone();
            let dirty_cell = dirty.clone();
            renderer
                .set(
                    "setContents",
                    Func::from(MutFn::from(move |json: String| {
                        *tree_cell.borrow_mut() = json;
                        *dirty_cell.borrow_mut() = true;
                    })),
                )
                .unwrap();

            ctx.globals().set("renderer", renderer).unwrap();
        });

        Engine {
            _rt: rt,
            ctx,
            tree_json,
            dirty,
            timers,
            listeners,
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

    /// Dispatch a touch event. Fires all "event" listeners with (nodeId, eventType).
    /// Also flushes any expired timers so Preact's batched re-renders complete
    /// before the caller reads the tree.
    pub fn dispatch_event(&self, node_id: u32, event_type: &str) {
        let callbacks: Vec<Persistent<Function<'static>>> = self
            .listeners
            .borrow()
            .iter()
            .filter(|l| l.event == "event")
            .map(|l| l.callback.clone())
            .collect();

        if !callbacks.is_empty() {
            self.ctx.with(|ctx| {
                let event = Object::new(ctx.clone()).unwrap();
                event.set("nodeId", node_id).unwrap();
                event.set("type", event_type).unwrap();

                for cb in callbacks {
                    let func = cb.restore(&ctx).unwrap();
                    let _ = func.call::<_, ()>((event.clone(),)).catch(&ctx);
                }
                while ctx.execute_pending_job() {}
            });
        }

        // Flush any zero-delay timers (e.g. Preact's batched setState)
        self.tick();
    }

    /// Run any timers that have expired. Call once per frame from your event loop.
    pub fn tick(&self) {
        let now = Instant::now();
        let ready: Vec<Persistent<Function<'static>>> = {
            let mut timers = self.timers.borrow_mut();
            let mut ready = Vec::new();
            timers.retain(|t| {
                if t.fire_at <= now {
                    ready.push(t.callback.clone());
                    false
                } else {
                    true
                }
            });
            ready
        };

        if !ready.is_empty() {
            self.ctx.with(|ctx| {
                for cb in ready {
                    let func = cb.restore(&ctx).unwrap();
                    let _ = func.call::<_, ()>(()).catch(&ctx);
                }
                while ctx.execute_pending_job() {}
            });
        }
    }

    /// Returns true if `renderer.setContents` has been called since the last check.
    pub fn has_update(&self) -> bool {
        self.dirty.replace(false)
    }

    pub fn read_tree(&self) -> String {
        self.tree_json.borrow().clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Clear Persistent values before the Runtime drops, otherwise it aborts.
        self.timers.borrow_mut().clear();
        self.listeners.borrow_mut().clear();
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
