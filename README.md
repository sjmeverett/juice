# juice

Javascript User Interface for Compact Electronics.

A UI engine for embedded Linux displays. Runs Preact inside QuickJS, uses a lightweight fake DOM that serializes to JSON via `toJSON()`, then lays it out with Taffy and renders to a framebuffer using fontdue.

## Architecture

```
┌─────────────────────────────────────────────────┐
│  TypeScript / Preact                            │
│  Fake DOM → toJSON() → renderer.update(json)    │
├─────────────────────────────────────────────────┤
│  QuickJS (via rquickjs)                         │
│  JS ←→ Rust function bridge                     │
├─────────────────────────────────────────────────┤
│  juice lib crate                                │
│  DOM → Taffy layout → Canvas                    │
├─────────────────────────────────────────────────┤
│  Display target (DrawTarget<Color = Rgb888>)    │
│  simulator: SDL2 window                         │
│  embedded: DRM/KMS + evdev touchscreen          │
└─────────────────────────────────────────────────┘
```

## Packages

| Package | Description |
|---------|-------------|
| `packages/juice` | Core TypeScript library: fake DOM classes (`UINode`, `UIElement`, `UITextNode`, `UIDocument`), event system, Preact integration, and `render()` entrypoint |
| `packages/app` | Example Preact application |

## Crates

| Crate | Type | Description |
|-------|------|-------------|
| `crates/juice` | lib | Core engine: QuickJS runtime, DOM parsing, Taffy layout, canvas rendering |
| `crates/juice-dev` | lib | Dev server for hot-reloading JS bundles |
| `crates/simulator` | bin | Desktop simulator using embedded-graphics-simulator (SDL2) |
| `crates/embedded` | bin | Embedded Linux target using DRM/KMS display + evdev touch input |

### juice lib modules

| Module | Description |
|--------|-------------|
| `engine` | Thin wrapper around QuickJS `Runtime` + `Context` |
| `timers` | `setTimeout`/`clearTimeout`/`setInterval`/`clearInterval` implementation |
| `dom` | Deserializes the JSON DOM tree and computes Taffy layout |
| `canvas` | XRGB8888 software framebuffer with text rendering (fontdue) and `DrawTarget` impl |
| `renderer` | High-level orchestrator: owns the engine, canvas, DOM, fonts, and handles events |
| `inherited_style` | CSS-like style inheritance (color, font, fontSize) |

## Quick start

```sh
npm install
npm run build          # builds packages/ → dist/bundle.js
cargo run -p simulator # opens SDL2 window
```

## JS ↔ Rust bridge

### Renderer setup

Create a `Renderer` with a setup closure for registering native globals, a canvas, fonts, and a base inherited style:

```rust
use juice::{canvas::{Canvas, RgbColor}, inherited_style::InheritedStyle, renderer::Renderer};

let mut renderer = Renderer::new(
    |ctx| {
        // Register native globals (e.g. console.log)
        let console = Object::new(ctx.clone()).unwrap();
        console.set("log", Func::from(|msg: String| println!("[JS] {}", msg))).unwrap();
        ctx.globals().set("console", console).unwrap();
    },
    Canvas::new(width, height),
    fonts,
    InheritedStyle {
        color: RgbColor::from_array([255, 255, 255]),
        font_name: "Roboto-Regular".to_string(),
        font_size: 24.0,
    },
);

renderer.engine.load(&bundle);
```

Then in your event loop:

```rust
loop {
    renderer.tick();       // fire expired timers
    renderer.render();     // re-render if the DOM changed
    display.blit_from(&renderer.canvas);

    // dispatch touch/mouse events
    renderer.press_event(x, y, EventName::PressIn);
    renderer.press_event(x, y, EventName::PressOut);
}
```

### Registering native functions

Use `rquickjs::function::Func` inside the setup closure:

```rust
Renderer::new(|ctx| {
    ctx.globals()
        .set("myFunction", Func::from(|msg: String| {
            println!("{}", msg);
        }))
        .unwrap();
}, ...);
```

Supported argument/return types: `String`, `f64`, `i32`, `bool`, `()`, `Option<T>`.

For stateful closures (FnMut), wrap with `MutFn`:

