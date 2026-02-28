use embedded_graphics::{
    pixelcolor::Rgb888, pixelcolor::RgbColor as _, prelude::*, primitives::Rectangle,
};
use fontdue::Font;

#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn from_array(rgb: [u8; 3]) -> Self {
        RgbColor {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }

    pub fn to_xrgb(self) -> u32 {
        to_xrgb(self.r, self.g, self.b)
    }
}

/// Pack r, g, b into a single XRGB8888 u32
#[inline(always)]
fn to_xrgb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// Software framebuffer stored in XRGB8888 format for zero-copy blit to DRM.
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u32>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;

        Self {
            width,
            height,
            pixels: vec![0xFF00_0000; size],
        }
    }

    pub fn clear(&mut self, color: RgbColor) {
        self.pixels.fill(color.to_xrgb());
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
        self.pixels[idx] = to_xrgb(r, g, b);
    }

    /// Returns the raw XRGB8888 pixel buffer for direct memcpy to display.
    pub fn as_xrgb_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, self.pixels.len() * 4)
        }
    }

    /// Flush all pixels through a DrawTarget (for simulator or other embedded-graphics displays).
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

    pub fn draw_text(
        &mut self,
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

                        let py =
                            start_y as i32 + ascent as i32 - metrics.ymin - metrics.height as i32
                                + row as i32;

                        self.blend_pixel(px, py, color, coverage);
                    }
                }
            }

            cursor_x += metrics.advance_width;
        }
    }

    /// Blit non-premultiplied RGBA pixels onto the canvas with alpha blending.
    pub fn blit_rgba(
        &mut self,
        data: &[u8],
        src_w: u32,
        src_h: u32,
        dst_x: i32,
        dst_y: i32,
    ) {
        for row in 0..src_h as i32 {
            let cy = dst_y + row;
            if cy < 0 || cy >= self.height as i32 {
                continue;
            }

            for col in 0..src_w as i32 {
                let cx = dst_x + col;
                if cx < 0 || cx >= self.width as i32 {
                    continue;
                }

                let si = ((row as u32 * src_w + col as u32) * 4) as usize;
                let a = data[si + 3];

                if a == 0 {
                    continue;
                }

                let r = data[si];
                let g = data[si + 1];
                let b = data[si + 2];

                let di = (cy as u32 * self.width + cx as u32) as usize;

                if a == 255 {
                    self.pixels[di] = to_xrgb(r, g, b);
                } else {
                    let bg = self.pixels[di];
                    let alpha = a as u16;
                    let inv_a = 255 - alpha;
                    let nr = ((r as u16 * alpha + ((bg >> 16) & 0xFF) as u16 * inv_a) / 255) as u8;
                    let ng = ((g as u16 * alpha + ((bg >> 8) & 0xFF) as u16 * inv_a) / 255) as u8;
                    let nb = ((b as u16 * alpha + (bg & 0xFF) as u16 * inv_a) / 255) as u8;
                    self.pixels[di] = to_xrgb(nr, ng, nb);
                }
            }
        }
    }

    /// Blit premultiplied RGBA pixels onto the canvas with alpha blending.
    pub fn blit_premultiplied_rgba(
        &mut self,
        data: &[u8],
        src_w: u32,
        src_h: u32,
        dst_x: i32,
        dst_y: i32,
    ) {
        for row in 0..src_h as i32 {
            let cy = dst_y + row;
            if cy < 0 || cy >= self.height as i32 {
                continue;
            }

            for col in 0..src_w as i32 {
                let cx = dst_x + col;
                if cx < 0 || cx >= self.width as i32 {
                    continue;
                }

                let si = ((row as u32 * src_w + col as u32) * 4) as usize;
                let a = data[si + 3];

                if a == 0 {
                    continue;
                }

                let di = (cy as u32 * self.width + cx as u32) as usize;

                if a == 255 {
                    self.pixels[di] = to_xrgb(data[si], data[si + 1], data[si + 2]);
                } else {
                    // src is premultiplied: out = src + dst * (1 - src_alpha/255)
                    let bg = self.pixels[di];
                    let inv_a = 255 - a as u16;
                    let r = (data[si] as u16 + (((bg >> 16) & 0xFF) as u16 * inv_a + 127) / 255) as u8;
                    let g = (data[si + 1] as u16 + (((bg >> 8) & 0xFF) as u16 * inv_a + 127) / 255) as u8;
                    let b = (data[si + 2] as u16 + ((bg & 0xFF) as u16 * inv_a + 127) / 255) as u8;
                    self.pixels[di] = to_xrgb(r, g, b);
                }
            }
        }
    }
}

impl DrawTarget for Canvas {
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
                    to_xrgb(color.r(), color.g(), color.b());
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let px = to_xrgb(color.r(), color.g(), color.b());
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
        self.pixels.fill(to_xrgb(color.r(), color.g(), color.b()));
        Ok(())
    }
}

impl OriginDimensions for Canvas {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}
