use rquickjs::function::{Func, MutFn, Opt};
use rquickjs::{CatchResultExt, Ctx, Function, Persistent};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Timer {
    id: u32,
    callback: Persistent<Function<'static>>,
    fire_at: Instant,
    /// None for one-shot (setTimeout), Some(duration) for repeating (setInterval).
    interval: Option<Duration>,
}

pub struct Timers {
    timers: Rc<RefCell<Vec<Timer>>>,
    next_id: Rc<RefCell<u32>>,
}

impl Timers {
    pub fn new() -> Self {
        Timers {
            timers: Rc::new(RefCell::new(Vec::new())),
            next_id: Rc::new(RefCell::new(1)),
        }
    }

    /// Register setTimeout, clearTimeout, setInterval, clearInterval on the JS global object.
    pub fn register(&self, ctx: &Ctx<'_>) {
        let timers = self.timers.clone();
        let next_id = self.next_id.clone();

        let timers_cell = timers.clone();
        let id_cell = next_id.clone();
        ctx.globals()
            .set(
                "setTimeout",
                Func::from(MutFn::from(
                    move |callback: Persistent<Function<'static>>, ms: Opt<f64>| -> u32 {
                        let id = allocate_id(&id_cell);
                        let delay = Duration::from_millis(ms.0.unwrap_or(0.0).max(0.0) as u64);

                        timers_cell.borrow_mut().push(Timer {
                            id,
                            callback,
                            fire_at: Instant::now() + delay,
                            interval: None,
                        });

                        id
                    },
                )),
            )
            .unwrap();

        let timers_cell = timers.clone();

        ctx.globals()
            .set(
                "clearTimeout",
                Func::from(MutFn::from(move |id: u32| {
                    timers_cell.borrow_mut().retain(|t| t.id != id);
                })),
            )
            .unwrap();

        let timers_cell = timers.clone();
        let id_cell = next_id.clone();

        ctx.globals()
            .set(
                "setInterval",
                Func::from(MutFn::from(
                    move |callback: Persistent<Function<'static>>, ms: Opt<f64>| -> u32 {
                        let id = allocate_id(&id_cell);
                        let interval = Duration::from_millis(ms.0.unwrap_or(0.0).max(0.0) as u64);

                        timers_cell.borrow_mut().push(Timer {
                            id,
                            callback,
                            fire_at: Instant::now() + interval,
                            interval: Some(interval),
                        });

                        id
                    },
                )),
            )
            .unwrap();

        let timers_cell = timers.clone();

        ctx.globals()
            .set(
                "clearInterval",
                Func::from(MutFn::from(move |id: u32| {
                    timers_cell.borrow_mut().retain(|t| t.id != id);
                })),
            )
            .unwrap();
    }

    /// Fire any expired timers. Intervals are rescheduled; timeouts are removed.
    pub fn tick(&self, ctx: &Ctx<'_>) {
        let now = Instant::now();

        let ready: Vec<Persistent<Function<'static>>> = {
            let mut timers = self.timers.borrow_mut();
            let mut ready = Vec::new();

            for timer in timers.iter_mut() {
                if timer.fire_at <= now {
                    ready.push(timer.callback.clone());

                    if let Some(interval) = timer.interval {
                        timer.fire_at = now + interval;
                    }
                }
            }

            timers.retain(|t| t.interval.is_some() || t.fire_at > now);
            ready
        };

        for cb in ready {
            let func = cb.restore(ctx).unwrap();

            if let Err(e) = func.call::<_, ()>(()).catch(&ctx) {
                println!("Timer callback error: {}", e);
            }
        }
    }

    /// Drop all timers. Must be called before the Runtime is dropped.
    pub fn clear(&self) {
        self.timers.borrow_mut().clear();
    }
}

fn allocate_id(next_id: &RefCell<u32>) -> u32 {
    let mut id_ref = next_id.borrow_mut();
    let id = *id_ref;
    *id_ref += 1;
    id
}
