//! Chemistry: all 118 elements (periodic table, tiles, categories), electron
//! configurations, Bohr atoms (static & orbiting), orbital box diagrams, orbital
//! shapes, formula parsing, molar mass, automatic equation balancing, 2D structural
//! formulas, 3D ball-and-stick molecules (with XYZ import), Lewis dot structures,
//! and lab glassware (beaker, flask, test tube, burette, burner, bubbles).

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::shapes::*;
use crate::tex::tex_sized;
use crate::text::{bold_text, text_sized};
use crate::three_d::{Ball, Face, Mesh};
use std::collections::BTreeMap;

/// (symbol, name, standard atomic mass, category)
/// Categories: a alkali, e alkaline-earth, t transition, p post-transition, m metalloid,
/// n reactive nonmetal, h halogen, g noble gas, l lanthanide, c actinide, u unknown.
pub const ELEMENTS: [(&str, &str, f64, char); 118] = [
    ("H", "Hydrogen", 1.008, 'n'),
    ("He", "Helium", 4.0026, 'g'),
    ("Li", "Lithium", 6.94, 'a'),
    ("Be", "Beryllium", 9.0122, 'e'),
    ("B", "Boron", 10.81, 'm'),
    ("C", "Carbon", 12.011, 'n'),
    ("N", "Nitrogen", 14.007, 'n'),
    ("O", "Oxygen", 15.999, 'n'),
    ("F", "Fluorine", 18.998, 'h'),
    ("Ne", "Neon", 20.180, 'g'),
    ("Na", "Sodium", 22.990, 'a'),
    ("Mg", "Magnesium", 24.305, 'e'),
    ("Al", "Aluminium", 26.982, 'p'),
    ("Si", "Silicon", 28.085, 'm'),
    ("P", "Phosphorus", 30.974, 'n'),
    ("S", "Sulfur", 32.06, 'n'),
    ("Cl", "Chlorine", 35.45, 'h'),
    ("Ar", "Argon", 39.948, 'g'),
    ("K", "Potassium", 39.098, 'a'),
    ("Ca", "Calcium", 40.078, 'e'),
    ("Sc", "Scandium", 44.956, 't'),
    ("Ti", "Titanium", 47.867, 't'),
    ("V", "Vanadium", 50.942, 't'),
    ("Cr", "Chromium", 51.996, 't'),
    ("Mn", "Manganese", 54.938, 't'),
    ("Fe", "Iron", 55.845, 't'),
    ("Co", "Cobalt", 58.933, 't'),
    ("Ni", "Nickel", 58.693, 't'),
    ("Cu", "Copper", 63.546, 't'),
    ("Zn", "Zinc", 65.38, 't'),
    ("Ga", "Gallium", 69.723, 'p'),
    ("Ge", "Germanium", 72.630, 'm'),
    ("As", "Arsenic", 74.922, 'm'),
    ("Se", "Selenium", 78.971, 'n'),
    ("Br", "Bromine", 79.904, 'h'),
    ("Kr", "Krypton", 83.798, 'g'),
    ("Rb", "Rubidium", 85.468, 'a'),
    ("Sr", "Strontium", 87.62, 'e'),
    ("Y", "Yttrium", 88.906, 't'),
    ("Zr", "Zirconium", 91.224, 't'),
    ("Nb", "Niobium", 92.906, 't'),
    ("Mo", "Molybdenum", 95.95, 't'),
    ("Tc", "Technetium", 98.0, 't'),
    ("Ru", "Ruthenium", 101.07, 't'),
    ("Rh", "Rhodium", 102.91, 't'),
    ("Pd", "Palladium", 106.42, 't'),
    ("Ag", "Silver", 107.87, 't'),
    ("Cd", "Cadmium", 112.41, 't'),
    ("In", "Indium", 114.82, 'p'),
    ("Sn", "Tin", 118.71, 'p'),
    ("Sb", "Antimony", 121.76, 'm'),
    ("Te", "Tellurium", 127.60, 'm'),
    ("I", "Iodine", 126.90, 'h'),
    ("Xe", "Xenon", 131.29, 'g'),
    ("Cs", "Caesium", 132.91, 'a'),
    ("Ba", "Barium", 137.33, 'e'),
    ("La", "Lanthanum", 138.91, 'l'),
    ("Ce", "Cerium", 140.12, 'l'),
    ("Pr", "Praseodymium", 140.91, 'l'),
    ("Nd", "Neodymium", 144.24, 'l'),
    ("Pm", "Promethium", 145.0, 'l'),
    ("Sm", "Samarium", 150.36, 'l'),
    ("Eu", "Europium", 151.96, 'l'),
    ("Gd", "Gadolinium", 157.25, 'l'),
    ("Tb", "Terbium", 158.93, 'l'),
    ("Dy", "Dysprosium", 162.50, 'l'),
    ("Ho", "Holmium", 164.93, 'l'),
    ("Er", "Erbium", 167.26, 'l'),
    ("Tm", "Thulium", 168.93, 'l'),
    ("Yb", "Ytterbium", 173.05, 'l'),
    ("Lu", "Lutetium", 174.97, 'l'),
    ("Hf", "Hafnium", 178.49, 't'),
    ("Ta", "Tantalum", 180.95, 't'),
    ("W", "Tungsten", 183.84, 't'),
    ("Re", "Rhenium", 186.21, 't'),
    ("Os", "Osmium", 190.23, 't'),
    ("Ir", "Iridium", 192.22, 't'),
    ("Pt", "Platinum", 195.08, 't'),
    ("Au", "Gold", 196.97, 't'),
    ("Hg", "Mercury", 200.59, 't'),
    ("Tl", "Thallium", 204.38, 'p'),
    ("Pb", "Lead", 207.2, 'p'),
    ("Bi", "Bismuth", 208.98, 'p'),
    ("Po", "Polonium", 209.0, 'p'),
    ("At", "Astatine", 210.0, 'h'),
    ("Rn", "Radon", 222.0, 'g'),
    ("Fr", "Francium", 223.0, 'a'),
    ("Ra", "Radium", 226.0, 'e'),
    ("Ac", "Actinium", 227.0, 'c'),
    ("Th", "Thorium", 232.04, 'c'),
    ("Pa", "Protactinium", 231.04, 'c'),
    ("U", "Uranium", 238.03, 'c'),
    ("Np", "Neptunium", 237.0, 'c'),
    ("Pu", "Plutonium", 244.0, 'c'),
    ("Am", "Americium", 243.0, 'c'),
    ("Cm", "Curium", 247.0, 'c'),
    ("Bk", "Berkelium", 247.0, 'c'),
    ("Cf", "Californium", 251.0, 'c'),
    ("Es", "Einsteinium", 252.0, 'c'),
    ("Fm", "Fermium", 257.0, 'c'),
    ("Md", "Mendelevium", 258.0, 'c'),
    ("No", "Nobelium", 259.0, 'c'),
    ("Lr", "Lawrencium", 266.0, 'c'),
    ("Rf", "Rutherfordium", 267.0, 't'),
    ("Db", "Dubnium", 268.0, 't'),
    ("Sg", "Seaborgium", 269.0, 't'),
    ("Bh", "Bohrium", 270.0, 't'),
    ("Hs", "Hassium", 277.0, 't'),
    ("Mt", "Meitnerium", 278.0, 'u'),
    ("Ds", "Darmstadtium", 281.0, 'u'),
    ("Rg", "Roentgenium", 282.0, 'u'),
    ("Cn", "Copernicium", 285.0, 't'),
    ("Nh", "Nihonium", 286.0, 'u'),
    ("Fl", "Flerovium", 289.0, 'u'),
    ("Mc", "Moscovium", 290.0, 'u'),
    ("Lv", "Livermorium", 293.0, 'u'),
    ("Ts", "Tennessine", 294.0, 'u'),
    ("Og", "Oganesson", 294.0, 'u'),
];

