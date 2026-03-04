use crate::timers::Timers;
use rquickjs::{AsyncContext, AsyncRuntime, CatchResultExt, Ctx};

pub struct Engine {
    js_runtime: AsyncRuntime,
    js_context: AsyncContext,
    timers: Timers,
}

impl Engine {
    pub async fn new(setup: impl FnOnce(Ctx)) -> Self {
        let js_runtime = AsyncRuntime::new().unwrap();
        let js_context = AsyncContext::full(&js_runtime).await.unwrap();
        let timers = Timers::new();

        js_context
            .with(|ctx| {
                timers.register(&ctx);
                setup(ctx);
            })
            .await;

        Self {
            js_runtime,
            js_context,
            timers,
        }
    }

    pub async fn with_context<R>(&self, f: impl FnOnce(Ctx) -> R) -> R {
        self.js_context.with(f).await
    }

    /// Get the async context, for use with `rquickjs::async_with!`.
    pub fn context(&self) -> &AsyncContext {
        &self.js_context
    }

    pub async fn load(&self, js: &str) {
        self.with_context(|ctx| {
            if let Err(err) = ctx.eval::<(), _>(js).catch(&ctx) {
                eprintln!("Error loading JS: {}", err);
            }
        })
        .await
    }

    pub async fn tick(&self) {
        self.with_context(|ctx| {
            self.timers.tick(&ctx);
        })
        .await;

        // Drive the async runtime — poll spawned futures and process resolved promises.
        while self.js_runtime.execute_pending_job().await.unwrap_or(false) {}
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Clear Persistent values before the Runtime drops, otherwise it aborts.
        self.timers.clear();
    }
}
