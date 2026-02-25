use crate::layout::{LayoutTree, NodeContext, RgbColor};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::Pixel;
use fontdue::Font;
use std::collections::HashMap;
use taffy::NodeId;
use taffy::TaffyTree;

/// Double-buffered software framebuffer with dirty-rect tracking.
/// Renders to the front buffer, then flushes only changed pixels to the display.
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<[u8; 3]>,
    prev: Vec<[u8; 3]>,
    full_redraw: bool,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Framebuffer {
            width,
            height,
            pixels: vec![[0, 0, 0]; size],
            prev: vec![[0, 0, 0]; size],
            full_redraw: true,
        }
    }

    pub fn clear(&mut self, color: RgbColor) {
        self.pixels.fill([color.r, color.g, color.b]);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: RgbColor) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x + w as i32) as u32).min(self.width);
        let y1 = ((y + h as i32) as u32).min(self.height);
        for py in y0..y1 {
            for px in x0..x1 {
                self.pixels[(py * self.width + px) as usize] = [color.r, color.g, color.b];
            }
        }
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, color: RgbColor, alpha: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        let bg = self.pixels[idx];
        let a = alpha as u16;
        let inv_a = 255 - a;
        self.pixels[idx] = [
            ((color.r as u16 * a + bg[0] as u16 * inv_a) / 255) as u8,
            ((color.g as u16 * a + bg[1] as u16 * inv_a) / 255) as u8,
            ((color.b as u16 * a + bg[2] as u16 * inv_a) / 255) as u8,
        ];
    }

    /// Flush changed pixels to the display, then snapshot for next frame.
    pub fn flush(&mut self, display: &mut impl DrawTarget<Color = Rgb888>) {
        let full = self.full_redraw;
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                if full || self.pixels[idx] != self.prev[idx] {
                    let px = self.pixels[idx];
                    let _ = Pixel(
                        Point::new(x as i32, y as i32),
                        Rgb888::new(px[0], px[1], px[2]),
                    )
                    .draw(display);
                }
            }
        }
        self.prev.copy_from_slice(&self.pixels);
        self.full_redraw = false;
    }
}

pub fn render_tree(
    fb: &mut Framebuffer,
    layout_tree: &LayoutTree,
    fonts: &HashMap<String, Font>,
) {
    render_node(fb, &layout_tree.taffy, layout_tree.root, fonts, 0.0, 0.0);
}

fn render_node(
    fb: &mut Framebuffer,
    taffy: &TaffyTree<NodeContext>,
    node_id: NodeId,
    fonts: &HashMap<String, Font>,
    parent_x: f32,
    parent_y: f32,
) {
    let layout = taffy.layout(node_id).unwrap();
    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    let context = taffy.get_node_context(node_id);

    match context {
        Some(NodeContext::Container { background: Some(bg), .. }) => {
            fb.fill_rect(x as i32, y as i32, w as u32, h as u32, *bg);
        }
        Some(NodeContext::Text { content, color, font_name, font_size }) => {
            if let Some(font) = fonts.get(font_name) {
                draw_text(fb, font, content, *font_size, *color, x, y);
            }
        }
        _ => {}
    }

    if let Ok(children) = taffy.children(node_id) {
        for child_id in children {
            render_node(fb, taffy, child_id, fonts, x, y);
        }
    }
}

fn draw_text(
    fb: &mut Framebuffer,
    font: &Font,
    text: &str,
    font_size: f32,
    color: RgbColor,
    start_x: f32,
    start_y: f32,
) {
    let ascent = font
        .horizontal_line_metrics(font_size)
        .map(|m| m.ascent)
        .unwrap_or(font_size * 0.8);

    let mut cursor_x = start_x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + col];
                if coverage > 0 {
                    let px = cursor_x as i32 + metrics.xmin + col as i32;
                    let py = start_y as i32 + ascent as i32 - metrics.ymin
                        - metrics.height as i32
                        + row as i32;

                    fb.blend_pixel(px, py, color, coverage);
                }
            }
        }
        cursor_x += metrics.advance_width;
    }
}
