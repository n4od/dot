//! Port of `src/bot/shape.ts` (the parts the engine uses).

use std::sync::OnceLock;

use super::math::{lerp, r2, TAU};

pub const PROFILE_SAMPLES: usize = 64;

pub struct CosSin {
    pub cos: [f64; PROFILE_SAMPLES],
    pub sin: [f64; PROFILE_SAMPLES],
}

static COS_SIN: OnceLock<CosSin> = OnceLock::new();

pub fn cos_sin() -> &'static CosSin {
    COS_SIN.get_or_init(|| {
        let mut cos = [0.0; PROFILE_SAMPLES];
        let mut sin = [0.0; PROFILE_SAMPLES];
        for i in 0..PROFILE_SAMPLES {
            let a = (i as f64 / PROFILE_SAMPLES as f64) * TAU;
            cos[i] = a.cos();
            sin[i] = a.sin();
        }
        CosSin { cos, sin }
    })
}

#[derive(Clone, Debug)]
pub struct Silhouette {
    pub radii: Vec<f64>,
    pub rot: f64,
    pub cx: f64,
    pub cy: f64,
    pub sx: f64,
    pub sy: f64,
}

impl Silhouette {
    pub fn circle(radius: f64) -> Self {
        Self {
            radii: vec![radius; PROFILE_SAMPLES],
            rot: 0.0,
            cx: 0.0,
            cy: 0.0,
            sx: 1.0,
            sy: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Interpolation of two silhouettes; rotation takes the shortest arc.
pub fn blend(a: &Silhouette, b: &Silhouette, t: f64) -> Silhouette {
    let radii: Vec<f64> = (0..PROFILE_SAMPLES)
        .map(|i| {
            let ar = a.radii.get(i).copied().unwrap_or(1.0);
            let br = b.radii.get(i).copied().unwrap_or(1.0);
            lerp(ar, br, t)
        })
        .collect();
    let mut drot = b.rot - a.rot;
    while drot > std::f64::consts::PI {
        drot -= TAU;
    }
    while drot < -std::f64::consts::PI {
        drot += TAU;
    }
    Silhouette {
        radii,
        rot: a.rot + drot * t,
        cx: lerp(a.cx, b.cx, t),
        cy: lerp(a.cy, b.cy, t),
        sx: lerp(a.sx, b.sx, t),
        sy: lerp(a.sy, b.sy, t),
    }
}

/// Project the silhouette to screen points.
pub fn to_points(s: &Silhouette, scale: f64) -> Vec<Point> {
    let cs = cos_sin();
    let cr = s.rot.cos();
    let sr = s.rot.sin();
    let mut out = Vec::with_capacity(PROFILE_SAMPLES);
    for i in 0..PROFILE_SAMPLES {
        let r = s.radii.get(i).copied().unwrap_or(1.0);
        let x = r * cs.cos[i];
        let y = r * cs.sin[i];
        let rx = x * cr - y * sr;
        let ry = x * sr + y * cr;
        out.push(Point {
            x: (rx * s.sx + s.cx) * scale,
            y: (ry * s.sy + s.cy) * scale,
        });
    }
    out
}

/// Closed polyline -> Catmull-Rom cubics.
pub fn closed_path(pts: &[Point], tension: f64) -> String {
    let n = pts.len();
    if n < 3 {
        return String::new();
    }
    let first = pts[0];
    let mut d = format!("M{} {}", r2(first.x), r2(first.y));
    for i in 0..n {
        let p0 = pts[(i + n - 1) % n];
        let p1 = pts[i];
        let p2 = pts[(i + 1) % n];
        let p3 = pts[(i + 2) % n];
        let c1x = p1.x + (p2.x - p0.x) * tension;
        let c1y = p1.y + (p2.y - p0.y) * tension;
        let c2x = p2.x - (p3.x - p1.x) * tension;
        let c2y = p2.y - (p3.y - p1.y) * tension;
        d.push_str(&format!(
            "C{} {} {} {} {} {}",
            r2(c1x),
            r2(c1y),
            r2(c2x),
            r2(c2y),
            r2(p2.x),
            r2(p2.y)
        ));
    }
    d.push('Z');
    d
}

/// Radius of the profile in an arbitrary direction, between two samples.
pub fn radius_at_angle(radii: &[f64], angle: f64) -> f64 {
    let n = radii.len();
    let t = ((((angle / TAU) % 1.0) + 1.0) % 1.0) * n as f64;
    let i = t.floor();
    let a = radii.get((i as usize) % n).copied().unwrap_or(1.0);
    let b = radii.get((i as usize + 1) % n).copied().unwrap_or(1.0);
    lerp(a, b, t - i)
}

/// Capsule (stadium) centred on the origin: the exact shape of the eyes.
pub fn capsule_path(w: f64, h: f64) -> String {
    let hw = w.max(0.01) / 2.0;
    let hh = h.max(0.01) / 2.0;
    let r = hw.min(hh);
    format!(
        "M{} {}A{} {} 0 0 1 {} {}L{} {}A{} {} 0 0 1 {} {}L{} {}A{} {} 0 0 1 {} {}L{} {}A{} {} 0 0 1 {} {}Z",
        r2(-hw),
        r2(-hh + r),
        r2(r),
        r2(r),
        r2(-hw + r),
        r2(-hh),
        r2(hw - r),
        r2(-hh),
        r2(r),
        r2(r),
        r2(hw),
        r2(-hh + r),
        r2(hw),
        r2(hh - r),
        r2(r),
        r2(r),
        r2(hw - r),
        r2(hh),
        r2(-hw + r),
        r2(hh),
        r2(r),
        r2(r),
        r2(-hw),
        r2(hh - r)
    )
}