/// Information about one element.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub z: u32,
    pub symbol: &'static str,
    pub name: &'static str,
    pub mass: f64,
    pub category: char,
    pub period: u32,
    /// 1..18, or 0 for the f-block (lanthanides/actinides other than La/Ac).
    pub group: u32,
}

pub fn element(z: u32) -> Option<Element> {
    if z == 0 || z > 118 {
        return None;
    }
    let (symbol, name, mass, category) = ELEMENTS[(z - 1) as usize];
    let (period, start) = match z {
        1..=2 => (1, 1),
        3..=10 => (2, 3),
        11..=18 => (3, 11),
        19..=36 => (4, 19),
        37..=54 => (5, 37),
        55..=86 => (6, 55),
        _ => (7, 87),
    };
    let i = z - start + 1;
    let group = match period {
        1 => {
            if z == 1 {
                1
            } else {
                18
            }
        }
        2 | 3 => {
            if i <= 2 {
                i
            } else {
                i + 10
            }
        }
        4 | 5 => i,
        _ => {
            if i <= 2 {
                i
            } else if i == 3 {
                3
            } else if i <= 17 {
                0
            } else {
                i - 14
            }
        }
    };
    Some(Element {
        z,
        symbol,
        name,
        mass,
        category,
        period,
        group,
    })
}

pub fn element_by_symbol(sym: &str) -> Option<Element> {
    ELEMENTS
        .iter()
        .position(|e| e.0.eq_ignore_ascii_case(sym) && e.0.len() == sym.len())
        .and_then(|i| element(i as u32 + 1))
}

pub fn category_name(c: char) -> &'static str {
    match c {
        'a' => "Alkali metal",
        'e' => "Alkaline earth metal",
        't' => "Transition metal",
        'p' => "Post-transition metal",
        'm' => "Metalloid",
        'n' => "Reactive nonmetal",
        'h' => "Halogen",
        'g' => "Noble gas",
        'l' => "Lanthanide",
        'c' => "Actinide",
        _ => "Unknown",
    }
}

pub fn category_color(c: char) -> Color {
    match c {
        'a' => rgb8(0xE5, 0x6B, 0x6F),
        'e' => rgb8(0xF0, 0x9D, 0x51),
        't' => rgb8(0xE9, 0xC4, 0x6A),
        'p' => rgb8(0x8A, 0xB1, 0x7D),
        'm' => rgb8(0x2A, 0x9D, 0x8F),
        'n' => rgb8(0x4C, 0xA3, 0xDD),
        'h' => rgb8(0x6C, 0x8E, 0xF5),
        'g' => rgb8(0xA2, 0x7B, 0xD9),
        'l' => rgb8(0xE0, 0x84, 0xB5),
        'c' => rgb8(0xC0, 0x6C, 0x84),
        _ => GRAY_C,
    }
}

// ----------------------------------------------------- electron configuration

const ORBITALS: [(u32, char); 19] = [
    (1, 's'),
    (2, 's'),
    (2, 'p'),
    (3, 's'),
    (3, 'p'),
    (4, 's'),
    (3, 'd'),
    (4, 'p'),
    (5, 's'),
    (4, 'd'),
    (5, 'p'),
    (6, 's'),
    (4, 'f'),
    (5, 'd'),
    (6, 'p'),
    (7, 's'),
    (5, 'f'),
    (6, 'd'),
    (7, 'p'),
];

fn capacity(l: char) -> u32 {
    match l {
        's' => 2,
        'p' => 6,
        'd' => 10,
        _ => 14,
    }
}

/// Subshell occupancy (n, l, electrons) by the Aufbau principle with the common
/// exceptions (Cr, Cu, Nb, Mo, Ru, Rh, Pd, Ag, Pt, Au, La, Ce, Gd, Ac, Th…).
pub fn electron_configuration(z: u32) -> Vec<(u32, char, u32)> {
    let mut left = z;
    let mut out: Vec<(u32, char, u32)> = Vec::new();
    for (n, l) in ORBITALS {
        if left == 0 {
            break;
        }
        let k = left.min(capacity(l));
        out.push((n, l, k));
        left -= k;
    }
    let mut set = |n: u32, l: char, v: u32| {
        if let Some(e) = out.iter_mut().find(|e| e.0 == n && e.1 == l) {
            e.2 = v;
        } else {
            out.push((n, l, v));
        }
    };
    match z {
        24 => {
            set(3, 'd', 5);
            set(4, 's', 1)
        }
        29 => {
            set(3, 'd', 10);
            set(4, 's', 1)
        }
        41 => {
            set(4, 'd', 4);
            set(5, 's', 1)
        }
        42 => {
            set(4, 'd', 5);
            set(5, 's', 1)
        }
        44 => {
            set(4, 'd', 7);
            set(5, 's', 1)
        }
        45 => {
            set(4, 'd', 8);
            set(5, 's', 1)
        }
        46 => {
            set(4, 'd', 10);
            set(5, 's', 0)
        }
        47 => {
            set(4, 'd', 10);
            set(5, 's', 1)
        }
        57 => {
            set(4, 'f', 0);
            set(5, 'd', 1)
        }
        58 => {
            set(4, 'f', 1);
            set(5, 'd', 1)
        }
        64 => {
            set(4, 'f', 7);
            set(5, 'd', 1)
        }
        78 => {
            set(5, 'd', 9);
            set(6, 's', 1)
        }
        79 => {
            set(5, 'd', 10);
            set(6, 's', 1)
        }
        89 => {
            set(5, 'f', 0);
            set(6, 'd', 1)
        }
        90 => {
            set(5, 'f', 0);
            set(6, 'd', 2)
        }
        _ => {}
    }
    out.retain(|e| e.2 > 0);
    out
}

/// Configuration as TeX, e.g. `1s^{2} 2s^{2} 2p^{6}`; `noble` uses the [Ne]-style core.
pub fn electron_configuration_tex(z: u32, noble: bool) -> String {
    let cfg = electron_configuration(z);
    let mut prefix = String::new();
    let mut skip = 0;
    if noble {
        for (gz, sym) in [
            (86, "Rn"),
            (54, "Xe"),
            (36, "Kr"),
            (18, "Ar"),
            (10, "Ne"),
            (2, "He"),
        ] {
            if z > gz {
                let core = electron_configuration(gz);
                prefix = format!("[\\mathrm{{{sym}}}]\\,");
                skip = core.len();
                break;
            }
        }
    }
    let body: Vec<String> = cfg
        .iter()
        .skip(skip)
        .map(|(n, l, k)| format!("{n}{l}^{{{k}}}"))
        .collect();
    format!("{prefix}{}", body.join("\\,"))
}

/// Electrons per shell (K, L, M, …) for Bohr diagrams.
pub fn shells(z: u32) -> Vec<u32> {
    let mut s = vec![0u32; 7];
    for (n, _, k) in electron_configuration(z) {
        s[(n - 1) as usize] += k;
    }
    while s.last() == Some(&0) {
        s.pop();
    }
    s
}

pub fn valence_electrons(z: u32) -> u32 {
    let e = element(z).unwrap();
    match e.group {
        1 | 2 => e.group,
        13..=18 => {
            if z == 2 {
                2
            } else {
                e.group - 10
            }
        }
        _ => *shells(z).last().unwrap_or(&0),
    }
}

// ------------------------------------------------------------- tiles & table

