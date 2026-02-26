use crate::layout::{LayoutTree, NodeContext, RgbColor};
use embedded_graphics::pixelcolor::{Rgb888, RgbColor as _};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle,
};
use fontdue::Font;
use std::collections::HashMap;
use taffy::NodeId;
use taffy::TaffyTree;

/// Pack r, g, b into a single XRGB8888 u32 (little-endian: 0xFF_RR_GG_BB).
#[inline(always)]
fn pack_xrgb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// Software framebuffer stored in XRGB8888 format for zero-copy blit to DRM.
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Framebuffer {
            width,
            height,
            pixels: vec![0xFF00_0000; size],
        }
    }

    pub fn clear(&mut self, color: RgbColor) {
        self.pixels.fill(pack_xrgb(color.r, color.g, color.b));
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, color: RgbColor, alpha: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        let bg = self.pixels[idx];
        let bg_r = ((bg >> 16) & 0xFF) as u16;
        let bg_g = ((bg >> 8) & 0xFF) as u16;
        let bg_b = (bg & 0xFF) as u16;
        let a = alpha as u16;
        let inv_a = 255 - a;
        let r = ((color.r as u16 * a + bg_r * inv_a) / 255) as u8;
        let g = ((color.g as u16 * a + bg_g * inv_a) / 255) as u8;
        let b = ((color.b as u16 * a + bg_b * inv_a) / 255) as u8;
        self.pixels[idx] = pack_xrgb(r, g, b);
    }

    /// Returns the raw XRGB8888 pixel buffer for direct memcpy to display.
    pub fn as_xrgb_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.pixels.as_ptr() as *const u8,
                self.pixels.len() * 4,
            )
        }
    }

    /// Flush all pixels through a DrawTarget (for simulator or other e-g displays).
    pub fn flush(&self, display: &mut impl DrawTarget<Color = Rgb888>) {
        for y in 0..self.height {
            for x in 0..self.width {
                let px = self.pixels[(y * self.width + x) as usize];
                let _ = Pixel(
                    Point::new(x as i32, y as i32),
                    Rgb888::new((px >> 16) as u8, (px >> 8) as u8, px as u8),
                )
                .draw(display);
            }
        }
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            let x = point.x;
            let y = point.y;
            if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                self.pixels[(y as u32 * self.width + x as u32) as usize] =
                    pack_xrgb(color.r(), color.g(), color.b());
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let px = pack_xrgb(color.r(), color.g(), color.b());
        let clipped = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if let Some(bottom_right) = clipped.bottom_right() {
            let x0 = clipped.top_left.x as u32;
            let y0 = clipped.top_left.y as u32;
            let x1 = bottom_right.x as u32 + 1;
            let y1 = bottom_right.y as u32 + 1;
            for y in y0..y1 {
                let row_start = (y * self.width + x0) as usize;
                let row_end = (y * self.width + x1) as usize;
                self.pixels[row_start..row_end].fill(px);
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.pixels.fill(pack_xrgb(color.r(), color.g(), color.b()));
        Ok(())
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
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
        Some(NodeContext::Container { background: Some(bg), border_radius, .. }) => {
            let color = Rgb888::new(bg.r, bg.g, bg.b);
            let style = PrimitiveStyle::with_fill(color);
            let rect = Rectangle::new(
                Point::new(x as i32, y as i32),
                Size::new(w as u32, h as u32),
            );
            if *border_radius > 0.0 {
                let r = *border_radius as u32;
                let _ = RoundedRectangle::new(rect, CornerRadii::new(Size::new(r, r)))
                    .into_styled(style)
                    .draw(fb);
            } else {
                let _ = rect.into_styled(style).draw(fb);
            }
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
