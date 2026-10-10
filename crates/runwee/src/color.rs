//! Colors: sRGB with straight alpha, manim palette, OKLab mixing and gradients.

use crate::math::lerp;

/// sRGB color with straight (non premultiplied) alpha, components in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        WHITE
    }
}

pub const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

const fn hex(v: u32) -> Color {
    rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    /// Parse `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r,g,b)` or a palette name (`BLUE`, `teal_c`, `red`…).
    pub fn parse(s: &str) -> Option<Color> {
        let s = s.trim();
        if let Some(h) = s.strip_prefix('#') {
            let p = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
            return match h.len() {
                3 => {
                    let d = |i| p(i, 1).map(|v| v * 17);
                    Some(rgb8(d(0)?, d(1)?, d(2)?))
                }
                6 => Some(rgb8(p(0, 2)?, p(2, 2)?, p(4, 2)?)),
                8 => Some(rgb8(p(0, 2)?, p(2, 2)?, p(4, 2)?).with_alpha(p(6, 2)? as f32 / 255.0)),
                _ => None,
            };
        }
        if let Some(inner) = s
            .strip_prefix("rgb(")
            .or_else(|| s.strip_prefix("rgba("))
            .and_then(|r| r.strip_suffix(')'))
        {
            let v: Vec<f32> = inner
                .split(',')
                .filter_map(|x| x.trim().parse().ok())
                .collect();
            return match v.len() {
                3 => Some(rgb8(v[0] as u8, v[1] as u8, v[2] as u8)),
                4 => Some(rgb8(v[0] as u8, v[1] as u8, v[2] as u8).with_alpha(v[3])),
                _ => None,
            };
        }
        named(s)
    }

    pub fn to_hex(&self) -> String {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        if self.a < 1.0 {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                c(self.r),
                c(self.g),
                c(self.b),
                c(self.a)
            )
        } else {
            format!("#{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b))
        }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
    pub fn fade(self, opacity: f32) -> Self {
        Self {
            a: self.a * opacity,
            ..self
        }
    }

    /// HSV constructor, `h` in degrees.
    pub fn hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h.rem_euclid(360.0) / 60.0;
        let c = v * s;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = v - c;
        Color::rgb(r + m, g + m, b + m)
    }

    pub fn lighter(self, amount: f32) -> Self {
        self.mix_srgb(WHITE.with_alpha(self.a), amount)
    }
    pub fn darker(self, amount: f32) -> Self {
        self.mix_srgb(BLACK.with_alpha(self.a), amount)
    }

    /// Perceptual relative luminance (0..1).
    pub fn luminance(&self) -> f32 {
        0.2126 * to_linear(self.r) + 0.7152 * to_linear(self.g) + 0.0722 * to_linear(self.b)
    }

    /// Black or white, whichever reads better on top of this colour.
    pub fn contrasting(&self) -> Color {
        if self.luminance() > 0.35 {
            BLACK
        } else {
            WHITE
        }
    }

    /// Plain sRGB interpolation (what manim does).
    pub fn mix_srgb(self, o: Color, t: f32) -> Color {
        Color {
            r: self.r + (o.r - self.r) * t,
            g: self.g + (o.g - self.g) * t,
            b: self.b + (o.b - self.b) * t,
            a: self.a + (o.a - self.a) * t,
        }
    }

    /// Perceptual interpolation in OKLab — avoids the muddy midpoints of sRGB mixing.
    pub fn mix(self, o: Color, t: f32) -> Color {
        if t <= 0.0 {
            return self;
        }
        if t >= 1.0 {
            return o;
        }
        let a = self.to_oklab();
        let b = o.to_oklab();
        let m = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ];
        let mut c = Color::from_oklab(m);
        c.a = self.a + (o.a - self.a) * t;
        c
    }

    pub fn to_oklab(&self) -> [f32; 3] {
        let (r, g, b) = (to_linear(self.r), to_linear(self.g), to_linear(self.b));
        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
        [
            0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        ]
    }

    pub fn from_oklab(lab: [f32; 3]) -> Color {
        let l = lab[0] + 0.3963377774 * lab[1] + 0.2158037573 * lab[2];
        let m = lab[0] - 0.1055613458 * lab[1] - 0.0638541728 * lab[2];
        let s = lab[0] - 0.0894841775 * lab[1] - 1.2914855480 * lab[2];
        let (l, m, s) = (l * l * l, m * m * m, s * s * s);
        Color::rgb(
            from_linear(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
            from_linear(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
            from_linear(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s),
        )
    }

    pub fn to_skia(&self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba(
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
            self.a.clamp(0.0, 1.0),
        )
        .unwrap_or(tiny_skia::Color::BLACK)
    }

    pub fn to_rgba8(&self) -> [u8; 4] {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [c(self.r), c(self.g), c(self.b), c(self.a)]
    }
}

fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Sample a multi-stop gradient at `t ∈ [0,1]` (OKLab mixing).
pub fn color_gradient(colors: &[Color], t: f64) -> Color {
    match colors.len() {
        0 => WHITE,
        1 => colors[0],
        n => {
            let t = t.clamp(0.0, 1.0) * (n - 1) as f64;
            let i = (t.floor() as usize).min(n - 2);
            colors[i].mix(colors[i + 1], (t - i as f64) as f32)
        }
    }
}

/// `n` colors evenly spread along a gradient.
pub fn color_range(colors: &[Color], n: usize) -> Vec<Color> {
    (0..n)
        .map(|i| {
            color_gradient(
                colors,
                if n > 1 {
                    i as f64 / (n - 1) as f64
                } else {
                    0.0
                },
            )
        })
        .collect()
}

/// Interpolate opacity-like scalars stored as f32.
pub fn lerp_f32(a: f32, b: f32, t: f64) -> f32 {
    lerp(a as f64, b as f64, t) as f32
}

// ---- manim palette ---------------------------------------------------------

pub const WHITE: Color = hex(0xFFFFFF);
pub const BLACK: Color = hex(0x000000);
pub const GRAY_A: Color = hex(0xDDDDDD);
pub const GRAY_B: Color = hex(0xBBBBBB);
pub const GRAY_C: Color = hex(0x888888);
pub const GRAY_D: Color = hex(0x444444);
pub const GRAY_E: Color = hex(0x222222);
pub const GRAY: Color = GRAY_C;
pub const GREY: Color = GRAY_C;
pub const LIGHT_GRAY: Color = GRAY_B;
pub const DARK_GRAY: Color = GRAY_D;
pub const BLUE_A: Color = hex(0xC7E9F1);
pub const BLUE_B: Color = hex(0x9CDCEB);
pub const BLUE_C: Color = hex(0x58C4DD);
pub const BLUE_D: Color = hex(0x29ABCA);
pub const BLUE_E: Color = hex(0x236B8E);
pub const BLUE: Color = BLUE_C;
pub const TEAL_A: Color = hex(0xACEAD7);
pub const TEAL_B: Color = hex(0x76DDC0);
pub const TEAL_C: Color = hex(0x5CD0B3);
pub const TEAL_D: Color = hex(0x55C1A7);
pub const TEAL_E: Color = hex(0x49A88F);
pub const TEAL: Color = TEAL_C;
pub const GREEN_A: Color = hex(0xC9E2AE);
pub const GREEN_B: Color = hex(0xA6CF8C);
pub const GREEN_C: Color = hex(0x83C167);
pub const GREEN_D: Color = hex(0x77B05D);
pub const GREEN_E: Color = hex(0x699C52);
pub const GREEN: Color = GREEN_C;
pub const YELLOW_A: Color = hex(0xFFF1B6);
pub const YELLOW_B: Color = hex(0xFFEA94);
pub const YELLOW_C: Color = hex(0xFFFF00);
pub const YELLOW_D: Color = hex(0xF4D345);
pub const YELLOW_E: Color = hex(0xE8C11C);
pub const YELLOW: Color = YELLOW_C;
pub const GOLD_A: Color = hex(0xF7C797);
pub const GOLD_B: Color = hex(0xF9B775);
pub const GOLD_C: Color = hex(0xF0AC5F);
pub const GOLD_D: Color = hex(0xE1A158);
pub const GOLD_E: Color = hex(0xC78D46);
pub const GOLD: Color = GOLD_C;
pub const RED_A: Color = hex(0xF7A1A3);
pub const RED_B: Color = hex(0xFF8080);
pub const RED_C: Color = hex(0xFC6255);
pub const RED_D: Color = hex(0xE65A4C);
pub const RED_E: Color = hex(0xCF5044);
pub const RED: Color = RED_C;
pub const MAROON_A: Color = hex(0xECABC1);
pub const MAROON_B: Color = hex(0xEC92AB);
pub const MAROON_C: Color = hex(0xC55F73);
pub const MAROON_D: Color = hex(0xA24D61);
pub const MAROON_E: Color = hex(0x94424F);
pub const MAROON: Color = MAROON_C;
pub const PURPLE_A: Color = hex(0xCAA3E8);
pub const PURPLE_B: Color = hex(0xB189C6);
pub const PURPLE_C: Color = hex(0x9A72AC);
pub const PURPLE_D: Color = hex(0x715582);
pub const PURPLE_E: Color = hex(0x644172);
pub const PURPLE: Color = PURPLE_C;
pub const PINK: Color = hex(0xD147BD);
pub const LIGHT_PINK: Color = hex(0xDC75CD);
pub const ORANGE: Color = hex(0xFF862F);
pub const LIGHT_BROWN: Color = hex(0xCD853F);
pub const DARK_BROWN: Color = hex(0x8B4513);
pub const GRAY_BROWN: Color = hex(0x736357);
pub const PURE_RED: Color = hex(0xFF0000);
pub const PURE_GREEN: Color = hex(0x00FF00);
pub const PURE_BLUE: Color = hex(0x0000FF);
pub const DARK_BLUE: Color = BLUE_E;

/// Default background (3b1b-style deep charcoal).
pub const BACKGROUND: Color = hex(0x0E1117);

/// All palette names, for scripting and parsing.
pub const PALETTE: &[(&str, Color)] = &[
    ("WHITE", WHITE),
    ("BLACK", BLACK),
    ("GRAY_A", GRAY_A),
    ("GRAY_B", GRAY_B),
    ("GRAY_C", GRAY_C),
    ("GRAY_D", GRAY_D),
    ("GRAY_E", GRAY_E),
    ("GRAY", GRAY),
    ("GREY", GREY),
    ("LIGHT_GRAY", LIGHT_GRAY),
    ("DARK_GRAY", DARK_GRAY),
    ("BLUE_A", BLUE_A),
    ("BLUE_B", BLUE_B),
    ("BLUE_C", BLUE_C),
    ("BLUE_D", BLUE_D),
    ("BLUE_E", BLUE_E),
    ("BLUE", BLUE),
    ("DARK_BLUE", DARK_BLUE),
    ("TEAL_A", TEAL_A),
    ("TEAL_B", TEAL_B),
    ("TEAL_C", TEAL_C),
    ("TEAL_D", TEAL_D),
    ("TEAL_E", TEAL_E),
    ("TEAL", TEAL),
    ("GREEN_A", GREEN_A),
    ("GREEN_B", GREEN_B),
    ("GREEN_C", GREEN_C),
    ("GREEN_D", GREEN_D),
    ("GREEN_E", GREEN_E),
    ("GREEN", GREEN),
    ("YELLOW_A", YELLOW_A),
    ("YELLOW_B", YELLOW_B),
    ("YELLOW_C", YELLOW_C),
    ("YELLOW_D", YELLOW_D),
    ("YELLOW_E", YELLOW_E),
    ("YELLOW", YELLOW),
    ("GOLD_A", GOLD_A),
    ("GOLD_B", GOLD_B),
    ("GOLD_C", GOLD_C),
    ("GOLD_D", GOLD_D),
    ("GOLD_E", GOLD_E),
    ("GOLD", GOLD),
    ("RED_A", RED_A),
    ("RED_B", RED_B),
    ("RED_C", RED_C),
    ("RED_D", RED_D),
    ("RED_E", RED_E),
    ("RED", RED),
    ("MAROON_A", MAROON_A),
    ("MAROON_B", MAROON_B),
    ("MAROON_C", MAROON_C),
    ("MAROON_D", MAROON_D),
    ("MAROON_E", MAROON_E),
    ("MAROON", MAROON),
    ("PURPLE_A", PURPLE_A),
    ("PURPLE_B", PURPLE_B),
    ("PURPLE_C", PURPLE_C),
    ("PURPLE_D", PURPLE_D),
    ("PURPLE_E", PURPLE_E),
    ("PURPLE", PURPLE),
    ("PINK", PINK),
    ("LIGHT_PINK", LIGHT_PINK),
    ("ORANGE", ORANGE),
    ("LIGHT_BROWN", LIGHT_BROWN),
    ("DARK_BROWN", DARK_BROWN),
    ("GRAY_BROWN", GRAY_BROWN),
    ("PURE_RED", PURE_RED),
    ("PURE_GREEN", PURE_GREEN),
    ("PURE_BLUE", PURE_BLUE),
    ("BACKGROUND", BACKGROUND),
    ("TRANSPARENT", Color::TRANSPARENT),
];

/// Look up a palette / CSS-like color name (case insensitive).
pub fn named(name: &str) -> Option<Color> {
    let up = name.to_ascii_uppercase().replace([' ', '-'], "_");
    if let Some((_, c)) = PALETTE.iter().find(|(n, _)| *n == up) {
        return Some(*c);
    }
    Some(match up.as_str() {
        "CYAN" | "AQUA" => hex(0x00FFFF),
        "MAGENTA" | "FUCHSIA" => hex(0xFF00FF),
        "NAVY" => hex(0x000080),
        "LIME" => hex(0x32CD32),
        "BROWN" => hex(0x8B5A2B),
        "SILVER" => hex(0xC0C0C0),
        "INDIGO" => hex(0x4B0082),
        "VIOLET" => hex(0xEE82EE),
        "CRIMSON" => hex(0xDC143C),
        "SKY" | "SKYBLUE" | "SKY_BLUE" => hex(0x87CEEB),
        "BEIGE" => hex(0xF5F5DC),
        "SALMON" => hex(0xFA8072),
        "OLIVE" => hex(0x808000),
        "CORAL" => hex(0xFF7F50),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_colors() {
        assert_eq!(Color::parse("#ff0000"), Some(PURE_RED));
        assert_eq!(Color::parse("#f00"), Some(PURE_RED));
        assert_eq!(Color::parse("blue"), Some(BLUE));
        assert_eq!(Color::parse("teal_e"), Some(TEAL_E));
        assert_eq!(Color::parse("rgb(0,255,0)"), Some(PURE_GREEN));
        assert!((Color::parse("#00000080").unwrap().a - 0.502).abs() < 0.01);
        assert_eq!(BLUE.to_hex(), "#58c4dd");
    }

    #[test]
    fn oklab_roundtrip() {
        for c in [BLUE, RED, YELLOW, GRAY, WHITE, BLACK] {
            let d = Color::from_oklab(c.to_oklab());
            assert!(
                (c.r - d.r).abs() < 1e-3 && (c.g - d.g).abs() < 1e-3 && (c.b - d.b).abs() < 1e-3
            );
        }
        let m = BLUE.mix(RED, 0.5);
        assert!(m.r > 0.3 && m.b > 0.3);
        assert_eq!(color_range(&[BLACK, WHITE], 3).len(), 3);
    }
}