```rust
use rquickjs::function::{Func, MutFn};

let mut count = 0u32;
ctx.globals()
    .set("increment", Func::from(MutFn::from(move || -> u32 {
        count += 1;
        count
    })))
    .unwrap();
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

Register in the setup closure:

```rust
Renderer::new(|ctx| {
    Class::<File>::define(&ctx.globals()).unwrap();
}, ...);
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

Declare native globals in `packages/juice/src/render.ts` or a separate `.d.ts`:

```typescript
declare global {
    const renderer: UIRenderer;
}
```

The `renderer` object is registered on the Rust side by `Renderer` and exposes a single method:

```js
renderer.update(json, eventCallback) // sends serialized DOM to Rust, registers event callback
```

## Hot reloading

The `juice` CLI watches for TypeScript changes, rebuilds with esbuild, and pushes the new bundle to the running app over WebSocket.

In one terminal, start the dev server:

```sh
npx juice dev <entrypoint> [--port <port>]

# e.g.
npx juice dev packages/app/src/index.tsx
npx juice dev src/index.tsx --port 4000
```

In another, run the simulator with the `DEV_SERVER` env var pointing at the WebSocket:

```sh
DEV_SERVER=ws://localhost:3000 cargo run -p simulator
```

On each rebuild the dev server broadcasts the new bundle. The Rust side (`juice-dev` crate) connects via WebSocket on a background thread and the `Renderer` re-creates the JS engine with the new bundle, preserving the canvas and fonts.

For the embedded target, use the `hotreload` feature:

```sh
DEV_SERVER=ws://YOUR_HOST:3000 cargo run -p embedded --features hotreload
```

## Cross-compilation

For ARM targets:

```sh
cross build -p embedded --release
```

Requires `cross` installed. The `Dockerfile.cross` installs `libclang-dev` for rquickjs bindgen.

Deploy to device: copy the binary + `dist/bundle.js` + `assets/` directory.

## Components (TypeScript)

The `Box` component is the fundamental building block. All layout is flexbox-based via Taffy.

```tsx
import { Box, render } from "@juice/core";

render(
    <Box style={{ flexDirection: "column", padding: 40, gap: 10, background: "#1a1a2e" }}>
        <Box style={{ color: "#ffffff", font: "Roboto-Bold", fontSize: 72 }}>
            Hello, World
        </Box>
        <Box
            onPress={() => console.log("pressed!")}
            style={{ padding: 20, background: "#ff8000", borderRadius: 5 }}
        >
            Click me
        </Box>
    </Box>
);
```

### Supported style properties

| Property | Type | Description |
|----------|------|-------------|
| `alignItems` | `"stretch" \| "flex-start" \| "center" \| "flex-end"` | Cross-axis alignment of children |
| `alignSelf` | `"stretch" \| "flex-start" \| "center" \| "flex-end"` | Cross-axis alignment override for this element |
| `background` | `string` (hex) | Background color |
| `borderRadius` | `number` | Corner radius in pixels |
| `color` | `string` (hex) | Text color (inherited) |
| `flexDirection` | `"row" \| "column"` | Main axis direction |
| `flexGrow` | `number` | Flex grow factor |
| `flexShrink` | `number` | Flex shrink factor |
| `font` | `string` | Font name matching a .ttf file in `assets/` (inherited) |
| `fontSize` | `number` | Font size in pixels (inherited) |
| `gap` | `number` | Gap between flex children |
| `width` / `height` | `number \| string` | Size in pixels or percent (e.g. `"50%"`) |
| `padding` | `number` | Padding (all sides) |
| `paddingX` / `paddingY` | `number` | Horizontal / vertical padding |
| `paddingTop/Right/Bottom/Left` | `number` | Per-side padding |
| `margin` | `number` | Margin (all sides) |
| `marginX` / `marginY` | `number` | Horizontal / vertical margin |
| `marginTop/Right/Bottom/Left` | `number` | Per-side margin |

### Events

| Event | Description |
|-------|-------------|
| `onPressIn` | Fired when a touch/click begins on the element |
| `onPressOut` | Fired when a touch/click ends on the element |
| `onPress` | Convenience event: fires on PressOut if the press started on the same element |