/// One periodic-table tile (Z, symbol, name, mass).
pub fn element_tile(z: u32, size: f64) -> Mobject {
    let e = element(z).expect("1..=118");
    let c = category_color(e.category);
    let bg = rounded_rectangle(size, size * 1.12, size * 0.08)
        .fill(c.darker(0.35), 1.0)
        .stroke(c, (size * 4.0) as f32);
    let num = text_sized(&z.to_string(), size * 0.16, WHITE).move_to(v2(-size * 0.32, size * 0.4));
    let sym = bold_text(e.symbol, size * 0.42, WHITE).move_to(v2(0.0, size * 0.06));
    let mut g = Mobject::group([bg, num, sym]);
    if size >= 0.9 {
        g.push(text_sized(e.name, size * 0.12, WHITE).move_to(v2(0.0, -size * 0.27)));
        g.push(
            text_sized(&format!("{:.3}", e.mass), size * 0.11, GRAY_A)
                .move_to(v2(0.0, -size * 0.42)),
        );
    }
    g.with_tag(e.symbol)
}

/// Grid position (column, row) of an element in the standard 18-column layout with
/// the f-block placed in two extra rows below.
pub fn table_position(z: u32) -> (u32, u32) {
    let e = element(z).unwrap();
    if e.group == 0 || (e.category == 'l' || e.category == 'c') && !(z == 57 || z == 89) {
        let row = if e.category == 'l' { 8 } else { 9 };
        let start = if e.category == 'l' { 57 } else { 89 };
        (3 + (z - start), row)
    } else {
        (e.group, e.period)
    }
}

/// The full periodic table (children are tiles tagged by symbol), `cell` units per tile.
pub fn periodic_table(cell: f64) -> Mobject {
    let mut g = Mobject::empty();
    for z in 1..=118 {
        let (col, row) = table_position(z);
        let y_off = if row >= 8 { cell * 0.35 } else { 0.0 };
        let p = v2(
            (col as f64 - 9.5) * cell * 1.04,
            -(row as f64 - 5.0) * cell * 1.16 - y_off,
        );
        let e = element(z).unwrap();
        let c = category_color(e.category);
        let tile = Mobject::group([
            rectangle(cell, cell * 1.12)
                .move_to(p)
                .fill(c.darker(0.3), 1.0)
                .stroke(c, 2.0),
            text_sized(e.symbol, cell * 0.4, WHITE).move_to(p + UP * (cell * 0.05)),
            text_sized(&z.to_string(), cell * 0.18, GRAY_A)
                .move_to(p + v2(-cell * 0.28, cell * 0.38)),
        ])
        .with_tag(e.symbol);
        g.push(tile);
    }
    g
}

// ----------------------------------------------------------------- atoms

/// Bohr model: nucleus and electrons on shells; `phase` rotates electrons (animate it).
pub fn bohr_atom(z: u32, radius: f64, phase: f64) -> Mobject {
    let e = element(z).unwrap();
    let sh = shells(z);
    let n = sh.len().max(1);
    let nucleus_r = radius * 0.16;
    let mut g = Mobject::empty();
    for (i, count) in sh.iter().enumerate() {
        let r = nucleus_r * 1.6 + (radius - nucleus_r * 1.6) * (i + 1) as f64 / n as f64;
        g.push(circle(r).stroke(GRAY_B, 2.0).stroke_opacity(0.6));
        for k in 0..*count {
            let a = TAU * k as f64 / *count as f64
                + phase * (1.0 + 0.35 * i as f64) / (i + 1) as f64
                + i as f64 * 0.4;
            g.push(
                circle_at(Vec2::polar(r, a), radius * 0.045)
                    .no_stroke()
                    .sheen(BLUE_B, UL)
                    .with_tag("electron"),
            );
        }
    }
    g.push(
        circle(nucleus_r)
            .no_stroke()
            .sheen(RED_D, UL)
            .with_tag("nucleus"),
    );
    g.push(text_sized(e.symbol, nucleus_r * 0.9, WHITE).move_to(Vec2::ZERO));
    g
}

/// Orbital box diagram (arrows ↑↓ in boxes) following Hund's rule.
pub fn orbital_box_diagram(z: u32, box_size: f64) -> Mobject {
    let mut g = Mobject::empty();
    let mut x = 0.0;
    for (n, l, k) in electron_configuration(z) {
        let boxes = capacity(l) / 2;
        let mut sub = Mobject::empty();
        for b in 0..boxes {
            let c = v2(x + b as f64 * box_size, 0.0);
            sub.push(square(box_size).move_to(c).stroke(WHITE, 2.5));
            let up = (b < k.min(boxes)) as u32;
            let down = (k > boxes && b < k - boxes) as u32;
            if up == 1 {
                sub.push(
                    arrow(
                        c + v2(-box_size * 0.15, -box_size * 0.35),
                        c + v2(-box_size * 0.15, box_size * 0.35),
                    )
                    .color(YELLOW),
                );
            }
            if down == 1 {
                sub.push(
                    arrow(
                        c + v2(box_size * 0.15, box_size * 0.35),
                        c + v2(box_size * 0.15, -box_size * 0.35),
                    )
                    .color(YELLOW),
                );
            }
        }
        let label = tex_sized(&format!("{n}{l}"), box_size * 0.6, GRAY_A).next_to(
            &sub,
            DOWN,
            box_size * 0.2,
        );
        sub.push(label);
        g.push(sub);
        x += boxes as f64 * box_size + box_size * 0.6;
    }
    let c = g.center();
    g.shift(-c)
}

/// Lewis dot symbol with `electrons` valence dots (paired around 4 sides).
pub fn lewis_dot(symbol: &str, electrons: u32, size: f64) -> Mobject {
    let s = text_sized(symbol, size, WHITE);
    let (w, h) = (
        s.width() / 2.0 + size * 0.25,
        s.height() / 2.0 + size * 0.22,
    );
    let sides = [
        (v2(0.0, h), RIGHT),
        (v2(w, 0.0), UP),
        (v2(0.0, -h), RIGHT),
        (v2(-w, 0.0), UP),
    ];
    let mut g = Mobject::group([s]);
    for i in 0..electrons.min(8) {
        let side = (i % 4) as usize;
        let second = i >= 4;
        let (p, along) = sides[side];
        let off = if electrons > 4 || second {
            size * 0.1
        } else {
            0.0
        };
        let pos = p + along * (if second { off } else { -off });
        g.push(dot_r(pos, size * 0.06, YELLOW));
    }
    g
}

// ------------------------------------------------------------------ orbitals

fn lobe(dir: f64, len: f64, color: Color) -> Mobject {
    let pts: Vec<Vec2> = (0..=60)
        .map(|i| {
            let a = -PI / 2.0 + PI * i as f64 / 60.0;
            let r = len * a.cos().powi(2);
            Vec2::polar(r, a + dir)
        })
        .collect();
    smooth_curve(&pts, true)
        .no_stroke()
        .sheen(color, UL)
        .fill_opacity(0.85)
}

