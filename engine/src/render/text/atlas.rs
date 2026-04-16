use super::types::{ATLAS_COLS, ATLAS_ROWS, FIRST_CHAR, Glyph, LAST_CHAR, NUM_GLYPHS, RASTER_PX};
use fontdue::{Font, FontSettings};

/// All data produced by rasterising the font into an R8 atlas.
pub(super) struct BuiltAtlas {
    pub glyphs: [Glyph; NUM_GLYPHS],
    pub cell_w: u32,
    pub atlas_w: u32,
    pub atlas_h: u32,
    pub ascent: f32,
    pub pixels: Vec<u8>,
}

/// Rasterise every printable ASCII glyph and pack them into a single R8 bitmap.
pub(super) fn build() -> BuiltAtlas {
    let font_bytes = load_font_bytes();
    let font = Font::from_bytes(font_bytes.as_slice(), FontSettings::default())
        .expect("failed to parse font");

    let raw: Vec<(fontdue::Metrics, Vec<u8>)> = (FIRST_CHAR..=LAST_CHAR)
        .map(|c| font.rasterize(c as char, RASTER_PX))
        .collect();

    let cell_w = raw.iter().map(|(m, _)| m.width as u32).max().unwrap_or(12) + 2;
    let cell_h = raw.iter().map(|(m, _)| m.height as u32).max().unwrap_or(20) + 2;
    let ascent = raw
        .iter()
        .map(|(m, _)| m.bounds.ymin + m.height as f32)
        .fold(0.0_f32, f32::max);

    let atlas_w = cell_w * ATLAS_COLS;
    let atlas_h = cell_h * ATLAS_ROWS;

    let mut pixels = vec![0u8; (atlas_w * atlas_h) as usize];
    let mut glyphs = [Glyph::default(); NUM_GLYPHS];

    for (i, (metrics, bitmap)) in raw.iter().enumerate() {
        let col = (i as u32) % ATLAS_COLS;
        let row = (i as u32) / ATLAS_COLS;
        let cx = col * cell_w;
        let cy = row * cell_h;
        let bx = cx + 1;
        let by = cy + 1;

        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let src = bitmap[y * metrics.width + x];
                let dst = (by + y as u32) * atlas_w + (bx + x as u32);
                pixels[dst as usize] = src;
            }
        }

        glyphs[i] = Glyph {
            cell_x: cx,
            cell_y: cy,
            bmp_w: metrics.width as u32,
            bmp_h: metrics.height as u32,
            bear_x: metrics.bounds.xmin,
            bear_y: metrics.bounds.ymin + metrics.height as f32,
            advance: metrics.advance_width,
        };
    }

    BuiltAtlas {
        glyphs,
        cell_w,
        atlas_w,
        atlas_h,
        ascent,
        pixels,
    }
}

/// Checks project assets then common system font paths.
fn load_font_bytes() -> Vec<u8> {
    let candidates: &[&str] = &[
        "assets/font.ttf",
        // macOS
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Monaco.ttf",
        // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        // Windows
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/calibri.ttf",
    ];
    for &path in candidates {
        if let Ok(data) = std::fs::read(path) {
            return data;
        }
    }
    panic!("[text] no font found — place a .ttf file at assets/font.ttf");
}
