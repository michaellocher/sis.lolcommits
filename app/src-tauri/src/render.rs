//! Meme captions, the blurred band for sharing, contact sheets – all in-process,
//! so the app needs neither ffmpeg nor ImageMagick.
use ab_glyph::{FontRef, PxScale};
use image::codecs::jpeg::JpegEncoder;
use image::{imageops, Rgb, RgbImage};
use imageproc::drawing::{draw_text_mut, text_size};
use std::path::Path;

pub const W: u32 = 1280;
pub const H: u32 = 720;

static FONT_BYTES: &[u8] = include_bytes!("../fonts/Anton-Regular.ttf");

fn font() -> FontRef<'static> {
    FontRef::try_from_slice(FONT_BYTES).expect("bundled font")
}

/// Scale to cover 1280x720 and crop the middle, like the CLI.
pub fn fit(img: RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    let s = (W as f32 / w as f32).max(H as f32 / h as f32);
    let (nw, nh) = (((w as f32) * s).ceil() as u32, ((h as f32) * s).ceil() as u32);
    let scaled = imageops::resize(&img, nw.max(W), nh.max(H), imageops::FilterType::Triangle);
    imageops::crop_imm(&scaled, (scaled.width() - W) / 2, (scaled.height() - H) / 2, W, H).to_image()
}

/// Word-wrap at about `width` characters, at most `max` lines.
fn wrap(text: &str, width: usize, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.len() > max {
        lines.truncate(max);
        if let Some(l) = lines.last_mut() {
            l.push('…');
        }
    }
    lines
}

/// White text with a thick black outline.
fn outlined(img: &mut RgbImage, x: i32, y: i32, size: f32, stroke: i32, text: &str) {
    let f = font();
    let scale = PxScale::from(size);
    for dx in -stroke..=stroke {
        for dy in -stroke..=stroke {
            if dx * dx + dy * dy <= stroke * stroke + 1 && (dx, dy) != (0, 0) {
                draw_text_mut(img, Rgb([0, 0, 0]), x + dx, y + dy, scale, &f, text);
            }
        }
    }
    draw_text_mut(img, Rgb([255, 255, 255]), x, y, scale, &f, text);
}

fn measure(size: f32, text: &str) -> (i32, i32) {
    let (w, h) = text_size(PxScale::from(size), &font(), text);
    (w as i32, h as i32)
}

pub fn caption(mut img: RgbImage, sha: &str, subject: &str, repo: &str, branch: &str) -> RgbImage {
    let (w, h) = (img.width() as i32, img.height() as i32);
    // commit message: big, centered, bottom
    let size = 72.0;
    let lines = wrap(subject, 30, 4);
    let line_h = (size * 1.12) as i32;
    let mut y = h - 40 - line_h * lines.len() as i32;
    for l in &lines {
        let (tw, _) = measure(size, l);
        outlined(&mut img, (w - tw) / 2, y, size, 4, l);
        y += line_h;
    }
    // sha top left, repo · branch top right
    outlined(&mut img, 32, 22, 52.0, 3, sha);
    let rb = format!("{repo} · {branch}");
    let (tw, _) = measure(30.0, &rb);
    outlined(&mut img, w - tw - 32, 34, 30.0, 2, &rb);
    img
}

/// For sharing: blur the sha / repo band at the top.
pub fn blur_band(img: &RgbImage) -> RgbImage {
    let mut out = img.clone();
    let band = imageops::crop_imm(img, 0, 0, img.width(), 110.min(img.height())).to_image();
    let blurred = imageops::blur(&band, 14.0);
    imageops::replace(&mut out, &blurred, 0, 0);
    out
}

/// JPEG without EXIF or comments.
pub fn save_jpeg(img: &RgbImage, path: &Path) -> Result<(), String> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    {
        let f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = std::io::BufWriter::new(f);
        JpegEncoder::new_with_quality(&mut w, 88).encode_image(img).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn load(path: &Path) -> Result<RgbImage, String> {
    image::open(path).map(|i| i.to_rgb8()).map_err(|e| format!("{}: {e}", path.display()))
}

/// Colour bars for tests without a camera (LOLCOMMITS_TEST_PATTERN=1).
pub fn test_pattern() -> RgbImage {
    let cols = [[230, 57, 70], [255, 214, 10], [42, 157, 143], [69, 123, 157], [155, 93, 229], [241, 91, 181]];
    let t = chrono::Local::now().timestamp_subsec_millis() as u32;
    RgbImage::from_fn(W, H, |x, y| {
        let c = cols[((x + t) / (W / 6) % 6) as usize];
        let shade = 0.75 + 0.25 * (y as f32 / H as f32);
        Rgb([(c[0] as f32 * shade) as u8, (c[1] as f32 * shade) as u8, (c[2] as f32 * shade) as u8])
    })
}

/// Contact sheet of up to 144 pictures, 320x180 tiles.
pub fn contact_sheet(files: &[String]) -> Result<RgbImage, String> {
    let n = files.len().min(144);
    if n == 0 {
        return Err("no pictures".into());
    }
    let mut cols = 1;
    while cols * cols < n {
        cols += 1;
    }
    let cols = cols.min(12) as u32;
    let rows = ((n as u32) + cols - 1) / cols;
    let (tw, th, pad, margin) = (320u32, 180u32, 4u32, 8u32);
    let mut sheet = RgbImage::from_pixel(margin * 2 + cols * tw + (cols - 1) * pad, margin * 2 + rows * th + (rows - 1) * pad, Rgb([0, 0, 0]));
    for (i, f) in files.iter().take(n).enumerate() {
        if let Ok(img) = load(Path::new(f)) {
            let t = imageops::resize(&fit(img), tw, th, imageops::FilterType::Triangle);
            let (c, r) = (i as u32 % cols, i as u32 / cols);
            imageops::replace(&mut sheet, &t, (margin + c * (tw + pad)) as i64, (margin + r * (th + pad)) as i64);
        }
    }
    Ok(sheet)
}
