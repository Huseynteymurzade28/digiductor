//! Sprite pipeline: turns the Digi-API's mixed artwork into one consistent look.
//!
//! The API serves whatever art the wiki has — some Digimon get full-colour
//! illustrations, others black-and-white manga line art, all on a white
//! background. Rendered side by side that looks inconsistent, so every sprite is
//! pushed through the same pipeline before it reaches the terminal:
//!
//! 1. **Cut out** — the white background is flood-filled from the edges and made
//!    transparent (interior whites such as eyes or wings are kept).
//! 2. **Frame** — crop to the silhouette and centre it on a square canvas, so
//!    every Digimon is the same size regardless of the source framing.
//! 3. **Pixelate** — downsample to a small fixed grid with a hard alpha edge.
//! 4. **Colourise** — line art (near-grayscale) is gradient-mapped onto the
//!    Digimon's attribute colour, so it sits next to coloured sprites naturally.
//! 5. **Posterize + outline** — a reduced palette and a 1px glow outline give
//!    the handheld "game sprite" feel.
//!
//! Finally the grid is upscaled with nearest-neighbour so terminals with a real
//! graphics protocol show crisp pixels instead of a blurry upscale.

use image::imageops::FilterType;
use image::{DynamicImage, Rgba, RgbaImage};

/// Side length of the pixel grid every sprite is reduced to.
const GRID: u32 = 56;
/// Nearest-neighbour upscale factor applied after pixelation.
const UPSCALE: u32 = 6;
/// Channel value above which a border-connected pixel counts as background.
const BG_THRESHOLD: u8 = 232;
/// Mean saturation (0–255) below which art is treated as line art.
const LINE_ART_SATURATION: f32 = 18.0;
/// Mean luminance (0–255) above which colourless art counts as line art.
const LINE_ART_MIN_LUMA: f32 = 120.0;

/// How sprites are drawn. Cycled with `p`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteStyle {
    /// Uniform pixel sprites; line art is tinted by attribute. The default.
    Pixel,
    /// Monochrome hologram in the attribute colour, with scanlines.
    Holo,
    /// The untouched source artwork.
    Original,
}

impl SpriteStyle {
    pub fn cycle(self) -> Self {
        match self {
            Self::Pixel => Self::Holo,
            Self::Holo => Self::Original,
            Self::Original => Self::Pixel,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pixel => "PIXEL",
            Self::Holo => "HOLO",
            Self::Original => "ORIGINAL",
        }
    }
}

/// Apply `style` to `src`, tinting with `tint` (the attribute colour) where the
/// style calls for it.
pub fn stylize(src: &DynamicImage, style: SpriteStyle, tint: [u8; 3]) -> DynamicImage {
    if style == SpriteStyle::Original {
        return src.clone();
    }

    let mut img = src.to_rgba8();
    cut_out_background(&mut img);
    let framed = frame_square(&img);
    let mut grid = pixelate(&framed);

    match style {
        SpriteStyle::Pixel => {
            if is_line_art(&grid) {
                gradient_map(&mut grid, tint);
            }
            posterize(&mut grid);
        }
        SpriteStyle::Holo => {
            gradient_map(&mut grid, tint);
            posterize(&mut grid);
            scanlines(&mut grid);
        }
        SpriteStyle::Original => unreachable!(),
    }
    outline(&mut grid, scale(tint, 0.55));

    DynamicImage::ImageRgba8(image::imageops::resize(
        &grid,
        GRID * UPSCALE,
        GRID * UPSCALE,
        FilterType::Nearest,
    ))
}

// ---------------------------------------------------------------------------
// Pipeline stages
// ---------------------------------------------------------------------------

/// Flood-fill near-white pixels reachable from the image border and make them
/// transparent. Already-transparent pixels count as background too.
fn cut_out_background(img: &mut RgbaImage) {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return;
    }
    let is_bg = |p: &Rgba<u8>| {
        p[3] < 16 || (p[0] >= BG_THRESHOLD && p[1] >= BG_THRESHOLD && p[2] >= BG_THRESHOLD)
    };

    let mut seen = vec![false; (w * h) as usize];
    let mut stack: Vec<(u32, u32)> = Vec::new();
    for x in 0..w {
        stack.push((x, 0));
        stack.push((x, h - 1));
    }
    for y in 0..h {
        stack.push((0, y));
        stack.push((w - 1, y));
    }

    while let Some((x, y)) = stack.pop() {
        let idx = (y * w + x) as usize;
        if seen[idx] {
            continue;
        }
        seen[idx] = true;
        if !is_bg(img.get_pixel(x, y)) {
            continue;
        }
        img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
}

/// Crop to the opaque bounding box and centre it on a square transparent canvas
/// with a small margin (room for the outline).
fn frame_square(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 0 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x0 > x1 || y0 > y1 {
        return img.clone(); // fully transparent — nothing to frame
    }

    let cw = x1 - x0 + 1;
    let ch = y1 - y0 + 1;
    let side = cw.max(ch);
    let margin = side / 16 + 1;
    let canvas_side = side + margin * 2;

    let mut canvas = RgbaImage::from_pixel(canvas_side, canvas_side, Rgba([0, 0, 0, 0]));
    let ox = margin + (side - cw) / 2;
    let oy = margin + (side - ch) / 2;
    for y in 0..ch {
        for x in 0..cw {
            canvas.put_pixel(ox + x, oy + y, *img.get_pixel(x0 + x, y0 + y));
        }
    }
    canvas
}

