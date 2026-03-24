use anyhow::Result;
use image::{ImageBuffer, Rgba};
use std::ffi::c_void;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use egui_mcp_protocol::messages::Rect;

pub fn capture_screen(
    region: Option<Rect>,
) -> Result<egui_mcp_protocol::messages::ScreenshotResponse> {
    unsafe {
        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        let hdc_mem = CreateCompatibleDC(hdc_screen);

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

        let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        SelectObject(hdc_mem, hbitmap);

        let _ = BitBlt(
            hdc_mem, 0, 0, width, height, hdc_screen, rect.left, rect.top, SRCCOPY,
        );

        let mut image = ImageBuffer::new(width as u32, height as u32);
        get_bitmap_data(hbitmap, &mut image)?;

        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        // Convert to RGB bytes
        let mut rgb_data = Vec::with_capacity(width as usize * height as usize * 3);
        for pixel in image.pixels() {
            rgb_data.push(pixel[0]); // Red
            rgb_data.push(pixel[1]); // Green
            rgb_data.push(pixel[2]); // Blue
        }

        Ok(egui_mcp_protocol::messages::ScreenshotResponse {
            image_data: rgb_data,
            width: width as u32,
            height: height as u32,
        })
    }
}

unsafe fn get_bitmap_data(
    hbitmap: HBITMAP,
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> Result<()> {
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: image.width() as i32,
            biHeight: -(image.height() as i32), // Top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB is 0
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let mut bits: *mut u8 = std::ptr::null_mut();
    GetDIBits(
        GetDC(HWND(std::ptr::null_mut())),
        hbitmap,
        0,
        image.height() as u32,
        Some(std::ptr::addr_of_mut!(bits) as *mut c_void),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    if bits.is_null() {
        return Err(anyhow::anyhow!("Failed to get bitmap bits"));
    }

    let bytes_per_pixel = 4;
    let stride = image.width() as i32 * bytes_per_pixel;

    for y in 0..image.height() {
        let row_ptr = bits.offset(y as isize * stride as isize);
        for x in 0..image.width() {
            let pixel_ptr = row_ptr.offset(x as isize * bytes_per_pixel as isize);
            let bgra_ptr = pixel_ptr as *const [u8; 4];
            let bgra = bgra_ptr.read_unaligned();
            let rgba = Rgba([
                bgra[2], // Red
                bgra[1], // Green
                bgra[0], // Blue
                bgra[3], // Alpha
            ]);
            image.put_pixel(x, y, rgba);
        }
    }

    Ok(())
}
