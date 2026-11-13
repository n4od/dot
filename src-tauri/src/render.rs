// `Paint::default()` then assigning its fields reads better than a five-field
// struct literal here, and this module has several such paints.
#![allow(clippy::field_reassign_with_default)]

//! Paints a `BotFrame` with tiny-skia, reproducing the three styles from
//! `src/pet/materials.ts`. Geometry comes from the Rust engine; this module only
//! decides fill, exactly as `PetFigure.vue` does for the webview.

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Mask, Paint, Path,
    PathBuilder, Pixmap, Point, RadialGradient, SpreadMode, Stroke, Transform,
};

use crate::engine::{BotFrame, Point as VPoint, RenderedEye};

/// Engine unit: the resting ball radius in viewBox units.
pub const RAYON: f64 = 100.0;
/// Half the displayed viewBox; the margin that holds the orbits.
pub const DEMI: f64 = 158.0;

/* --------------------------------------------------------------- colours */

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let v = u32::from_str_radix(h.get(0..6).unwrap_or("0a0a0c"), 16).unwrap_or(0x0a0a0c);
    (
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
    )
}

fn rgba(hex: &str, alpha: f64) -> Color {
    let (r, g, b) = parse_hex(hex);
    Color::from_rgba8(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Port of `mixHex` from `src/bot/skins.ts`.
fn mix_hex(from: &str, to: &str, t: f64) -> String {
    let (ar, ag, ab) = parse_hex(from);
    let (br, bg, bb) = parse_hex(to);
    let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    format!("#{:02x}{:02x}{:02x}", mix(ar, br), mix(ag, bg), mix(ab, bb))
}

/// Port of `src/contrast.ts`: the eye fill must contrast with the ink.
pub fn eye_fill_for(ink: &str) -> String {
    let (r, g, b) = parse_hex(ink);
    let lum =
        0.2126 * (r as f64 / 255.0) + 0.7152 * (g as f64 / 255.0) + 0.0722 * (b as f64 / 255.0);
    if lum > 0.55 {
        "#111116".into()
    } else {
        "#fdfdfd".into()
    }
}

fn eye_shade(eye_fill: &str, depth: f64) -> String {
    if depth <= 0.0 {
        eye_fill.into()
    } else {
        mix_hex(eye_fill, "#000000", depth)
    }
}

/* ---------------------------------------------------------------- styles */

struct Surface {
    light: String,
    base: String,
    deep: String,
    lx: f64,
    ly: f64,
    reach: f64,
    mid: f64,
    shade: f64,
    gloss: f64,
    gloss_r: f64,
    eye_depth: f64,
    orbits: f64,
}

fn surface_for(style: &str, ink: &str) -> Option<Surface> {
    let rule = crate::engine::catalogue().surfaces.get(style)?;
    let tone = |x: &crate::engine::ToneRule| mix_hex(ink, &x.to, x.t);
    Some(Surface {
        light: tone(&rule.light),
        base: rule
            .base
            .as_ref()
            .map(tone)
            .unwrap_or_else(|| ink.to_string()),
        deep: tone(&rule.deep),
        lx: rule.lx,
        ly: rule.ly,
        reach: rule.reach,
        mid: rule.mid,
        shade: rule.shade,
        gloss: rule.gloss,
        gloss_r: rule.gloss_r,
        eye_depth: rule.eye_depth,
        orbits: rule.orbits,
    })
}

/* ------------------------------------------------------- paths and tokens */

enum Tok {
    Cmd(char),
    Num(f64),
}

fn tokenize(d: &str) -> Vec<Tok> {
    let b = d.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_alphabetic() {
            out.push(Tok::Cmd(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            i += 1;
            while i < b.len() {
                let n = b[i] as char;
                if n.is_ascii_digit() || n == '.' || n == '-' || n == '+' || n == 'e' || n == 'E' {
                    i += 1;
                } else {
                    break;
                }
            }
            if let Ok(v) = d[start..i].parse::<f64>() {
                out.push(Tok::Num(v));
            }
        } else {
            i += 1;
        }
    }
    out
}

fn num(toks: &[Tok], i: &mut usize) -> Option<f64> {
    match toks.get(*i)? {
        Tok::Num(v) => {
            *i += 1;
            Some(*v)
        }
        _ => None,
    }
}

/// Reads the M/L/C/Z commands the engine emits (no arcs: capsules are built
/// directly from their size).
fn parse_path(d: &str) -> Option<Path> {
    let toks = tokenize(d);
    let mut pb = PathBuilder::new();
    let mut i = 0;
    let mut cmd = 'M';
    while i < toks.len() {
        if let Tok::Cmd(c) = toks[i] {
            cmd = c;
            i += 1;
            if cmd == 'Z' {
                pb.close();
                continue;
            }
        }
        match cmd {
            'M' | 'L' => {
                let x = num(&toks, &mut i)? as f32;
                let y = num(&toks, &mut i)? as f32;
                if cmd == 'M' {
                    pb.move_to(x, y);
                    cmd = 'L';
                } else {
                    pb.line_to(x, y);
                }
            }
            'C' => {
                let a = num(&toks, &mut i)? as f32;
                let b = num(&toks, &mut i)? as f32;
                let c = num(&toks, &mut i)? as f32;
                let e = num(&toks, &mut i)? as f32;
                let f = num(&toks, &mut i)? as f32;
                let g = num(&toks, &mut i)? as f32;
                pb.cubic_to(a, b, c, e, f, g);
            }
            _ => i += 1,
        }
    }
    pb.finish()
}

fn empty_path() -> Path {
    let mut pb = PathBuilder::new();
    pb.move_to(0.0, 0.0);
    pb.finish().unwrap()
}

fn circle(r: f64) -> Path {
    let mut pb = PathBuilder::new();
    pb.push_circle(0.0, 0.0, r as f32);
    pb.finish().unwrap_or_else(empty_path)
}

/// Stadium centred on the origin: the exact eye shape.
fn capsule(w: f64, h: f64) -> Path {
    let hw = w.max(0.01) / 2.0;
    let hh = h.max(0.01) / 2.0;
    let r = hw.min(hh);
    let k = 0.552_284_749_830_793_6 * r;
    let mut pb = PathBuilder::new();
    pb.move_to((-hw + r) as f32, -hh as f32);
    pb.line_to((hw - r) as f32, -hh as f32);
    pb.cubic_to(
        (hw - r + k) as f32,
        -hh as f32,
        hw as f32,
        (-hh + r - k) as f32,
        hw as f32,
        (-hh + r) as f32,
    );
    pb.line_to(hw as f32, (hh - r) as f32);
    pb.cubic_to(
        hw as f32,
        (hh - r + k) as f32,
        (hw - r + k) as f32,
        hh as f32,
        (hw - r) as f32,
        hh as f32,
    );
    pb.line_to((-hw + r) as f32, hh as f32);
    pb.cubic_to(
        (-hw + r - k) as f32,
        hh as f32,
        -hw as f32,
        (hh - r + k) as f32,
        -hw as f32,
        (hh - r) as f32,
    );
    pb.line_to(-hw as f32, (-hh + r) as f32);
    pb.cubic_to(
        -hw as f32,
        (-hh + r - k) as f32,
        (-hw + r - k) as f32,
        -hh as f32,
        (-hw + r) as f32,
        -hh as f32,
    );
    pb.close();
    pb.finish().unwrap_or_else(empty_path)
}

/// Catmull-Rom through the silhouette points, in pixels.
fn body_path(points: &[VPoint], s: f64) -> Path {
    let n = points.len();
    let mut pb = PathBuilder::new();
    if n < 3 {
        return pb.finish().unwrap_or_else(empty_path);
    }
    let px = |p: VPoint| (((p.x + DEMI) * s) as f32, ((p.y + DEMI) * s) as f32);
    let (x, y) = px(points[0]);
    pb.move_to(x, y);
    for i in 0..n {
        let p0 = points[(i + n - 1) % n];
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        let p3 = points[(i + 2) % n];
        let c1 = (
            ((p1.x + (p2.x - p0.x) / 6.0 + DEMI) * s) as f32,
            ((p1.y + (p2.y - p0.y) / 6.0 + DEMI) * s) as f32,
        );
        let c2 = (
            ((p2.x - (p3.x - p1.x) / 6.0 + DEMI) * s) as f32,
            ((p2.y - (p3.y - p1.y) / 6.0 + DEMI) * s) as f32,
        );
        let (dx, dy) = px(p2);
        pb.cubic_to(c1.0, c1.1, c2.0, c2.1, dx, dy);
    }
    pb.close();
    pb.finish().unwrap_or_else(empty_path)
}

fn parse_matrix(matrix: &str) -> Option<[f64; 6]> {
    let toks = tokenize(matrix);
    let mut nums = Vec::new();
    for t in toks {
        if let Tok::Num(v) = t {
            nums.push(v);
        }
    }
    if nums.len() < 6 {
        return None;
    }
    Some([nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]])
}

fn px_transform(s: f64) -> Transform {
    Transform::from_row(
        s as f32,
        0.0,
        0.0,
        s as f32,
        (DEMI * s) as f32,
        (DEMI * s) as f32,
    )
}

/// SVG `matrix(a,b,c,d,e,f)` then viewBox -> pixels.
fn eye_transform(m: [f64; 6], s: f64) -> Transform {
    Transform::from_row(
        (s * m[0]) as f32,
        (s * m[1]) as f32,
        (s * m[2]) as f32,
        (s * m[3]) as f32,
        (s * (m[4] + DEMI)) as f32,
        (s * (m[5] + DEMI)) as f32,
    )
}

/* ---------------------------------------------------------------- painting */

fn stroke_arc(pixmap: &mut Pixmap, arc: &crate::engine::ArcRender, d: &str, s: f64, group: f64) {
    if d.is_empty() {
        return;
    }
    let Some(path) = parse_path(d) else { return };
    let Some(path) = path.transform(px_transform(s)) else {
        return;
    };
    let stroke = Stroke {
        width: (arc.width * s) as f32,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    let Some(outline) = path.stroke(&stroke, 1.0) else {
        return;
    };
    let alpha = arc.opacity * group;
    let n = arc.grad.stops.len();
    let stops = arc
        .grad
        .stops
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let pos = if n <= 1 {
                0.0
            } else {
                i as f32 / (n - 1) as f32
            };
            GradientStop::new(pos, rgba(c, alpha))
        })
        .collect::<Vec<_>>();
    let Some(shader) = LinearGradient::new(
        Point::from_xy(arc.grad.x1 as f32, arc.grad.y1 as f32),
        Point::from_xy(arc.grad.x2 as f32, arc.grad.y2 as f32),
        stops,
        SpreadMode::Pad,
        px_transform(s),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.shader = shader;
    paint.anti_alias = true;
    pixmap.fill_path(
        &outline,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn draw_dot(pixmap: &mut Pixmap, d: &crate::engine::DotRender, ink: &str, s: f64) {
    let alpha = d.opacity * d.depth.unwrap_or(1.0);
    let color = d.color.as_deref().unwrap_or(ink);
    if let Some(glyph) = &d.d {
        if let Some(path) = parse_path(glyph) {
            let th = d.rot.unwrap_or(0.0).to_radians();
            let r = RAYON;
            let tf = Transform::from_row(
                (s * r * th.cos()) as f32,
                (s * r * th.sin()) as f32,
                (-s * r * th.sin()) as f32,
                (s * r * th.cos()) as f32,
                (s * (d.x + DEMI)) as f32,
                (s * (d.y + DEMI)) as f32,
            );
            if let Some(path) = path.transform(tf) {
                let mut paint = Paint::default();
                paint.set_color(rgba(color, alpha));
                paint.anti_alias = true;
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
        return;
    }
    let mut pb = PathBuilder::new();
    pb.push_circle(
        ((d.x + DEMI) * s) as f32,
        ((d.y + DEMI) * s) as f32,
        (d.r * s) as f32,
    );
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(rgba(color, alpha));
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// The mouth: a lip with round ends, filled and stroked with the same path so an
/// open mouth is a D rather than a wedge. Clipped to the body like the eyes.
fn draw_mouth(pixmap: &mut Pixmap, frame: &BotFrame, fill: &str, mask: &Mask, s: f64) {
    let Some(m) = &frame.mouth else { return };
    let x = ((m.x + DEMI) * s) as f32;
    let y = ((m.y + DEMI) * s) as f32;
    let hw = ((m.w * s) / 2.0) as f32;
    let curve = (m.curve * s) as f32;
    let open = (m.open * s) as f32;
    let mut pb = PathBuilder::new();
    pb.move_to(x - hw, y);
    pb.quad_to(x, y + 2.0 * curve, x + hw, y);
    pb.quad_to(x, y + 2.0 * (curve + open), x - hw, y);
    pb.close();
    let Some(path) = pb.finish() else { return };

    // The mouth is canted to the eye line, so that a head with its roll on reads
    // as tilted rather than as a level mouth under a crooked face. Turned about
    // its own centre, which keeps the anchor exactly where the engine put it.
    let turn = Transform::from_rotate_at(m.rot as f32, x, y);

    let alpha = m.alpha * frame.body_alpha;
    let mut paint = Paint::default();
    paint.set_color(rgba(fill, alpha));
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, FillRule::Winding, turn, Some(mask));

    // The stroke is what gives the lip its thickness and rounds the corners; the
    // fill above is the opening. At `open = 0` the path is degenerate, so this is
    // the only thing drawn.
    let stroke = Stroke {
        width: (m.thickness * s) as f32,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    if let Some(outline) = path.stroke(&stroke, 1.0) {
        pixmap.fill_path(&outline, &paint, FillRule::Winding, turn, Some(mask));
    }
}

fn draw_eyes(
    pixmap: &mut Pixmap,
    frame: &BotFrame,
    surface: &Option<Surface>,
    eye_fill: &str,
    mask: &Mask,
    s: f64,
) {
    let alpha = frame.body_alpha;
    for eye in &frame.eyes {
        let Some(m) = parse_matrix(&eye.matrix) else {
            continue;
        };
        let transform = eye_transform(m, s);
        let Some(cap) = capsule(eye.w, eye.h).transform(transform) else {
            continue;
        };
        let mut paint = Paint::default();
        paint.anti_alias = true;
        if let Some(surf) = surface {
            let hh = (eye.h / 2.0) as f32;
            let lid = eye_shade(eye_fill, surf.eye_depth);
            let stops = vec![
                GradientStop::new(0.0, rgba(&lid, eye.alpha * alpha)),
                GradientStop::new(1.0, rgba(eye_fill, eye.alpha * alpha)),
            ];
            if let Some(shader) = LinearGradient::new(
                Point::from_xy(0.0, -hh),
                Point::from_xy(0.0, hh),
                stops,
                SpreadMode::Pad,
                transform,
            ) {
                paint.shader = shader;
            }
        } else {
            paint.set_color(rgba(eye_fill, eye.alpha * alpha));
        }
        pixmap.fill_path(
            &cap,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            Some(mask),
        );
    }
}

pub fn paint_into(pixmap: &mut Pixmap, frame: &BotFrame, ink: &str, style: &str) {
    let size = pixmap.width();
    pixmap.fill(Color::TRANSPARENT);

    let s = size as f64 / (2.0 * DEMI);
    let surface = surface_for(style, ink);
    let eye_fill = eye_fill_for(ink);
    let body = body_path(&frame.body_points, s);
    let mut mask = Mask::new(size, size).expect("mask");
    mask.fill_path(&body, FillRule::Winding, true, Transform::identity());

    let orbit_opacity = surface.as_ref().map(|x| x.orbits).unwrap_or(1.0);
    for arc in &frame.arcs {
        stroke_arc(pixmap, arc, &arc.back, s, orbit_opacity);
    }
    if frame.dots_behind {
        for d in &frame.dots {
            draw_dot(pixmap, d, ink, s);
        }
    }

    let body_alpha = frame.body_alpha;
    match &surface {
        None => {
            let mut paint = Paint::default();
            paint.set_color(rgba(ink, body_alpha));
            paint.anti_alias = true;
            pixmap.fill_path(
                &body,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        Some(surf) => {
            let t = px_transform(s);
            let cx = surf.lx * RAYON;
            let cy = surf.ly * RAYON;
            let fx = (surf.lx - 0.14) * RAYON;
            let fy = (surf.ly - 0.16) * RAYON;
            let stops = vec![
                GradientStop::new(0.0, rgba(&surf.light, body_alpha)),
                GradientStop::new(surf.mid as f32, rgba(&surf.base, body_alpha)),
                GradientStop::new(1.0, rgba(&surf.deep, body_alpha)),
            ];
            if let Some(shader) = RadialGradient::new(
                Point::from_xy(fx as f32, fy as f32),
                Point::from_xy(cx as f32, cy as f32),
                (surf.reach * RAYON) as f32,
                stops,
                SpreadMode::Pad,
                t,
            ) {
                let mut paint = Paint::default();
                paint.shader = shader;
                paint.anti_alias = true;
                pixmap.fill_path(
                    &body,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }

            // Lower-edge shade wash.
            let b = body.bounds();
            let stops = vec![
                GradientStop::new(0.45, rgba("#000000", 0.0)),
                GradientStop::new(1.0, rgba("#000000", surf.shade * body_alpha)),
            ];
            if let Some(shader) = LinearGradient::new(
                Point::from_xy(b.left(), b.top()),
                Point::from_xy(b.left(), b.bottom()),
                stops,
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                let mut paint = Paint::default();
                paint.shader = shader;
                paint.anti_alias = true;
                pixmap.fill_path(
                    &body,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }

            // Specular / soft sheen.
            if surf.gloss > 0.0 {
                let gx = (surf.lx - 0.02) * RAYON;
                let gy = (surf.ly - 0.04) * RAYON;
                let rx = surf.gloss_r * RAYON;
                let ry = surf.gloss_r * 0.58 * RAYON;
                let (pcx, pcy) = ((gx + DEMI) * s, (gy + DEMI) * s);
                let th = (-32f64).to_radians();
                let tf = Transform::from_row(
                    (th.cos() * rx * s) as f32,
                    (th.sin() * rx * s) as f32,
                    (-th.sin() * ry * s) as f32,
                    (th.cos() * ry * s) as f32,
                    pcx as f32,
                    pcy as f32,
                );
                if let Some(ellipse) = circle(1.0).transform(tf) {
                    let stops = vec![
                        GradientStop::new(0.0, rgba("#ffffff", surf.gloss * body_alpha)),
                        GradientStop::new(0.6, rgba("#ffffff", 0.22 * surf.gloss * body_alpha)),
                        GradientStop::new(1.0, rgba("#ffffff", 0.0)),
                    ];
                    if let Some(shader) = RadialGradient::new(
                        Point::from_xy(pcx as f32, pcy as f32),
                        Point::from_xy(pcx as f32, pcy as f32),
                        (rx.max(ry) * s) as f32,
                        stops,
                        SpreadMode::Pad,
                        Transform::identity(),
                    ) {
                        let mut paint = Paint::default();
                        paint.shader = shader;
                        paint.anti_alias = true;
                        pixmap.fill_path(
                            &ellipse,
                            &paint,
                            FillRule::Winding,
                            Transform::identity(),
                            Some(&mask),
                        );
                    }
                }
            }
        }
    }

    draw_eyes(pixmap, frame, &surface, &eye_fill, &mask, s);
    draw_mouth(pixmap, frame, &eye_fill, &mask, s);

    if let Some(n) = &frame.notch {
        let mut pb = PathBuilder::new();
        pb.push_circle(
            ((n.x + DEMI) * s) as f32,
            ((n.y + DEMI) * s) as f32,
            (n.r * s) as f32,
        );
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(rgba(&eye_fill, body_alpha));
            paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                Some(&mask),
            );
        }
    }

    if !frame.dots_behind {
        for d in &frame.dots {
            draw_dot(pixmap, d, ink, s);
        }
    }

    if let Some(n) = &frame.notif {
        let mut pb = PathBuilder::new();
        pb.push_circle(
            ((n.x + DEMI) * s) as f32,
            ((n.y + DEMI) * s) as f32,
            (n.r * s) as f32,
        );
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(rgba(crate::engine::catalogue().notif.blue.as_str(), 1.0));
            paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    for arc in &frame.arcs {
        stroke_arc(pixmap, arc, &arc.front, s, orbit_opacity);
    }
}

pub fn paint(frame: &BotFrame, ink: &str, style: &str, size: u32) -> Pixmap {
    let mut pixmap = Pixmap::new(size, size).expect("pixmap");
    paint_into(&mut pixmap, frame, ink, style);
    pixmap
}

/// Premultiplied RGBA (tiny-skia) -> premultiplied BGRA (UpdateLayeredWindow).
pub fn to_bgra(pixmap: &Pixmap, out: &mut [u8]) {
    let data = pixmap.data();
    for i in (0..out.len()).step_by(4) {
        out[i] = data[i + 2];
        out[i + 1] = data[i + 1];
        out[i + 2] = data[i];
        out[i + 3] = data[i + 3];
    }
}

/// True if `(x, y)` in viewBox units is inside the silhouette polygon.
pub fn body_contains(points: &[VPoint], x: f64, y: f64) -> bool {
    let mut inside = false;
    let n = points.len();
    let mut j = n - 1;
    for i in 0..n {
        let pi = points[i];
        let pj = points[j];
        if ((pi.y > y) != (pj.y > y)) && (x < (pj.x - pi.x) * (y - pi.y) / (pj.y - pi.y) + pi.x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Kept for the eye glyph path in dot rendering.
#[allow(dead_code)]
fn unused(_: &RenderedEye) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<VPoint> {
        vec![
            VPoint { x: 0.0, y: 0.0 },
            VPoint { x: 10.0, y: 0.0 },
            VPoint { x: 10.0, y: 10.0 },
            VPoint { x: 0.0, y: 10.0 },
        ]
    }

    #[test]
    fn a_point_inside_the_silhouette_is_a_hit() {
        assert!(body_contains(&square(), 5.0, 5.0));
    }

    #[test]
    fn a_point_outside_is_not() {
        assert!(!body_contains(&square(), 15.0, 5.0));
        assert!(!body_contains(&square(), 5.0, -3.0));
    }

    #[test]
    fn a_concave_outline_reports_its_notch_as_outside() {
        // An L shape: the missing quadrant must read as outside, or click-through
        // would leave a dead zone where nothing is painted.
        let l = vec![
            VPoint { x: 0.0, y: 0.0 },
            VPoint { x: 10.0, y: 0.0 },
            VPoint { x: 10.0, y: 4.0 },
            VPoint { x: 4.0, y: 4.0 },
            VPoint { x: 4.0, y: 10.0 },
            VPoint { x: 0.0, y: 10.0 },
        ];
        assert!(body_contains(&l, 2.0, 2.0));
        assert!(body_contains(&l, 8.0, 2.0));
        assert!(!body_contains(&l, 8.0, 8.0));
    }
}
