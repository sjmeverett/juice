use crate::timers::Timers;
use rquickjs::{CatchResultExt, Context, Ctx, Runtime};

pub struct Engine {
    _js_runtime: Runtime,
    js_context: Context,
    timers: Timers,
}

impl Engine {
    pub fn new(setup: impl FnOnce(Ctx) -> ()) -> Self {
        let js_runtime = Runtime::new().unwrap();
        let js_context = Context::full(&js_runtime).unwrap();
        let timers = Timers::new();

        js_context.with(|ctx| {
            timers.register(&ctx);
            setup(ctx);
        });

        Self {
            _js_runtime: js_runtime,
            js_context,
            timers,
        }
    }

    pub fn with_context<R>(&self, f: impl FnOnce(Ctx) -> R) -> R {
        self.js_context.with(f)
    }

    pub fn load(&self, js: &str) {
        self.with_context(|ctx| {
            if let Err(err) = ctx.eval::<(), _>(js).catch(&ctx) {
                eprintln!("Error loading JS: {}", err);
            }
        })
    }

    pub fn tick(&self) {
        self.with_context(|ctx| {
            self.timers.tick(&ctx);
            while ctx.execute_pending_job() {}
        });
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Clear Persistent values before the Runtime drops, otherwise it aborts.
        self.timers.clear();
    }
}