/// 2D sketch of an atomic orbital: "s", "px", "py", "pz", "dxy", "dx2-y2", "dz2".
pub fn orbital_shape(kind: &str, size: f64) -> Mobject {
    let (pos, neg) = (BLUE, ORANGE);
    match kind {
        "s" => circle(size * 0.5)
            .no_stroke()
            .sheen(BLUE, UL)
            .fill_opacity(0.85),
        "px" => Mobject::group([lobe(0.0, size, pos), lobe(PI, size, neg)]),
        "py" | "pz" => Mobject::group([lobe(PI / 2.0, size, pos), lobe(-PI / 2.0, size, neg)]),
        "dxy" => Mobject::group((0..4).map(|i| {
            lobe(
                PI / 4.0 + i as f64 * PI / 2.0,
                size * 0.8,
                if i % 2 == 0 { pos } else { neg },
            )
        })),
        "dx2-y2" => Mobject::group((0..4).map(|i| {
            lobe(
                i as f64 * PI / 2.0,
                size * 0.8,
                if i % 2 == 0 { pos } else { neg },
            )
        })),
        "dz2" => Mobject::group([
            lobe(PI / 2.0, size, pos),
            lobe(-PI / 2.0, size, pos),
            ellipse(size * 0.9, size * 0.3).no_stroke().fill(neg, 0.8),
        ]),
        _ => circle(size * 0.5),
    }
}

// --------------------------------------------------------- formulas & mass

/// Parse a formula like `Ca(OH)2`, `CuSO4·5H2O`, `[Fe(CN)6]3-` into element counts.
pub fn parse_formula(f: &str) -> BTreeMap<String, i64> {
    fn parse(chars: &[char], i: &mut usize) -> BTreeMap<String, i64> {
        let mut m: BTreeMap<String, i64> = BTreeMap::new();
        while *i < chars.len() {
            let c = chars[*i];
            if c == '(' || c == '[' {
                *i += 1;
                let inner = parse(chars, i);
                let k = read_num(chars, i).unwrap_or(1);
                for (e, n) in inner {
                    *m.entry(e).or_default() += n * k;
                }
            } else if c == ')' || c == ']' {
                *i += 1;
                return m;
            } else if c.is_ascii_uppercase() {
                let mut sym = c.to_string();
                *i += 1;
                while *i < chars.len() && chars[*i].is_ascii_lowercase() {
                    sym.push(chars[*i]);
                    *i += 1;
                }
                let k = read_num(chars, i).unwrap_or(1);
                *m.entry(sym).or_default() += k;
            } else if c == '·' || c == '*' || c == '.' {
                *i += 1;
                let k = read_num(chars, i).unwrap_or(1);
                let rest = parse(chars, i);
                for (e, n) in rest {
                    *m.entry(e).or_default() += n * k;
                }
            } else {
                // charges, states, spaces: skip
                *i += 1;
                if c == '^' {
                    while *i < chars.len()
                        && (chars[*i].is_ascii_digit() || chars[*i] == '+' || chars[*i] == '-')
                    {
                        *i += 1;
                    }
                }
            }
        }
        m
    }
    fn read_num(chars: &[char], i: &mut usize) -> Option<i64> {
        let s = *i;
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            *i += 1;
        }
        if *i > s {
            chars[s..*i].iter().collect::<String>().parse().ok()
        } else {
            None
        }
    }
    let chars: Vec<char> = f.trim().chars().collect();
    let mut i = 0;
    // leading coefficient is not part of the formula
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    parse(&chars, &mut i)
}

