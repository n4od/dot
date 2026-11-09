//! Port of `src/bot/decor.ts` (arcs, wheel colours, particles).

use serde::Deserialize;

use super::math::{clamp, js_round, r2, TAU};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcSeed {
    pub a: f64,
    pub k: f64,
    pub tilt: f64,
    pub speed: f64,
    pub phase: f64,
    pub sweep: f64,
    pub hue: f64,
    pub hue_span: f64,
    pub width: f64,
    pub cx: f64,
    pub cy: f64,
}

/// What a state declares: geometry stays in ball-radius units.
#[derive(Clone, Debug)]
pub struct ArcSpec {
    pub id: String,
    pub seed: ArcSeed,
    pub t: f64,
    pub opacity: f64,
}

#[derive(Clone, Debug)]
pub struct Grad {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stops: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ArcRender {
    pub id: String,
    pub front: String,
    pub back: String,
    pub width: f64,
    pub opacity: f64,
    pub grad: Grad,
}

#[derive(Clone, Debug, Default)]
pub struct DotRender {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub opacity: f64,
    pub color: Option<String>,
    pub depth: Option<f64>,
    pub d: Option<String>,
    pub rot: Option<f64>,
}

/// Hue wheel at constant lightness.
pub fn wheel(hue: f64, s: f64, l: f64) -> String {
    let h = ((hue % 360.0) + 360.0) % 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let hex = |v: f64| format!("{:02x}", js_round((v + m) * 255.0) as i64);
    format!("#{}{}{}", hex(r), hex(g), hex(b))
}

/// Project an inclined 3D circle orthographically.
pub fn arc_render(seed: &ArcSeed, t: f64, scale: f64, id: &str, opacity: f64) -> ArcRender {
    let spin = seed.phase + t * seed.speed * TAU;
    let cu = seed.tilt.cos();
    let su = seed.tilt.sin();
    let kz = (1.0 - seed.k * seed.k).max(0.0).sqrt();

    const N: i32 = 64;
    let span = seed.sweep * TAU;
    let mut front = String::new();
    let mut back = String::new();
    let mut prev: Option<bool> = None;

    for i in 0..=N {
        let th = spin + (i as f64 / N as f64) * span;
        let ct = th.cos();
        let st = th.sin();
        let x = seed.a * (ct * cu + st * -su * seed.k) + seed.cx;
        let y = seed.a * (ct * su + st * cu * seed.k) + seed.cy;
        let z = seed.a * st * kz;

        let behind = z < 0.0;
        let sx = r2(x * scale);
        let sy = r2(y * scale);
        let cmd = if prev != Some(behind) { 'M' } else { 'L' };
        let seg = format!("{}{} {}", cmd, sx, sy);
        if behind {
            back.push_str(&seg);
        } else {
            front.push_str(&seg);
        }
        prev = Some(behind);
    }

    let gx = seed.tilt.cos() * seed.a * scale;
    let gy = seed.tilt.sin() * seed.a * scale;
    ArcRender {
        id: id.to_string(),
        front,
        back,
        width: seed.width * scale,
        opacity,
        grad: Grad {
            x1: r2(seed.cx * scale - gx),
            y1: r2(seed.cy * scale - gy),
            x2: r2(seed.cx * scale + gx),
            y2: r2(seed.cy * scale + gy),
            stops: vec![
                wheel(seed.hue, 0.55, 0.62),
                wheel(seed.hue + seed.hue_span * 0.5, 0.55, 0.62),
                wheel(seed.hue + seed.hue_span, 0.55, 0.62),
            ],
        },
    }
}

/// Particles spiral toward the centre while growing.
pub fn particles(t: f64, scale: f64) -> Vec<DotRender> {
    let cat = super::catalogue();
    let mut out = Vec::new();
    for p in &cat.particles {
        let u = t - p.birth;
        if !(0.0..=0.62).contains(&u) {
            continue;
        }
        let rho = p.rho * 0.75f64.powf(u * 10.0);
        let a = p.angle + (u * 100.0 * std::f64::consts::PI) / 180.0;
        out.push(DotRender {
            x: a.cos() * rho * scale,
            y: a.sin() * rho * scale,
            r: (0.04 + 0.028 * clamp(u / 0.55)) * scale,
            depth: Some(clamp(1.0 - rho / 0.8)),
            opacity: clamp(u / 0.06) * clamp((0.62 - u) / 0.08),
            ..Default::default()
        });
    }
    out
}
