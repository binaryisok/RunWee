//! Recorded canvas commands → [`Canvas2D`] replay.

use runwee::canvas2d::{css_color, Canvas2D, CanvasPaint};
use runwee::color::Color;
use runwee::math::v2;
use serde_json::Value;

/// One recorded canvas call (already parsed, so replay is cheap).
#[derive(Clone, Debug)]
pub enum Cmd {
    Save,
    Restore,
    ResetAll,
    Translate(f64, f64),
    Rotate(f64),
    Scale(f64, f64),
    Transform([f64; 6]),
    SetTransform([f64; 6]),
    BeginPath,
    ClosePath,
    MoveTo(f64, f64),
    LineTo(f64, f64),
    Quad(f64, f64, f64, f64),
    Bezier([f64; 6]),
    Arc(f64, f64, f64, f64, f64, bool),
    ArcTo(f64, f64, f64, f64, f64),
    Ellipse([f64; 7], bool),
    Rect(f64, f64, f64, f64),
    RoundRect(f64, f64, f64, f64, Vec<f64>),
    Fill(bool),
    Stroke,
    Clip,
    FillRect(f64, f64, f64, f64),
    StrokeRect(f64, f64, f64, f64),
    ClearRect(f64, f64, f64, f64),
    FillText(String, f64, f64),
    StrokeText(String, f64, f64),
    Dash(Vec<f64>),
    FillStyle(CanvasPaint),
    StrokeStyle(CanvasPaint),
    ShadowColor(String),
    LineWidth(f64),
    Font(String),
    TextAlign(String),
    TextBaseline(String),
    GlobalAlpha(f64),
    LineCap(String),
    LineJoin(String),
    LineDashOffset(f64),
    ShadowBlur(f64),
    Composite(String),
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn gradient(kind: &Value, args: &Value, stops: &Value) -> Option<CanvasPaint> {
    let a: Vec<f64> = args.as_array()?.iter().map(num).collect();
    let stops: Vec<(f32, Color)> = stops
        .as_array()?
        .iter()
        .filter_map(|s| {
            let s = s.as_array()?;
            Some((num(s.first()?) as f32, css_color(s.get(1)?.as_str()?)?))
        })
        .collect();
    if stops.is_empty() {
        return None;
    }
    match kind.as_str()? {
        "radial" if a.len() >= 6 => Some(CanvasPaint::Radial {
            c0: v2(a[0], a[1]),
            r0: a[2],
            c1: v2(a[3], a[4]),
            r1: a[5],
            stops,
        }),
        _ if a.len() >= 4 => Some(CanvasPaint::Linear {
            p0: v2(a[0], a[1]),
            p1: v2(a[2], a[3]),
            stops,
        }),
        _ => None,
    }
}

/// Parse the harness's JSON command list.
pub fn parse(json: &str) -> Vec<Cmd> {
    let Ok(Value::Array(list)) = serde_json::from_str::<Value>(json) else {
        return vec![];
    };
    let mut out = Vec::with_capacity(list.len());
    for c in &list {
        let Some(a) = c.as_array() else { continue };
        let Some(op) = a.first().and_then(|v| v.as_str()) else {
            continue;
        };
        let n = |i: usize| a.get(i).map(num).unwrap_or(f64::NAN);
        let s = |i: usize| a.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let b = |i: usize| a.get(i).and_then(|v| v.as_bool()).unwrap_or(false);
        let six = |o: usize| [n(o), n(o + 1), n(o + 2), n(o + 3), n(o + 4), n(o + 5)];
        let cmd = match op {
            "save" => Cmd::Save,
            "restore" => Cmd::Restore,
            "resetAll" => Cmd::ResetAll,
            "translate" => Cmd::Translate(n(1), n(2)),
            "rotate" => Cmd::Rotate(n(1)),
            "scale" => Cmd::Scale(n(1), n(2)),
            "transform" => Cmd::Transform(six(1)),
            "setTransform" => Cmd::SetTransform(six(1)),
            "beginPath" => Cmd::BeginPath,
            "closePath" => Cmd::ClosePath,
            "moveTo" => Cmd::MoveTo(n(1), n(2)),
            "lineTo" => Cmd::LineTo(n(1), n(2)),
            "quad" => Cmd::Quad(n(1), n(2), n(3), n(4)),
            "bezier" => Cmd::Bezier(six(1)),
            "arc" => Cmd::Arc(n(1), n(2), n(3), n(4), n(5), b(6)),
            "arcTo" => Cmd::ArcTo(n(1), n(2), n(3), n(4), n(5)),
            "ellipse" => Cmd::Ellipse([n(1), n(2), n(3), n(4), n(5), n(6), n(7)], b(8)),
            "rect" => Cmd::Rect(n(1), n(2), n(3), n(4)),
            "roundRect" => Cmd::RoundRect(
                n(1),
                n(2),
                n(3),
                n(4),
                a.get(5)
                    .and_then(|v| v.as_array())
                    .map(|r| r.iter().map(num).collect())
                    .unwrap_or_default(),
            ),
            "fill" => Cmd::Fill(s(1) == "evenodd"),
            "stroke" => Cmd::Stroke,
            "clip" => Cmd::Clip,
            "fillRect" => Cmd::FillRect(n(1), n(2), n(3), n(4)),
            "strokeRect" => Cmd::StrokeRect(n(1), n(2), n(3), n(4)),
            "clearRect" => Cmd::ClearRect(n(1), n(2), n(3), n(4)),
            "fillText" => Cmd::FillText(s(1), n(2), n(3)),
            "strokeText" => Cmd::StrokeText(s(1), n(2), n(3)),
            "dash" => Cmd::Dash(
                a.get(1)
                    .and_then(|v| v.as_array())
                    .map(|r| r.iter().map(num).collect())
                    .unwrap_or_default(),
            ),
            "fillStyle" => match css_color(&s(1)) {
                Some(c) => Cmd::FillStyle(CanvasPaint::Color(c)),
                None => continue,
            },
            "strokeStyle" => match css_color(&s(1)) {
                Some(c) => Cmd::StrokeStyle(CanvasPaint::Color(c)),
                None => continue,
            },
            "fillStyleGrad" => match gradient(&a[1], &a[2], &a[3]) {
                Some(g) => Cmd::FillStyle(g),
                None => continue,
            },
            "strokeStyleGrad" => match gradient(&a[1], &a[2], &a[3]) {
                Some(g) => Cmd::StrokeStyle(g),
                None => continue,
            },
            "shadowColor" => Cmd::ShadowColor(s(1)),
            "lineWidth" => Cmd::LineWidth(n(1)),
            "font" => Cmd::Font(s(1)),
            "textAlign" => Cmd::TextAlign(s(1)),
            "textBaseline" => Cmd::TextBaseline(s(1)),
            "globalAlpha" => Cmd::GlobalAlpha(n(1)),
            "lineCap" => Cmd::LineCap(s(1)),
            "lineJoin" => Cmd::LineJoin(s(1)),
            "lineDashOffset" => Cmd::LineDashOffset(n(1)),
            "shadowBlur" => Cmd::ShadowBlur(n(1)),
            "globalCompositeOperation" => Cmd::Composite(s(1)),
            _ => continue,
        };
        out.push(cmd);
    }
    out
}

/// Replay commands onto a canvas.
pub fn replay(cmds: &[Cmd], c: &mut Canvas2D) {
    for cmd in cmds {
        match cmd {
            Cmd::Save => c.save(),
            Cmd::Restore => c.restore(),
            Cmd::ResetAll => c.reset(),
            Cmd::Translate(x, y) => c.translate(*x, *y),
            Cmd::Rotate(a) => c.rotate(*a),
            Cmd::Scale(x, y) => c.scale(*x, *y),
            Cmd::Transform(m) => c.transform(m[0], m[1], m[2], m[3], m[4], m[5]),
            Cmd::SetTransform(m) => c.set_transform(m[0], m[1], m[2], m[3], m[4], m[5]),
            Cmd::BeginPath => c.begin_path(),
            Cmd::ClosePath => c.close_path(),
            Cmd::MoveTo(x, y) => c.move_to(*x, *y),
            Cmd::LineTo(x, y) => c.line_to(*x, *y),
            Cmd::Quad(a, b, x, y) => c.quadratic_curve_to(*a, *b, *x, *y),
            Cmd::Bezier(m) => c.bezier_curve_to(m[0], m[1], m[2], m[3], m[4], m[5]),
            Cmd::Arc(x, y, r, a0, a1, ccw) => c.arc(*x, *y, *r, *a0, *a1, *ccw),
            Cmd::ArcTo(x1, y1, x2, y2, r) => c.arc_to(*x1, *y1, *x2, *y2, *r),
            Cmd::Ellipse(m, ccw) => c.ellipse(m[0], m[1], m[2], m[3], m[4], m[5], m[6], *ccw),
            Cmd::Rect(x, y, w, h) => c.rect(*x, *y, *w, *h),
            Cmd::RoundRect(x, y, w, h, r) => c.round_rect(*x, *y, *w, *h, r),
            Cmd::Fill(evenodd) => c.fill_rule(if *evenodd { "evenodd" } else { "nonzero" }),
            Cmd::Stroke => c.stroke(),
            Cmd::Clip => c.clip(),
            Cmd::FillRect(x, y, w, h) => c.fill_rect(*x, *y, *w, *h),
            Cmd::StrokeRect(x, y, w, h) => c.stroke_rect(*x, *y, *w, *h),
            Cmd::ClearRect(x, y, w, h) => c.clear_rect(*x, *y, *w, *h),
            Cmd::FillText(t, x, y) => c.fill_text(t, *x, *y),
            Cmd::StrokeText(t, x, y) => c.stroke_text(t, *x, *y),
            Cmd::Dash(d) => c.set_line_dash(d),
            Cmd::FillStyle(p) => c.set_fill_style(p.clone()),
            Cmd::StrokeStyle(p) => c.set_stroke_style(p.clone()),
            Cmd::ShadowColor(s) => c.set_shadow_color_css(s),
            Cmd::LineWidth(w) => c.set_line_width(*w),
            Cmd::Font(f) => c.set_font(f),
            Cmd::TextAlign(a) => c.set_text_align(a),
            Cmd::TextBaseline(b) => c.set_text_baseline(b),
            Cmd::GlobalAlpha(a) => c.set_global_alpha(*a),
            Cmd::LineCap(s) => c.set_line_cap(s),
            Cmd::LineJoin(s) => c.set_line_join(s),
            Cmd::LineDashOffset(o) => c.set_line_dash_offset(*o),
            Cmd::ShadowBlur(b) => c.set_shadow_blur(*b),
            Cmd::Composite(s) => c.set_global_composite_operation(s),
        }
    }
}