/// Molar mass in g/mol.
pub fn molar_mass(formula: &str) -> f64 {
    parse_formula(formula)
        .iter()
        .map(|(e, n)| element_by_symbol(e).map_or(0.0, |x| x.mass) * *n as f64)
        .sum()
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

#[derive(Clone, Copy, Debug)]
struct Frac(i128, i128);
impl Frac {
    fn new(n: i128, d: i128) -> Frac {
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Frac(s * n / g, s * d / g)
    }
    fn sub(self, o: Frac) -> Frac {
        Frac::new(self.0 * o.1 - o.0 * self.1, self.1 * o.1)
    }
    fn mul(self, o: Frac) -> Frac {
        Frac::new(self.0 * o.0, self.1 * o.1)
    }
    fn div(self, o: Frac) -> Frac {
        Frac::new(self.0 * o.1, self.1 * o.0)
    }
    fn is_zero(self) -> bool {
        self.0 == 0
    }
}

/// Balance a reaction: returns the smallest positive integer coefficients for
/// `reactants` then `products`, or `None` if it cannot be balanced uniquely.
pub fn balance(reactants: &[&str], products: &[&str]) -> Option<Vec<u64>> {
    let species: Vec<BTreeMap<String, i64>> = reactants
        .iter()
        .chain(products)
        .map(|s| parse_formula(s))
        .collect();
    let mut elements: Vec<String> = species.iter().flat_map(|m| m.keys().cloned()).collect();
    elements.sort();
    elements.dedup();
    let nr = reactants.len();
    let cols = species.len();
    let mut a: Vec<Vec<Frac>> = elements
        .iter()
        .map(|e| {
            (0..cols)
                .map(|j| {
                    let v = *species[j].get(e).unwrap_or(&0) as i128;
                    Frac::new(if j < nr { v } else { -v }, 1)
                })
                .collect()
        })
        .collect();
    // RREF
    let mut pivots = Vec::new();
    let mut row = 0;
    for col in 0..cols {
        let Some(p) = (row..a.len()).find(|&r| !a[r][col].is_zero()) else {
            continue;
        };
        a.swap(row, p);
        let pv = a[row][col];
        for c in 0..cols {
            a[row][c] = a[row][c].div(pv);
        }
        for r in 0..a.len() {
            if r != row && !a[r][col].is_zero() {
                let f = a[r][col];
                for c in 0..cols {
                    a[r][c] = a[r][c].sub(f.mul(a[row][c]));
                }
            }
        }
        pivots.push(col);
        row += 1;
        if row == a.len() {
            break;
        }
    }
    let free: Vec<usize> = (0..cols).filter(|c| !pivots.contains(c)).collect();
    if free.len() != 1 {
        return None;
    }
    let fcol = free[0];
    let mut x = vec![Frac(0, 1); cols];
    x[fcol] = Frac(1, 1);
    for (r, &pc) in pivots.iter().enumerate() {
        x[pc] = Frac::new(-a[r][fcol].0, a[r][fcol].1);
    }
    let lcm = x.iter().fold(1i128, |l, f| l / gcd(l, f.1) * f.1);
    let ints: Vec<i128> = x.iter().map(|f| f.0 * (lcm / f.1)).collect();
    let g = ints.iter().fold(0i128, |g, v| gcd(g, *v));
    let sign = if ints.iter().any(|v| *v < 0) && ints.iter().all(|v| *v <= 0) {
        -1
    } else {
        1
    };
    let out: Vec<i128> = ints.iter().map(|v| sign * v / g.max(1)).collect();
    if out.iter().any(|v| *v <= 0) {
        return None;
    }
    Some(out.iter().map(|v| *v as u64).collect())
}

/// Balanced equation as `\ce`-style TeX ready for [`crate::tex::chem`].
pub fn balanced_equation(reactants: &[&str], products: &[&str]) -> Option<String> {
    let c = balance(reactants, products)?;
    let fmt = |species: &[&str], coefs: &[u64]| -> String {
        species
            .iter()
            .zip(coefs)
            .map(|(s, k)| {
                if *k == 1 {
                    s.to_string()
                } else {
                    format!("{k}{s}")
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
    };
    Some(format!(
        "{} -> {}",
        fmt(reactants, &c[..reactants.len()]),
        fmt(products, &c[reactants.len()..])
    ))
}

// --------------------------------------------------------------- molecules

/// CPK colour for an element symbol.
pub fn cpk_color(sym: &str) -> Color {
    match sym {
        "H" => rgb8(240, 240, 240),
        "C" => rgb8(70, 70, 76),
        "N" => rgb8(48, 80, 248),
        "O" => rgb8(235, 45, 40),
        "F" | "Cl" => rgb8(31, 220, 31),
        "Br" => rgb8(160, 34, 34),
        "I" => rgb8(148, 0, 148),
        "S" => rgb8(250, 210, 40),
        "P" => rgb8(255, 128, 0),
        "Na" | "K" | "Li" => rgb8(171, 92, 242),
        "Mg" | "Ca" => rgb8(61, 220, 0),
        "Fe" => rgb8(224, 102, 51),
        "Cu" => rgb8(200, 128, 51),
        "Zn" => rgb8(125, 128, 176),
        "He" | "Ne" | "Ar" => rgb8(100, 230, 255),
        _ => rgb8(255, 120, 200),
    }
}

/// Covalent radius in Å.
pub fn covalent_radius(sym: &str) -> f64 {
    match sym {
        "H" => 0.31,
        "C" => 0.76,
        "N" => 0.71,
        "O" => 0.66,
        "F" => 0.57,
        "Cl" => 1.02,
        "Br" => 1.20,
        "I" => 1.39,
        "S" => 1.05,
        "P" => 1.07,
        "Na" => 1.66,
        "K" => 2.03,
        "Mg" => 1.41,
        "Ca" => 1.76,
        "Fe" => 1.32,
        _ => 1.2,
    }
}

/// A 2D structural formula.
#[derive(Clone, Debug, Default)]
pub struct Molecule2D {
    pub atoms: Vec<(String, Vec2)>,
    /// (a, b, order) with order 1/2/3, or 4 = aromatic-dashed, 5 = wedge, 6 = hash.
    pub bonds: Vec<(usize, usize, u8)>,
    /// Draw a circle inside aromatic rings (centre, radius).
    pub rings: Vec<(Vec2, f64)>,
}

impl Molecule2D {
    pub fn atom(&mut self, sym: &str, p: Vec2) -> usize {
        self.atoms.push((sym.to_string(), p));
        self.atoms.len() - 1
    }
    pub fn bond(&mut self, a: usize, b: usize, order: u8) {
        self.bonds.push((a, b, order));
    }
    /// Render; `skeletal` hides carbon labels (and H attached to C).
    pub fn render(&self, scale: f64, skeletal: bool) -> Mobject {
        // carbons are implicit in skeletal formulas unless isolated from other carbons
        let shown = |i: usize| -> bool {
            let s = &self.atoms[i].0;
            !(skeletal && s == "C" && self.neighbors(i).iter().any(|j| self.atoms[*j].0 == "C"))
        };
        let pos = |i: usize| self.atoms[i].1 * scale;
        let mut bonds = Mobject::empty();
        for &(a, b, order) in &self.bonds {
            let ch =
                |x: usize, y: usize| self.atoms[x].0 == "H" && self.atoms[y].0 == "C" && !shown(y);
            if skeletal && (ch(a, b) || ch(b, a)) {
                continue;
            }
            let (pa, pb) = (pos(a), pos(b));
            let d = (pb - pa).normalize();
            let trim = 0.28 * scale.min(1.5);
            let pa2 = if shown(a) { pa + d * trim } else { pa };
            let pb2 = if shown(b) { pb - d * trim } else { pb };
            let n = d.perp() * (0.09 * scale.min(1.5));
            match order {
                2 => {
                    bonds.push(line(pa2 + n, pb2 + n));
                    bonds.push(line(pa2 - n, pb2 - n));
                }
                3 => {
                    bonds.push(line(pa2, pb2));
                    bonds.push(line(pa2 + n * 1.6, pb2 + n * 1.6));
                    bonds.push(line(pa2 - n * 1.6, pb2 - n * 1.6));
                }
                4 => {
                    bonds.push(line(pa2, pb2));
                    bonds.push(dashed_line(pa2 + n * 1.4, pb2 + n * 1.4));
                }
                5 => bonds.push(
                    polygon(&[pa2, pb2 + n * 1.2, pb2 - n * 1.2])
                        .fill(WHITE, 1.0)
                        .no_stroke(),
                ),
                6 => {
                    for k in 1..7 {
                        let t = k as f64 / 7.0;
                        let p = pa2.lerp(pb2, t);
                        bonds.push(line(p + n * (1.2 * t), p - n * (1.2 * t)));
                    }
                }
                _ => bonds.push(line(pa2, pb2)),
            }
        }
        let bonds = bonds.stroke(GRAY_A, 3.5);
        let mut g = Mobject::group([bonds]);
        for (c, r) in &self.rings {
            g.push(circle_at(*c * scale, r * scale).stroke(GRAY_A, 3.0));
        }
        for (i, (s, _)) in self.atoms.iter().enumerate() {
            if !shown(i)
                || (skeletal
                    && s == "H"
                    && self
                        .neighbors(i)
                        .iter()
                        .all(|j| self.atoms[*j].0 == "C" && !shown(*j)))
            {
                continue;
            }
            let c = if s == "H" || s == "C" {
                WHITE
            } else {
                cpk_color(s)
            };
            g.push(
                text_sized(s, 0.42 * scale.min(1.5), c)
                    .move_to(pos(i))
                    .with_tag(s),
            );
        }
        g
    }
    fn neighbors(&self, i: usize) -> Vec<usize> {
        self.bonds
            .iter()
            .filter_map(|&(a, b, _)| {
                if a == i {
                    Some(b)
                } else if b == i {
                    Some(a)
                } else {
                    None
                }
            })
            .collect()
    }
}

fn hexagon_points(r: f64, start: f64) -> Vec<Vec2> {
    (0..6)
        .map(|i| Vec2::polar(r, start + TAU * i as f64 / 6.0))
        .collect()
}

/// Prebuilt 2D structures: water, carbon_dioxide, methane, ammonia, ethane, ethene,
/// ethyne, ethanol, acetic_acid, benzene, glucose (Haworth-free chain sketch skipped).
pub fn molecule_2d(name: &str) -> Option<Molecule2D> {
    let mut m = Molecule2D::default();
    match name.to_ascii_lowercase().as_str() {
        "water" | "h2o" => {
            let o = m.atom("O", Vec2::ZERO);
            let a = 104.5 * DEGREES / 2.0;
            let h1 = m.atom("H", Vec2::polar(1.0, -PI / 2.0 - a));
            let h2 = m.atom("H", Vec2::polar(1.0, -PI / 2.0 + a));
            m.bond(o, h1, 1);
            m.bond(o, h2, 1);
        }
        "carbon_dioxide" | "co2" => {
            let c = m.atom("C", Vec2::ZERO);
            let o1 = m.atom("O", LEFT * 1.2);
            let o2 = m.atom("O", RIGHT * 1.2);
            m.bond(c, o1, 2);
            m.bond(c, o2, 2);
        }
        "methane" | "ch4" => {
            let c = m.atom("C", Vec2::ZERO);
            for d in [UP, DOWN, LEFT, RIGHT] {
                let h = m.atom("H", d);
                m.bond(c, h, 1);
            }
        }
        "ammonia" | "nh3" => {
            let n = m.atom("N", Vec2::ZERO);
            for a in [-PI / 2.0, PI / 6.0, 5.0 * PI / 6.0] {
                let h = m.atom("H", Vec2::polar(1.0, a));
                m.bond(n, h, 1);
            }
        }
        "ethane" | "ethene" | "ethylene" | "ethyne" | "acetylene" => {
            let order = match name {
                "ethane" => 1,
                "ethene" | "ethylene" => 2,
                _ => 3,
            };
            let c1 = m.atom("C", LEFT * 0.65);
            let c2 = m.atom("C", RIGHT * 0.65);
            m.bond(c1, c2, order);
            let hs: &[(f64, f64)] = match order {
                1 => &[
                    (-1.4, 0.9),
                    (-1.4, -0.9),
                    (-1.9, 0.0),
                    (1.4, 0.9),
                    (1.4, -0.9),
                    (1.9, 0.0),
                ],
                2 => &[(-1.3, 0.85), (-1.3, -0.85), (1.3, 0.85), (1.3, -0.85)],
                _ => &[(-1.75, 0.0), (1.75, 0.0)],
            };
            for &(x, y) in hs {
                let h = m.atom("H", v2(x, y));
                m.bond(if x < 0.0 { c1 } else { c2 }, h, 1);
            }
        }
        "ethanol" => {
            let c1 = m.atom("C", v2(-1.2, 0.0));
            let c2 = m.atom("C", v2(0.0, 0.5));
            let o = m.atom("O", v2(1.2, 0.0));
            let h = m.atom("H", v2(2.0, 0.5));
            m.bond(c1, c2, 1);
            m.bond(c2, o, 1);
            m.bond(o, h, 1);
            for (c, ps) in [
                (c1, vec![v2(-2.0, 0.5), v2(-1.2, -1.0), v2(-2.0, -0.5)]),
                (c2, vec![v2(-0.3, 1.5), v2(0.3, 1.5)]),
            ] {
                for p in ps {
                    let hh = m.atom("H", p);
                    m.bond(c, hh, 1);
                }
            }
        }
        "acetic_acid" | "ch3cooh" => {
            let c1 = m.atom("C", v2(-1.0, 0.0));
            let c2 = m.atom("C", v2(0.2, 0.0));
            let o1 = m.atom("O", v2(0.8, 1.0));
            let o2 = m.atom("O", v2(0.9, -1.0));
            let h = m.atom("H", v2(1.8, -1.0));
            m.bond(c1, c2, 1);
            m.bond(c2, o1, 2);
            m.bond(c2, o2, 1);
            m.bond(o2, h, 1);
            for p in [v2(-1.8, 0.6), v2(-1.8, -0.6), v2(-1.0, 1.0)] {
                let hh = m.atom("H", p);
                m.bond(c1, hh, 1);
            }
        }
        "benzene" | "c6h6" => {
            let ring = hexagon_points(1.0, PI / 2.0);
            let ids: Vec<usize> = ring.iter().map(|p| m.atom("C", *p)).collect();
            for i in 0..6 {
                m.bond(ids[i], ids[(i + 1) % 6], if i % 2 == 0 { 2 } else { 1 });
                let h = m.atom("H", ring[i] * 1.75);
                m.bond(ids[i], h, 1);
            }
        }
        _ => return None,
    }
    Some(m)
}

/// A 3D molecule for ball-and-stick rendering (coordinates in Å).
#[derive(Clone, Debug, Default)]
pub struct Molecule3D {
    pub atoms: Vec<(String, Vec3)>,
    pub bonds: Vec<(usize, usize, u8)>,
}

impl Molecule3D {
    /// Parse XYZ text (`N`, comment, then `Sym x y z` lines); bonds are inferred from
    /// covalent radii.
    pub fn from_xyz(text: &str) -> Molecule3D {
        let mut m = Molecule3D::default();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(z)) =
                    (parts[1].parse(), parts[2].parse(), parts[3].parse())
                {
                    m.atoms.push((parts[0].to_string(), v3(x, y, z)));
                }
            }
        }
        m.infer_bonds();
        m
    }
    pub fn infer_bonds(&mut self) {
        self.bonds.clear();
        for i in 0..self.atoms.len() {
            for j in i + 1..self.atoms.len() {
                let d = (self.atoms[i].1 - self.atoms[j].1).length();
                let lim =
                    (covalent_radius(&self.atoms[i].0) + covalent_radius(&self.atoms[j].0)) * 1.2;
                if d < lim && d > 0.4 {
                    self.bonds.push((i, j, 1));
                }
            }
        }
    }
    /// Ball-and-stick mesh, `scale` scene units per Å.
    pub fn mesh(&self, scale: f64) -> Mesh {
        let mut m = Mesh::new();
        for (s, p) in &self.atoms {
            m.balls.push(Ball {
                center: *p * scale,
                radius: (0.18 + 0.28 * covalent_radius(s)) * scale,
                color: cpk_color(s),
            });
        }
        for &(a, b, order) in &self.bonds {
            let (pa, pb) = (self.atoms[a].1 * scale, self.atoms[b].1 * scale);
            let r = 0.07 * scale;
            let axis = (pb - pa).normalize();
            let side = axis
                .cross(if axis.z.abs() < 0.9 { Vec3::Z } else { Vec3::X })
                .normalize();
            let offsets: Vec<f64> = match order {
                2 => vec![-1.2, 1.2],
                3 => vec![-2.0, 0.0, 2.0],
                _ => vec![0.0],
            };
            for o in offsets {
                let off = side * (o * r);
                let mid = (pa + pb) / 2.0 + off;
                m.add(stick(pa + off, mid, r, cpk_color(&self.atoms[a].0)));
                m.add(stick(mid, pb + off, r, cpk_color(&self.atoms[b].0)));
            }
        }
        m
    }
}

/// Shaded cylinder between two points (bond stick).
pub fn stick(a: Vec3, b: Vec3, r: f64, color: Color) -> Mesh {
    let axis = (b - a).normalize();
    let e1 = axis
        .cross(if axis.z.abs() < 0.9 { Vec3::Z } else { Vec3::X })
        .normalize();
    let e2 = axis.cross(e1);
    let n = 10;
    let mut m = Mesh::new();
    for i in 0..n {
        let a0 = TAU * i as f64 / n as f64;
        let a1 = TAU * (i + 1) as f64 / n as f64;
        let o0 = (e1 * a0.cos() + e2 * a0.sin()) * r;
        let o1 = (e1 * a1.cos() + e2 * a1.sin()) * r;
        m.faces.push(Face {
            verts: vec![a + o0, a + o1, b + o1, b + o0],
            color,
            opacity: 1.0,
            stroke: None,
        });
    }
    m
}

/// Prebuilt 3D molecules: water, methane, ammonia, carbon_dioxide, ethene, benzene, ethanol.
pub fn molecule_3d(name: &str) -> Option<Molecule3D> {
    let xyz = match name.to_ascii_lowercase().as_str() {
        "water" | "h2o" => "O 0 0 0\nH 0.757 0.586 0\nH -0.757 0.586 0",
        "methane" | "ch4" => "C 0 0 0\nH 0.629 0.629 0.629\nH -0.629 -0.629 0.629\nH -0.629 0.629 -0.629\nH 0.629 -0.629 -0.629",
        "ammonia" | "nh3" => "N 0 0 0.12\nH 0 0.94 -0.27\nH 0.814 -0.47 -0.27\nH -0.814 -0.47 -0.27",
        "carbon_dioxide" | "co2" => "C 0 0 0\nO 1.16 0 0\nO -1.16 0 0",
        "ethene" | "ethylene" => "C 0.667 0 0\nC -0.667 0 0\nH 1.23 0.92 0\nH 1.23 -0.92 0\nH -1.23 0.92 0\nH -1.23 -0.92 0",
        "benzene" | "c6h6" => {
            let mut s = String::new();
            for i in 0..6 {
                let a = TAU * i as f64 / 6.0;
                s.push_str(&format!("C {} {} 0\n", 1.39 * a.cos(), 1.39 * a.sin()));
                s.push_str(&format!("H {} {} 0\n", 2.48 * a.cos(), 2.48 * a.sin()));
            }
            return Some({
                let mut m = Molecule3D::from_xyz(&s);
                // alternate double bonds in the ring
                let mut ring = 0;
                for b in &mut m.bonds {
                    if m.atoms[b.0].0 == "C" && m.atoms[b.1].0 == "C" {
                        if ring % 2 == 0 {
                            b.2 = 2;
                        }
                        ring += 1;
                    }
                }
                m
            });
        }
        "ethanol" => "C -1.17 -0.40 0\nC 0.17 0.30 0\nO 1.20 -0.62 0\nH 2.03 -0.14 0\nH -1.29 -1.03 0.88\nH -1.29 -1.03 -0.88\nH -1.97 0.34 0\nH 0.28 0.94 0.88\nH 0.28 0.94 -0.88",
        _ => return None,
    };
    let mut m = Molecule3D::from_xyz(xyz);
    if name.eq_ignore_ascii_case("carbon_dioxide") || name.eq_ignore_ascii_case("co2") {
        for b in &mut m.bonds {
            b.2 = 2;
        }
    }
    if name.eq_ignore_ascii_case("ethene") || name.eq_ignore_ascii_case("ethylene") {
        for b in &mut m.bonds {
            if m.atoms[b.0].0 == "C" && m.atoms[b.1].0 == "C" {
                b.2 = 2;
            }
        }
    }
    Some(m)
}

/// Ionic crystal lattice (e.g. NaCl) as alternating coloured spheres in 3D.
pub fn ionic_lattice(n: usize, spacing: f64, a: Color, b: Color) -> Mesh {
    let mut m = Mesh::new();
    let off = (n as f64 - 1.0) / 2.0;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let p = v3(i as f64 - off, j as f64 - off, k as f64 - off) * spacing;
                let big = (i + j + k) % 2 == 0;
                m.balls.push(Ball {
                    center: p,
                    radius: if big { spacing * 0.32 } else { spacing * 0.2 },
                    color: if big { a } else { b },
                });
                for (di, dj, dk) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
                    if i + di < n && j + dj < n && k + dk < n {
                        let q = p + v3(di as f64, dj as f64, dk as f64) * spacing;
                        m.segs.push(crate::three_d::Seg3 {
                            a: p,
                            b: q,
                            color: GRAY_C,
                            width: 1.5,
                        });
                    }
                }
            }
        }
    }
    m
}