/// Downsample to the fixed grid using premultiplied-alpha averaging, then snap
/// alpha to fully opaque / fully transparent for a hard pixel-art edge.
fn pixelate(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(GRID, GRID);
    for gy in 0..GRID {
        for gx in 0..GRID {
            let sx0 = gx * w / GRID;
            let sx1 = ((gx + 1) * w / GRID).max(sx0 + 1).min(w);
            let sy0 = gy * h / GRID;
            let sy1 = ((gy + 1) * h / GRID).max(sy0 + 1).min(h);

            let (mut r, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for y in sy0..sy1 {
                for x in sx0..sx1 {
                    let p = img.get_pixel(x, y);
                    let pa = p[3] as u64;
                    r += p[0] as u64 * pa;
                    g += p[1] as u64 * pa;
                    b += p[2] as u64 * pa;
                    a += pa;
                    n += 1;
                }
            }
            let coverage = a.checked_div(n).unwrap_or(0);
            let px = if coverage >= 110 && a > 0 {
                Rgba([(r / a) as u8, (g / a) as u8, (b / a) as u8, 255])
            } else {
                Rgba([0, 0, 0, 0])
            };
            out.put_pixel(gx, gy, px);
        }
    }
    out
}

/// True when the opaque pixels are (nearly) colourless *and* mostly light —
/// manga line art. Dark colourless art (e.g. a black Digimon like Botamon) is
/// left alone, since its colour is genuinely black.
fn is_line_art(img: &RgbaImage) -> bool {
    let (mut sat, mut lum, mut n) = (0f32, 0f32, 0f32);
    for p in img.pixels().filter(|p| p[3] > 0) {
        let max = p[0].max(p[1]).max(p[2]) as f32;
        let min = p[0].min(p[1]).min(p[2]) as f32;
        sat += max - min;
        lum += 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
        n += 1.0;
    }
    n > 0.0 && sat / n < LINE_ART_SATURATION && lum / n > LINE_ART_MIN_LUMA
}

/// Map luminance onto a dark → tint → near-white ramp.
fn gradient_map(img: &mut RgbaImage, tint: [u8; 3]) {
    let shadow = scale(tint, 0.18);
    let highlight = mix(tint, [255, 255, 255], 0.75);
    for p in img.pixels_mut().filter(|p| p[3] > 0) {
        let l = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0;
        let c = if l < 0.5 {
            mix(shadow, tint, l * 2.0)
        } else {
            mix(tint, highlight, (l - 0.5) * 2.0)
        };
        *p = Rgba([c[0], c[1], c[2], 255]);
    }
}

/// Reduce each channel to a handful of levels for a limited-palette look.
fn posterize(img: &mut RgbaImage) {
    const STEP: f32 = 255.0 / 5.0; // 6 levels per channel
    for p in img.pixels_mut().filter(|p| p[3] > 0) {
        for c in 0..3 {
            p[c] = ((p[c] as f32 / STEP).round() * STEP) as u8;
        }
    }
}

/// Darken every other row for a CRT / hologram feel.
fn scanlines(img: &mut RgbaImage) {
    for (_, y, p) in img.enumerate_pixels_mut() {
        if y % 2 == 1 && p[3] > 0 {
            for c in 0..3 {
                p[c] = (p[c] as f32 * 0.6) as u8;
            }
        }
    }
}

/// Paint transparent pixels that touch the silhouette with `color`.
fn outline(img: &mut RgbaImage, color: [u8; 3]) {
    let (w, h) = img.dimensions();
    let src = img.clone();
    let opaque = |x: i64, y: i64| {
        x >= 0 && y >= 0 && x < w as i64 && y < h as i64 && src.get_pixel(x as u32, y as u32)[3] > 0
    };
    for y in 0..h {
        for x in 0..w {
            if src.get_pixel(x, y)[3] > 0 {
                continue;
            }
            let (xi, yi) = (x as i64, y as i64);
            if opaque(xi - 1, yi) || opaque(xi + 1, yi) || opaque(xi, yi - 1) || opaque(xi, yi + 1)
            {
                img.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

fn scale(c: [u8; 3], f: f32) -> [u8; 3] {
    [
        (c[0] as f32 * f) as u8,
        (c[1] as f32 * f) as u8,
        (c[2] as f32 * f) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A white canvas with a black ring: the outside should become transparent
    /// while the white inside the ring is preserved.
    #[test]
    fn cut_out_keeps_enclosed_white() {
        let mut img = RgbaImage::from_pixel(20, 20, Rgba([255, 255, 255, 255]));
        for i in 5..15 {
            for &(x, y) in &[(i, 5), (i, 14), (5, i), (14, i)] {
                img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        cut_out_background(&mut img);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(10, 10)[3], 255);
        assert_eq!(img.get_pixel(5, 5)[3], 255);
    }

    #[test]
    fn grayscale_is_line_art_colour_is_not() {
        let gray = RgbaImage::from_pixel(4, 4, Rgba([200, 200, 200, 255]));
        let black = RgbaImage::from_pixel(4, 4, Rgba([20, 20, 20, 255]));
        assert!(!is_line_art(&black));
        let orange = RgbaImage::from_pixel(4, 4, Rgba([240, 150, 20, 255]));
        assert!(is_line_art(&gray));
        assert!(!is_line_art(&orange));
    }

    #[test]
    fn output_is_uniform_size() {
        let a = DynamicImage::ImageRgba8(RgbaImage::from_pixel(320, 320, Rgba([200, 30, 30, 255])));
        let b = DynamicImage::ImageRgba8(RgbaImage::from_pixel(254, 320, Rgba([30, 30, 30, 255])));
        for img in [a, b] {
            let out = stylize(&img, SpriteStyle::Pixel, [0, 132, 255]);
            assert_eq!(
                (out.width(), out.height()),
                (GRID * UPSCALE, GRID * UPSCALE)
            );
        }
    }
}
