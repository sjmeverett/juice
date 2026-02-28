use drm::Device;
use drm::buffer::Buffer;
use drm::control::{Device as ControlDevice, Mode, connector, crtc, dumbbuffer, framebuffer};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use juice::canvas::Canvas;
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsFd, BorrowedFd};

pub struct DrmDisplay {
    file: File,
    #[allow(dead_code)]
    connector: connector::Handle,
    #[allow(dead_code)]
    crtc: crtc::Handle,
    #[allow(dead_code)]
    mode: Mode,
    fb: framebuffer::Handle,
    db: dumbbuffer::DumbBuffer,
    width: u32,
    height: u32,
    pitch: u32,
    buffer_ptr: *mut u8,
    buffer_size: usize,
}

impl AsFd for DrmDisplay {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl Device for DrmDisplay {}
impl ControlDevice for DrmDisplay {}

impl DrmDisplay {
    pub fn new(device_path: &str) -> Result<Self, String> {
        println!("Opening DRM device: {}", device_path);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(device_path)
            .map_err(|e| format!("Failed to open {}: {}", device_path, e))?;

        let drm = DrmDeviceInit { file };

        let res = drm
            .resource_handles()
            .map_err(|e| format!("Failed to get DRM resources: {}", e))?;

        println!(
            "Found {} connectors, {} CRTCs",
            res.connectors().len(),
            res.crtcs().len()
        );

        let (connector_handle, connector_info) = res
            .connectors()
            .iter()
            .find_map(|&conn| {
                let info = drm.get_connector(conn, false).ok()?;
                if info.state() == connector::State::Connected {
                    Some((conn, info))
                } else {
                    None
                }
            })
            .ok_or_else(|| "No connected display found".to_string())?;

        let mode = *connector_info
            .modes()
            .first()
            .ok_or_else(|| "No display modes found".to_string())?;

        let width = mode.size().0 as u32;
        let height = mode.size().1 as u32;
        println!("Display mode: {}x{}", width, height);

        let encoder = connector_info
            .current_encoder()
            .and_then(|enc| drm.get_encoder(enc).ok())
            .ok_or_else(|| "Failed to get encoder".to_string())?;

        let crtc = encoder
            .crtc()
            .ok_or_else(|| "No CRTC associated with encoder".to_string())?;

        // Create dumb buffer (XRGB8888 = 32 bpp)
        let mut db = drm
            .create_dumb_buffer((width, height), drm::buffer::DrmFourcc::Xrgb8888, 32)
            .map_err(|e| format!("Failed to create dumb buffer: {}", e))?;

        let pitch = db.pitch();
        let buffer_size = (pitch * height) as usize;

        println!(
            "Created dumb buffer: {}x{}, pitch={}, size={}",
            width, height, pitch, buffer_size
        );

        let fb = drm
            .add_framebuffer(&db, 24, 32)
            .map_err(|e| format!("Failed to add framebuffer: {}", e))?;

        // Map the buffer
        let mut map = drm
            .map_dumb_buffer(&mut db)
            .map_err(|e| format!("Failed to map dumb buffer: {}", e))?;

        let buffer_ptr = map.as_mut_ptr();

        // Set CRTC
        if let Err(e) = drm.set_crtc(crtc, Some(fb), (0, 0), &[connector_handle], Some(mode)) {
            println!("Warning: Failed to set CRTC: {}", e);
        } else {
            println!("Successfully set CRTC - display active");
        }

        // Forget the map so it doesn't get unmapped
        std::mem::forget(map);

        Ok(DrmDisplay {
            file: drm.file,
            connector: connector_handle,
            crtc,
            mode,
            fb,
            db,
            width,
            height,
            pitch,
            buffer_ptr,
            buffer_size,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn framebuffer_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer_ptr, self.buffer_size) }
    }

    /// Blit the framebuffer into the DRM display buffer.
    /// Both are XRGB8888, so this is a row-by-row memcpy.
    pub fn blit_from(&mut self, canvas: &Canvas) {
        let src = canvas.as_xrgb_bytes();
        let pitch = self.pitch as usize;
        let row_bytes = canvas.width as usize * 4;
        let dst = self.framebuffer_mut();

        // If pitch matches width (no padding), single memcpy for the whole buffer
        if pitch == row_bytes {
            dst[..src.len()].copy_from_slice(src);
        } else {
            for y in 0..canvas.height as usize {
                let src_start = y * row_bytes;
                let dst_start = y * pitch;
                dst[dst_start..dst_start + row_bytes]
                    .copy_from_slice(&src[src_start..src_start + row_bytes]);
            }
        }
    }
}

impl DrawTarget for DrmDisplay {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let pitch = self.pitch as usize;
        let w = self.width as i32;
        let h = self.height as i32;
        let fb = self.framebuffer_mut();

        for Pixel(point, color) in pixels {
            let x = point.x;
            let y = point.y;
            if x >= 0 && x < w && y >= 0 && y < h {
                let offset = (y as usize) * pitch + (x as usize) * 4;
                // XRGB8888: bytes are B, G, R, X
                fb[offset] = color.b();
                fb[offset + 1] = color.g();
                fb[offset + 2] = color.r();
                fb[offset + 3] = 0xFF;
            }
        }

        Ok(())
    }
}

impl OriginDimensions for DrmDisplay {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl Drop for DrmDisplay {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.buffer_ptr as *mut libc::c_void, self.buffer_size);
        }
        let _ = self.destroy_framebuffer(self.fb);
        let _ = self.destroy_dumb_buffer(self.db);
    }
}

struct DrmDeviceInit {
    file: File,
}

impl AsFd for DrmDeviceInit {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl Device for DrmDeviceInit {}
impl ControlDevice for DrmDeviceInit {}