// ---------------------------------------------------------------- glassware

fn liquid_clip(outline: &[Vec2], level_y: f64) -> Vec<Vec2> {
    // keep the part of a convex-ish outline below the liquid level
    let mut out = Vec::new();
    let n = outline.len();
    for i in 0..n {
        let a = outline[i];
        let b = outline[(i + 1) % n];
        if a.y <= level_y {
            out.push(a);
        }
        if (a.y <= level_y) != (b.y <= level_y) {
            let t = (level_y - a.y) / (b.y - a.y);
            out.push(a.lerp(b, t));
        }
    }
    out
}

/// Beaker with liquid filled to `level` (0..1).
pub fn beaker(width: f64, height: f64, level: f64, liquid: Color) -> Mobject {
    let (w, h) = (width / 2.0, height / 2.0);
    let outline = vec![
        v2(-w, h),
        v2(-w, -h + 0.1),
        v2(-w + 0.1, -h),
        v2(w - 0.1, -h),
        v2(w, -h + 0.1),
        v2(w, h),
    ];
    let glass = polyline(&outline).stroke(GRAY_A, 4.0);
    let lip = line(v2(-w - 0.12, h), v2(-w, h - 0.08)).stroke(GRAY_A, 4.0);
    let ly = -h + height * level.clamp(0.0, 1.0) * 0.92;
    let inner: Vec<Vec2> = vec![
        v2(-w + 0.04, h),
        v2(-w + 0.04, -h + 0.06),
        v2(w - 0.04, -h + 0.06),
        v2(w - 0.04, h),
    ];
    let liq = polygon(&liquid_clip(&inner, ly))
        .fill(liquid, 0.65)
        .no_stroke();
    let mut g = Mobject::group([liq.with_tag("liquid"), glass, lip]);
    for i in 1..5 {
        let y = -h + height * i as f64 / 5.0;
        g.push(line(v2(-w, y), v2(-w + 0.25, y)).stroke(GRAY_B, 2.0));
    }
    g
}

