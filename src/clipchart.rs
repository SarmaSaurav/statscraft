//! Put a chart on the clipboard as a PNG and, on Windows, as an Office chart.
//!
//! Both formats are set in one clipboard open. Word and PowerPoint take
//! `Art::GVML ClipFormat` and paste a native chart. Everything else takes
//! the PNG.

use crate::output::Chart;

pub enum Copied {
    /// Office received a chart, and a PNG is there for every other program.
    OfficeAndPng,
    /// Only the picture. Box plots, and platforms other than Windows.
    Png,
}

pub fn copy(chart: &Chart) -> Result<Copied, String> {
    let (w, h, pixels) = crate::figure::raster(chart, 1200, 750);
    let png = encode_png(w, h, &pixels)?;
    let office = crate::officechart::gvml(chart);
    #[cfg(windows)]
    {
        set_windows(&png, &pixels, w, h, office.as_deref())?;
        return Ok(if office.is_some() {
            Copied::OfficeAndPng
        } else {
            Copied::Png
        });
    }
    #[cfg(not(windows))]
    {
        let _ = office;
        let image = arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(pixels),
        };
        arboard::Clipboard::new()
            .and_then(|mut c| c.set_image(image))
            .map_err(|e| e.to_string())?;
        Ok(Copied::Png)
    }
}

fn encode_png(w: u32, h: u32, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(pixels).map_err(|e| e.to_string())?;
    drop(writer);
    Ok(out)
}

#[cfg(windows)]
fn set_windows(png: &[u8], rgba: &[u8], w: u32, h: u32, gvml: Option<&[u8]>) -> Result<(), String> {
    const CF_DIB: u32 = 8;

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("The clipboard is in use.".into());
        }
        EmptyClipboard();
        let png_fmt = register("PNG");
        if png_fmt != 0 {
            SetClipboardData(png_fmt, global(png));
        }
        SetClipboardData(CF_DIB, global(&dib(rgba, w, h)));
        if let Some(bytes) = gvml {
            let fmt = register("Art::GVML ClipFormat");
            if fmt != 0 {
                SetClipboardData(fmt, global(bytes));
            }
        }
        CloseClipboard();
    }
    Ok(())
}

#[cfg(windows)]
fn register(name: &str) -> u32 {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { RegisterClipboardFormatW(wide.as_ptr()) }
}

#[cfg(windows)]
fn global(bytes: &[u8]) -> *mut std::ffi::c_void {
    unsafe {
        let h = GlobalAlloc(0x0002, bytes.len());
        if h.is_null() {
            return h;
        }
        let p = GlobalLock(h);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
        GlobalUnlock(h);
        h
    }
}

/// A top-down 32-bit DIB (BI_RGB). Negative height stores rows top-down.
#[cfg(windows)]
fn dib(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let stride = w as usize * 4;
    let mut out = Vec::with_capacity(40 + stride * h as usize);
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(-(h as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(stride as u32 * h).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for px in rgba.chunks_exact(4) {
        out.push(px[2]);
        out.push(px[1]);
        out.push(px[0]);
        out.push(px[3]);
    }
    out
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(hwnd: *mut std::ffi::c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(format: u32, mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn RegisterClipboardFormatW(name: *const u16) -> u32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
    fn GlobalLock(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(mem: *mut std::ffi::c_void) -> i32;
}
