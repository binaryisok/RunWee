//! Font registry: bundled fonts (Inter + KaTeX/Computer Modern), user fonts,
//! glyph outlines as bezier paths (cached) and text shaping via rustybuzz.

use crate::math::*;
use crate::path::*;
use rustybuzz::ttf_parser;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Index of a registered font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub usize);

pub struct FontEntry {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub face: rustybuzz::Face<'static>,
    pub units_per_em: f64,
}

macro_rules! font_bytes {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/",
            $name
        ))
    };
}

/// Bundled fonts: (family, bold, italic, data).
const BUNDLED: &[(&str, bool, bool, &[u8])] = &[
    ("Inter", false, false, font_bytes!("Inter-Regular.otf")),
    ("Inter", true, false, font_bytes!("Inter-Bold.otf")),
    (
        "Inter SemiBold",
        false,
        false,
        font_bytes!("Inter-SemiBold.otf"),
    ),
    ("Inter", false, true, font_bytes!("Inter-Italic.otf")),
    (
        "KaTeX_Main",
        false,
        false,
        font_bytes!("KaTeX_Main-Regular.ttf"),
    ),
    (
        "KaTeX_Main",
        true,
        false,
        font_bytes!("KaTeX_Main-Bold.ttf"),
    ),
    (
        "KaTeX_Main",
        false,
        true,
        font_bytes!("KaTeX_Main-Italic.ttf"),
    ),
    (
        "KaTeX_Main",
        true,
        true,
        font_bytes!("KaTeX_Main-BoldItalic.ttf"),
    ),
    (
        "KaTeX_Math",
        false,
        true,
        font_bytes!("KaTeX_Math-Italic.ttf"),
    ),
    (
        "KaTeX_Math",
        true,
        true,
        font_bytes!("KaTeX_Math-BoldItalic.ttf"),
    ),
    (
        "KaTeX_Size1",
        false,
        false,
        font_bytes!("KaTeX_Size1-Regular.ttf"),
    ),
    (
        "KaTeX_Size2",
        false,
        false,
        font_bytes!("KaTeX_Size2-Regular.ttf"),
    ),
    (
        "KaTeX_Size3",
        false,
        false,
        font_bytes!("KaTeX_Size3-Regular.ttf"),
    ),
    (
        "KaTeX_Size4",
        false,
        false,
        font_bytes!("KaTeX_Size4-Regular.ttf"),
    ),
    (
        "KaTeX_AMS",
        false,
        false,
        font_bytes!("KaTeX_AMS-Regular.ttf"),
    ),
    (
        "KaTeX_Caligraphic",
        false,
        false,
        font_bytes!("KaTeX_Caligraphic-Regular.ttf"),
    ),
    (
        "KaTeX_Fraktur",
        false,
        false,
        font_bytes!("KaTeX_Fraktur-Regular.ttf"),
    ),
    (
        "KaTeX_Typewriter",
        false,
        false,
        font_bytes!("KaTeX_Typewriter-Regular.ttf"),
    ),
    (
        "KaTeX_SansSerif",
        false,
        false,
        font_bytes!("KaTeX_SansSerif-Regular.ttf"),
    ),
];

struct Registry {
    fonts: RwLock<Vec<Arc<FontEntry>>>,
    glyphs: RwLock<HashMap<(usize, u16), Arc<Path>>>,
}

fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(|| {
        let fonts = BUNDLED
            .iter()
            .map(|(fam, b, i, data)| {
                let face = rustybuzz::Face::from_slice(data, 0).expect("bundled font");
                Arc::new(FontEntry {
                    family: fam.to_string(),
                    bold: *b,
                    italic: *i,
                    units_per_em: face.units_per_em() as f64,
                    face,
                })
            })
            .collect();
        Registry {
            fonts: RwLock::new(fonts),
            glyphs: RwLock::new(HashMap::new()),
        }
    })
}

/// Register a font file (TTF/OTF) under `family`. Returns its id.
pub fn register_font_file(
    path: impl AsRef<std::path::Path>,
    family: &str,
    bold: bool,
    italic: bool,
) -> anyhow::Result<FontId> {
    let data = std::fs::read(path)?;
    register_font_data(data, family, bold, italic)
}

/// Register font bytes (the data is kept for the program lifetime).
pub fn register_font_data(
    data: Vec<u8>,
    family: &str,
    bold: bool,
    italic: bool,
) -> anyhow::Result<FontId> {
    let data: &'static [u8] = Box::leak(data.into_boxed_slice());
    let face =
        rustybuzz::Face::from_slice(data, 0).ok_or_else(|| anyhow::anyhow!("invalid font"))?;
    let mut fonts = registry().fonts.write().unwrap();
    fonts.push(Arc::new(FontEntry {
        family: family.to_string(),
        bold,
        italic,
        units_per_em: face.units_per_em() as f64,
        face,
    }));
    Ok(FontId(fonts.len() - 1))
}

pub fn font(id: FontId) -> Arc<FontEntry> {
    registry().fonts.read().unwrap()[id.0].clone()
}