/// Erlenmeyer (conical) flask with liquid.
pub fn erlenmeyer_flask(size: f64, level: f64, liquid: Color) -> Mobject {
    let s = size;
    let outline = vec![
        v2(-0.18 * s, 0.6 * s),
        v2(-0.18 * s, 0.2 * s),
        v2(-0.5 * s, -0.5 * s),
        v2(0.5 * s, -0.5 * s),
        v2(0.18 * s, 0.2 * s),
        v2(0.18 * s, 0.6 * s),
    ];
    let glass = polyline(&outline).stroke(GRAY_A, 4.0);
    let ly = -0.5 * s + s * 0.7 * level.clamp(0.0, 1.0);
    let liq = polygon(&liquid_clip(&outline, ly))
        .fill(liquid, 0.65)
        .no_stroke();
    Mobject::group([liq.with_tag("liquid"), glass])
}

/// Test tube with liquid.
pub fn test_tube(height: f64, level: f64, liquid: Color) -> Mobject {
    let w = height * 0.09;
    let mut p = Path::new();
    p.move_to(v2(-w, height / 2.0));
    p.line_to(v2(-w, -height / 2.0 + w));
    p.arc_to_center(v2(0.0, -height / 2.0 + w), PI);
    p.line_to(v2(w, height / 2.0));
    let glass = Mobject::from_path(p).stroke(GRAY_A, 4.0);
    let ly = -height / 2.0 + height * level.clamp(0.0, 1.0);
    let mut lp = Path::new();
    lp.move_to(v2(-w + 0.03, ly.max(-height / 2.0 + w)));
    lp.line_to(v2(-w + 0.03, -height / 2.0 + w));
    lp.arc_to_center(v2(0.0, -height / 2.0 + w), PI);
    lp.line_to(v2(w - 0.03, ly.max(-height / 2.0 + w)));
    lp.close();
    Mobject::group([
        Mobject::from_path(lp)
            .fill(liquid, 0.65)
            .no_stroke()
            .with_tag("liquid"),
        glass,
    ])
}

/// Burette with graduations and stopcock.
pub fn burette(height: f64, level: f64, liquid: Color) -> Mobject {
    let w = 0.12;
    let top = height / 2.0;
    let bot = -height / 2.0;
    let ly = bot + 0.3 + (height - 0.3) * level.clamp(0.0, 1.0);
    let mut g = Mobject::group([
        rectangle(w * 1.6, ly - (bot + 0.3))
            .move_to(v2(0.0, (ly + bot + 0.3) / 2.0))
            .fill(liquid, 0.6)
            .no_stroke(),
        line(v2(-w, top), v2(-w, bot + 0.3)).stroke(GRAY_A, 3.0),
        line(v2(w, top), v2(w, bot + 0.3)).stroke(GRAY_A, 3.0),
        polyline(&[
            v2(-w, bot + 0.3),
            v2(-0.03, bot),
            v2(0.03, bot),
            v2(w, bot + 0.3),
        ])
        .stroke(GRAY_A, 3.0),
        rectangle(0.4, 0.08)
            .move_to(v2(0.0, bot + 0.18))
            .fill(GRAY_B, 1.0)
            .no_stroke(),
    ]);
    for i in 0..10 {
        let y = top - 0.2 - (height - 0.6) * i as f64 / 10.0;
        g.push(line(v2(-w, y), v2(-w + 0.1, y)).stroke(GRAY_B, 1.5));
    }
    g
}

