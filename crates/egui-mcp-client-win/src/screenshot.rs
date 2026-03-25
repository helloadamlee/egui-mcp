use anyhow::{anyhow, Result};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use std::ffi::c_void;
use std::io::Cursor;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use egui_mcp_protocol::messages::Rect;

pub fn capture_screen(
    region: Option<Rect>,
) -> Result<egui_mcp_protocol::messages::ScreenshotResponse> {
    unsafe {
        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        if hdc_screen.0.is_null() {
            return Err(anyhow!("Failed to acquire screen device context"));
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.0.is_null() {
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err(anyhow!("Failed to create compatible device context"));
        }

        let rect = if let Some(region) = region {
            RECT {
                left: region.x,
                top: region.y,
                right: region.x + region.width,
                bottom: region.y + region.height,
            }
        } else {
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            RECT {
                left: 0,
                top: 0,
                right: screen_width,
                bottom: screen_height,
            }
        };

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err(anyhow!(
                "Screenshot bounds must have positive width and height"
            ));
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Top-down DIB for direct row order
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default(); 1],
        };

        let mut dib_bits: *mut c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            hdc_screen,
            &bmi,
            DIB_RGB_COLORS,
            std::ptr::addr_of_mut!(dib_bits),
            None,
            0,
        )
        .map_err(|err| anyhow!("CreateDIBSection failed: {}", err))?;

        if hbitmap.0.is_null() || dib_bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err(anyhow!("Failed to create DIB section for screenshot capture"));
        }

        let previous_obj = SelectObject(hdc_mem, hbitmap);
        if previous_obj.0.is_null() {
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err(anyhow!("Failed to select DIB section into memory device context"));
        }

        BitBlt(
            hdc_mem, 0, 0, width, height, hdc_screen, rect.left, rect.top, SRCCOPY,
        )
        .map_err(|err| anyhow!("BitBlt failed while capturing screenshot: {}", err))
        .inspect_err(|_| {
            let _ = SelectObject(hdc_mem, previous_obj);
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
        })?;

        let mut image = ImageBuffer::new(width as u32, height as u32);
        let raw_len = (width as usize) * (height as usize) * 4;
        let raw_bgra = std::slice::from_raw_parts(dib_bits as *const u8, raw_len);

        for y in 0..(height as usize) {
            let row_offset = y * (width as usize) * 4;
            for x in 0..(width as usize) {
                let offset = row_offset + x * 4;
                let b = raw_bgra[offset];
                let g = raw_bgra[offset + 1];
                let r = raw_bgra[offset + 2];
                let a = raw_bgra[offset + 3];
                image.put_pixel(x as u32, y as u32, Rgba([r, g, b, a]));
            }
        }

        let _ = SelectObject(hdc_mem, previous_obj);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        let mut cursor = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|err| anyhow!("Failed to encode screenshot PNG: {}", err))?;

        Ok(egui_mcp_protocol::messages::ScreenshotResponse {
            image_data: cursor.into_inner(),
            width: width as u32,
            height: height as u32,
        })
    }
}