/// Find a font by family (case-insensitive aliases: "sans", "serif", "mono").
pub fn find_font(family: &str, bold: bool, italic: bool) -> FontId {
    let fam = match family.to_ascii_lowercase().as_str() {
        "" | "sans" | "sans-serif" | "default" | "inter" => "Inter".to_string(),
        "serif" | "cm" | "computer modern" | "latex" | "main" => "KaTeX_Main".to_string(),
        "mono" | "monospace" | "code" | "typewriter" => "KaTeX_Typewriter".to_string(),
        _ => family.to_string(),
    };
    let fonts = registry().fonts.read().unwrap();
    let matches: Vec<usize> = (0..fonts.len())
        .filter(|&i| fonts[i].family.eq_ignore_ascii_case(&fam))
        .collect();
    let best = matches
        .iter()
        .copied()
        .find(|&i| fonts[i].bold == bold && fonts[i].italic == italic)
        .or_else(|| matches.iter().copied().find(|&i| fonts[i].bold == bold))
        .or_else(|| matches.first().copied());
    FontId(best.unwrap_or(0))
}

/// Glyph id for a char, if the font has it.
pub fn glyph_index(id: FontId, ch: char) -> Option<u16> {
    let f = font(id);
    f.face.glyph_index(ch).map(|g| g.0).filter(|&g| g != 0)
}

/// Horizontal advance in em.
pub fn advance(id: FontId, gid: u16) -> f64 {
    let f = font(id);
    f.face
        .glyph_hor_advance(ttf_parser::GlyphId(gid))
        .unwrap_or(0) as f64
        / f.units_per_em
}

/// (height above baseline, depth below baseline) in em.
pub fn glyph_extent(id: FontId, gid: u16) -> (f64, f64) {
    let f = font(id);
    match f.face.glyph_bounding_box(ttf_parser::GlyphId(gid)) {
        Some(b) => (
            b.y_max as f64 / f.units_per_em,
            -(b.y_min as f64) / f.units_per_em,
        ),
        None => (0.0, 0.0),
    }
}

struct Builder {
    path: Path,
    scale: f64,
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path
            .move_to(v2(x as f64 * self.scale, y as f64 * self.scale));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path
            .line_to(v2(x as f64 * self.scale, y as f64 * self.scale));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let s = self.scale;
        self.path.quad_to(
            v2(x1 as f64 * s, y1 as f64 * s),
            v2(x as f64 * s, y as f64 * s),
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let s = self.scale;
        self.path.cubic_to(
            v2(x1 as f64 * s, y1 as f64 * s),
            v2(x2 as f64 * s, y2 as f64 * s),
            v2(x as f64 * s, y as f64 * s),
        );
    }
    fn close(&mut self) {
        self.path.close();
    }
}

/// Outline of a glyph in em units (baseline at y = 0, y up). Cached.
pub fn glyph_path(id: FontId, gid: u16) -> Arc<Path> {
    let key = (id.0, gid);
    if let Some(p) = registry().glyphs.read().unwrap().get(&key) {
        return p.clone();
    }
    let f = font(id);
    let mut b = Builder {
        path: Path::new(),
        scale: 1.0 / f.units_per_em,
    };
    f.face.outline_glyph(ttf_parser::GlyphId(gid), &mut b);
    let p = Arc::new(b.path);
    registry().glyphs.write().unwrap().insert(key, p.clone());
    p
}

/// One positioned glyph from shaping (em units, relative to the run origin).
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    pub gid: u16,
    pub x: f64,
    pub y: f64,
    pub advance: f64,
    /// Byte offset of the source cluster in the input string.
    pub cluster: usize,
}

/// Shape a single line of text with kerning/ligatures. Returns glyphs and total advance (em).
pub fn shape(id: FontId, text: &str) -> (Vec<ShapedGlyph>, f64) {
    let f = font(id);
    let mut buf = rustybuzz::UnicodeBuffer::new();
    buf.push_str(text);
    let out = rustybuzz::shape(&f.face, &[], buf);
    let upem = f.units_per_em;
    let mut x = 0.0;
    let mut glyphs = Vec::with_capacity(out.len());
    for (info, pos) in out.glyph_infos().iter().zip(out.glyph_positions()) {
        let adv = pos.x_advance as f64 / upem;
        glyphs.push(ShapedGlyph {
            gid: info.glyph_id as u16,
            x: x + pos.x_offset as f64 / upem,
            y: pos.y_offset as f64 / upem,
            advance: adv,
            cluster: info.cluster as usize,
        });
        x += adv;
    }
    (glyphs, x)
}

/// Ascender / descender of a font in em.
pub fn line_metrics(id: FontId) -> (f64, f64) {
    let f = font(id);
    (
        f.face.ascender() as f64 / f.units_per_em,
        -(f.face.descender() as f64) / f.units_per_em,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_load_and_shape() {
        let inter = find_font("sans", false, false);
        let (g, w) = shape(inter, "Hello");
        assert_eq!(g.len(), 5);
        assert!(w > 1.5 && w < 4.0);
        let p = glyph_path(inter, g[0].gid);
        assert!(!p.is_empty());
        let math = find_font("KaTeX_Math", false, true);
        assert!(glyph_index(math, 'x').is_some());
        assert!(glyph_index(math, 'α').is_some());
        let size4 = find_font("KaTeX_Size4", false, false);
        assert!(glyph_index(size4, '⎛').is_some());
    }
}
