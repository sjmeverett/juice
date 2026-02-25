# jsui

A JavaScript UI engine for embedded Linux displays. Runs Preact (or any JS framework) inside QuickJS, serializes a virtual DOM tree to JSON, then lays it out with Taffy and renders it to a framebuffer using fontdue.

## Architecture

```
┌─────────────────────────────────────────────────┐
│  TypeScript / Preact                            │
│  Fake DOM → serialize → renderer.setTree(json)   │
├─────────────────────────────────────────────────┤
│  QuickJS (via rquickjs)                         │
│  JS ←→ Rust function bridge                     │
├─────────────────────────────────────────────────┤
│  jsui lib crate                                 │
│  Tree parsing → Taffy layout → Framebuffer      │
├─────────────────────────────────────────────────┤
│  Display target (DrawTarget<Color = Rgb888>)    │
│  simulator: SDL2 window                         │
│  embedded: DRM/KMS + evdev touchscreen          │
└─────────────────────────────────────────────────┘
```

## Crates

| Crate | Type | Description |
|-------|------|-------------|
| `crates/jsui` | lib | Core engine: QuickJS runtime, tree parsing, Taffy layout, framebuffer rendering |
| `crates/simulator` | bin | Desktop simulator using embedded-graphics-simulator (SDL2) |
| `crates/embedded` | bin | Embedded Linux target using DRM/KMS display + evdev touch input |

## Quick start

```sh
npm install
npm run build          # builds ts/ → dist/bundle.js
cargo run -p simulator # opens SDL2 window
```

## JS ↔ Rust bridge

### Built-in globals

The engine auto-registers a `renderer` object:

```js
renderer.setTree(json) // sends the serialized widget tree to Rust
```

### Registering native functions

Additional functions are registered via `with_context` before `boot`:

```rust
let engine = engine::Engine::new();

engine.with_context(|ctx| {
    ctx.globals()
        .set("nativeLog", Func::from(|msg: String| {
            println!("[JS] {}", msg);
        }))
        .unwrap();
});

engine.boot(&bundle);
```

Supported argument/return types: `String`, `f64`, `i32`, `bool`, `()`, `Option<T>`.

For stateful closures (FnMut), wrap with `MutFn`:

```rust
use rquickjs::function::{Func, MutFn};

let mut count = 0u32;
ctx.globals()
    .set("nativeIncrement", Func::from(MutFn::from(move || -> u32 {
        count += 1;
        count
    })))
    .unwrap();
```

### Registering global objects

Use `rquickjs::Object` to group related functions under a namespace:

```rust
use rquickjs::{Func, Object};

engine.with_context(|ctx| {
    let device = Object::new(ctx.clone()).unwrap();
    device.set("getBrightness", Func::from(|| -> f64 { 0.8 })).unwrap();
    device.set("reboot", Func::from(|| { /* ... */ })).unwrap();
    ctx.globals().set("device", device).unwrap();
});
```

```js
device.getBrightness() // 0.8
device.reboot()
```

### Registering native classes

Use the `#[rquickjs::class]` and `#[rquickjs::methods]` macros:

```rust
use rquickjs::{class::Trace, Class, JsLifetime};

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct File {
    path: String,
    contents: String,
}

#[rquickjs::methods]
impl File {
    #[qjs(constructor)]
    pub fn new(path: String) -> Self {
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        File { path, contents }
    }

    #[qjs(get)]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    #[qjs(get)]
    pub fn contents(&self) -> String {
        self.contents.clone()
    }

    pub fn write(&mut self, data: String) {
        std::fs::write(&self.path, &data).unwrap();
        self.contents = data;
    }
}
```

Register in the context:

```rust
engine.with_context(|ctx| {
    Class::<File>::define(&ctx.globals()).unwrap();
});
```

JS usage:

```js
const f = new File("/tmp/hello.txt");
f.contents;          // reads file
f.write("new data"); // writes file
```

Key attributes:
- `#[qjs(constructor)]` — called by `new`
- `#[qjs(get)]` / `#[qjs(set)]` — property accessors
- `#[qjs(static)]` — static method on the constructor
- `#[qjs(rename = "jsName")]` — rename for JS
- `#[qjs(skip)]` — hide from JS

### TypeScript declarations

Declare native functions/classes in `ts/fake-dom.ts`:

```typescript
declare global {
    var renderer: {
        setTree(json: string): void;
    };
    function nativeLog(message: string): void;
}
```

## Cross-compilation

For ARM targets:

```sh
cross build -p embedded --release
```

Requires `cross` installed. The `Dockerfile.cross` installs `libclang-dev` for rquickjs bindgen.

Deploy to device: copy the binary + `dist/bundle.js` + `assets/` directory.

## Components (TypeScript)

```tsx
<Screen style={{ background: "#000000" }}>
    <Label style={{ color: "#ffffff", font: "CabinetGrotesk-Bold", fontSize: 72 }}>
        Hello World
    </Label>
    <Button onPress={() => doSomething()}>
        Click me
    </Button>
</Screen>
```

Supported style properties: `background`, `color`, `font`, `fontSize`, `flexDirection`, `flexGrow`, `flexShrink`, `width`, `height`, `padding`, `paddingLeft/Right/Top/Bottom`, `gap`.

Events: `onPressIn`, `onPressOut`, `onPress` (Button convenience — fires on PressOut).