/// Bunsen burner with flame (flicker with `t`).
pub fn bunsen_burner(size: f64, t: f64) -> Mobject {
    let s = size;
    let flick = 1.0 + 0.08 * (t * 13.0).sin() + 0.05 * (t * 7.3).cos();
    let flame_outer = smooth_curve(
        &[
            v2(-0.12 * s, 0.5 * s),
            v2(-0.1 * s, 0.75 * s),
            v2(0.0, (0.5 + 0.55 * flick) * s),
            v2(0.1 * s, 0.75 * s),
            v2(0.12 * s, 0.5 * s),
        ],
        true,
    )
    .no_stroke()
    .fill(BLUE_B, 0.7)
    .glow(0.15 * s, Some(BLUE));
    let flame_inner = smooth_curve(
        &[
            v2(-0.06 * s, 0.5 * s),
            v2(0.0, (0.5 + 0.28 * flick) * s),
            v2(0.06 * s, 0.5 * s),
        ],
        true,
    )
    .no_stroke()
    .fill(BLUE_E, 0.9);
    Mobject::group([
        rectangle(0.12 * s, 0.8 * s)
            .move_to(v2(0.0, 0.1 * s))
            .fill(GRAY_C, 1.0)
            .no_stroke(),
        rounded_rectangle(0.6 * s, 0.12 * s, 0.04 * s)
            .move_to(v2(0.0, -0.32 * s))
            .fill(GRAY_D, 1.0)
            .no_stroke(),
        flame_outer,
        flame_inner,
    ])
}

/// Rising bubbles inside a region (animate with `t`).
pub fn bubbles(region: Rect, count: usize, t: f64, seed: u64) -> Mobject {
    let mut rng = Rng::new(seed);
    let mut g = Mobject::empty();
    for _ in 0..count {
        let x = rng.range(region.min.x + 0.1, region.max.x - 0.1);
        let speed = rng.range(0.3, 0.8);
        let phase = rng.range(0.0, 1.0);
        let r = rng.range(0.03, 0.08);
        let y = region.min.y + ((t * speed / region.height() + phase).fract()) * region.height();
        let wobble = 0.05 * (t * 5.0 + phase * 10.0).sin();
        g.push(
            circle_at(v2(x + wobble, y), r)
                .stroke(WHITE, 1.5)
                .stroke_opacity(0.8),
        );
    }
    g
}

/// pH scale bar 0–14 with a marker.
pub fn ph_scale(width: f64, ph: f64) -> Mobject {
    let colors = [RED, ORANGE, YELLOW, GREEN, TEAL, BLUE, PURPLE];
    let bar = rectangle(width, 0.4)
        .gradient(&colors, RIGHT)
        .fill_opacity(1.0)
        .no_stroke();
    let mut g = Mobject::group([bar]);
    for i in 0..=14 {
        let x = -width / 2.0 + width * i as f64 / 14.0;
        g.push(text_sized(&i.to_string(), 0.22, WHITE).move_to(v2(x, -0.42)));
    }
    let x = -width / 2.0 + width * ph.clamp(0.0, 14.0) / 14.0;
    g.push(
        triangle(0.15)
            .rotate(PI)
            .move_to(v2(x, 0.4))
            .fill(WHITE, 1.0)
            .no_stroke(),
    );
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_and_layout() {
        assert_eq!(element(1).unwrap().symbol, "H");
        assert_eq!(element(118).unwrap().symbol, "Og");
        assert_eq!(element_by_symbol("Fe").unwrap().z, 26);
        assert_eq!(element(26).unwrap().group, 8);
        assert_eq!(element(17).unwrap().group, 17);
        assert_eq!(element(79).unwrap().group, 11);
        assert_eq!(element(118).unwrap().period, 7);
        assert_eq!(table_position(58), (4, 8));
        assert_eq!(table_position(92), (6, 9));
        assert_eq!(periodic_table(0.5).children.len(), 118);
        // configurations
        assert_eq!(shells(11), vec![2, 8, 1]);
        assert_eq!(shells(26), vec![2, 8, 14, 2]);
        assert_eq!(
            electron_configuration_tex(24, true),
            "[\\mathrm{Ar}]\\,4s^{1}\\,3d^{5}"
        );
        let total: u32 = electron_configuration(92).iter().map(|e| e.2).sum();
        assert_eq!(total, 92);
        assert_eq!(valence_electrons(8), 6);
        assert!(bohr_atom(11, 2.0, 0.3).leaf_count() > 11);
        assert!(orbital_box_diagram(8, 0.4).leaf_count() > 5);
    }

    #[test]
    fn formulas_and_balancing() {
        let f = parse_formula("Ca(OH)2");
        assert_eq!(f["Ca"], 1);
        assert_eq!(f["O"], 2);
        assert_eq!(f["H"], 2);
        let h = parse_formula("CuSO4·5H2O");
        assert_eq!(h["O"], 9);
        assert_eq!(h["H"], 10);
        assert!((molar_mass("H2O") - 18.015).abs() < 0.01);
        assert!((molar_mass("C6H12O6") - 180.156).abs() < 0.05);
        assert_eq!(balance(&["H2", "O2"], &["H2O"]), Some(vec![2, 1, 2]));
        assert_eq!(
            balance(&["C3H8", "O2"], &["CO2", "H2O"]),
            Some(vec![1, 5, 3, 4])
        );
        assert_eq!(balance(&["Fe", "O2"], &["Fe2O3"]), Some(vec![4, 3, 2]));
        assert_eq!(
            balance(&["KMnO4", "HCl"], &["KCl", "MnCl2", "H2O", "Cl2"]),
            Some(vec![2, 16, 2, 2, 8, 5])
        );
        assert_eq!(
            balanced_equation(&["H2", "O2"], &["H2O"]).unwrap(),
            "2H2 + O2 -> 2H2O"
        );
    }

    #[test]
    fn molecules_and_glassware() {
        for n in [
            "water",
            "co2",
            "methane",
            "ammonia",
            "ethane",
            "ethene",
            "ethyne",
            "ethanol",
            "acetic_acid",
            "benzene",
        ] {
            let m = molecule_2d(n).unwrap();
            assert!(m.render(1.0, false).leaf_count() > 2, "{n}");
            assert!(m.render(1.0, true).leaf_count() >= 1, "{n}");
        }
        for n in [
            "water", "methane", "ammonia", "co2", "ethene", "benzene", "ethanol",
        ] {
            let m = molecule_3d(n).unwrap();
            assert!(!m.bonds.is_empty(), "{n}");
            assert!(
                m.mesh(1.0)
                    .render(&crate::three_d::Camera3D::default())
                    .leaf_count()
                    > 3
            );
        }
        assert_eq!(molecule_3d("methane").unwrap().bonds.len(), 4);
        assert_eq!(molecule_3d("benzene").unwrap().bonds.len(), 12);
        for m in [
            beaker(2.0, 2.5, 0.6, BLUE),
            erlenmeyer_flask(2.0, 0.4, GREEN),
            test_tube(2.0, 0.5, RED),
            burette(4.0, 0.7, BLUE),
            bunsen_burner(1.5, 0.3),
            bubbles(Rect::from_center(Vec2::ZERO, 1.0, 2.0), 10, 1.0, 1),
            ph_scale(6.0, 7.0),
            lewis_dot("O", 6, 0.6),
            orbital_shape("dxy", 1.0),
            orbital_shape("pz", 1.0),
            element_tile(26, 1.5),
        ] {
            assert!(m.leaf_count() > 0);
        }
        assert!(ionic_lattice(3, 1.0, GREEN, PURPLE).balls.len() == 27);
    }
}
