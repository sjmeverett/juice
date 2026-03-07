use juice::engine::JsModule;
use rquickjs::{Object, prelude::Func};

pub struct Console {}

impl JsModule for Console {
    fn register(&self, ctx: &rquickjs::Ctx<'_>) {
        let console = Object::new(ctx.clone()).unwrap();

        console
            .set(
                "log",
                Func::from(|msg: String| {
                    println!("[JS] {}", msg);
                }),
            )
            .unwrap();

        console
            .set(
                "error",
                Func::from(|msg: String| {
                    eprintln!("[JS] {}", msg);
                }),
            )
            .unwrap();

        ctx.globals().set("console", console).unwrap();
    }
}
