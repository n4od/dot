//! Port of `src/bot/states.ts`: the catalogue of animated poses.

use serde::Deserialize;

use super::decor::{particles, ArcSpec, DotRender};
use super::face::{HeadGaze, EYE_H, EYE_SPLIT, EYE_W, REST_GAZE};
use super::math::{clamp, ease_in_out_cubic, ease_out_cubic, ease_out_quint, lerp, TAU};
use super::shape::Silhouette;
use super::{Catalogue, StateId};

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct EyeCfg {
    pub w: f64,
    pub h: f64,
    pub open: f64,
    pub tilt: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Notif {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub notch: f64,
}

/// The mouth, in the same units as the eyes: fractions of the body radius.
///
/// `curve` is how far the lip bows down (negative bows it up, for a frown) and
/// `open` is how far the mouth hangs below the lip — at `open = 0` the two curves
/// coincide and the painter draws only the lip, a rounded line. Every field
/// interpolates, so a mouth can part without swapping shapes.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MouthCfg {
    pub w: f64,
    pub curve: f64,
    pub thickness: f64,
    pub open: f64,
    pub alpha: f64,
}

impl Default for MouthCfg {
    fn default() -> Self {
        Self::REST
    }
}

impl MouthCfg {
    /// The resting mouth: the parted grin from the design gate. It is the face
    /// the pet wears when nothing is happening, and the shape every expression
    /// and emote moves away from.
    pub const REST: MouthCfg = MouthCfg {
        w: 0.358,
        curve: 0.069,
        thickness: 0.041,
        open: 0.062,
        alpha: 1.0,
    };

    /// The same shape at another opacity — a state with no face at all, or one
    /// whose face fades in with its eyes.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }
}

/// A mouth of a given width, bow, opening and lip, fully opaque.
fn mouth(w: f64, curve: f64, open: f64, thickness: f64) -> MouthCfg {
    MouthCfg {
        w,
        curve,
        thickness,
        open,
        alpha: 1.0,
    }
}

#[derive(Clone, Debug)]
pub struct Pose {
    pub sil: Silhouette,
    pub off_x: f64,
    pub off_y: f64,
    pub gaze: HeadGaze,
    pub split: f64,
    pub eyes: [EyeCfg; 2],
    pub mouth: MouthCfg,
    pub eye_alpha: f64,
    pub body_alpha: f64,
    pub dots: Vec<DotRender>,
    pub arcs: Vec<ArcSpec>,
    pub notif: Option<Notif>,
    pub dots_behind: bool,
}

fn eye(w: f64, h: f64) -> EyeCfg {
    EyeCfg {
        w,
        h,
        open: 1.0,
        tilt: 0.0,
    }
}

fn pair(w: f64, h: f64) -> [EyeCfg; 2] {
    [eye(w, h), eye(w, h)]
}

pub fn lerp_eye_cfg(a: &EyeCfg, b: &EyeCfg, t: f64) -> EyeCfg {
    EyeCfg {
        w: super::math::lerp(a.w, b.w, t),
        h: super::math::lerp(a.h, b.h, t),
        open: super::math::lerp(a.open, b.open, t),
        tilt: super::math::lerp(a.tilt, b.tilt, t),
    }
}

pub fn lerp_mouth_cfg(a: &MouthCfg, b: &MouthCfg, t: f64) -> MouthCfg {
    MouthCfg {
        w: super::math::lerp(a.w, b.w, t),
        curve: super::math::lerp(a.curve, b.curve, t),
        thickness: super::math::lerp(a.thickness, b.thickness, t),
        open: super::math::lerp(a.open, b.open, t),
        alpha: super::math::lerp(a.alpha, b.alpha, t),
    }
}

pub fn base() -> Pose {
    Pose {
        sil: Silhouette::circle(1.0),
        off_x: 0.0,
        off_y: 0.0,
        gaze: REST_GAZE,
        split: EYE_SPLIT,
        eyes: pair(EYE_W, EYE_H),
        mouth: MouthCfg::REST,
        eye_alpha: 1.0,
        body_alpha: 1.0,
        dots: Vec::new(),
        arcs: Vec::new(),
        notif: None,
        dots_behind: false,
    }
}

fn silhouette(radii: &[f64], rot: f64, cx: f64, cy: f64) -> Silhouette {
    Silhouette {
        radii: radii.to_vec(),
        rot,
        cx,
        cy,
        sx: 1.0,
        sy: 1.0,
    }
}

fn circle(radius: f64, cx: f64, cy: f64) -> Silhouette {
    Silhouette {
        radii: vec![radius; super::shape::PROFILE_SAMPLES],
        rot: 0.0,
        cx,
        cy,
        sx: 1.0,
        sy: 1.0,
    }
}

/// Pulse wave travelling across the three dots, left to right.
fn dot_pulse(t: f64, index: f64) -> f64 {
    let p = ((((t - index * 0.5) / 1.5) % 1.0) + 1.0) % 1.0;
    let k = if p < 0.5 {
        0.5 - 0.5 * (p * TAU).cos()
    } else {
        0.0
    };
    clamp(k * 2.0)
}

fn spinning_triangle(rot: f64, cat: &Catalogue) -> Silhouette {
    silhouette(
        &cat.profiles.triangle,
        rot,
        -cat.tri_orbit * rot.sin(),
        cat.tri_orbit * rot.cos(),
    )
}

fn bar_italic(rot: f64, cx: f64, cy: f64, cat: &Catalogue) -> Silhouette {
    silhouette(&cat.bars.italic, rot, cx, cy)
}

fn bar_upright(cat: &Catalogue) -> Silhouette {
    silhouette(&cat.bars.upright, 0.0, 0.0, cat.bars.upright_cy)
}

/// The pose of `id` at local time `t`.
pub fn pose(id: StateId, t: f64, cat: &Catalogue) -> Pose {
    match id {
        StateId::Idle => base(),

        StateId::Thinking => {
            let mid = dot_pulse(t, 1.0);
            let emerge = 0.3 + 0.7 * ease_out_cubic(clamp(t / 0.3));
            let mut p = base();
            p.sil = circle(
                cat.dot.r * (1.0 + (cat.dot.peak - 1.0) * mid),
                cat.dot.x[1],
                0.0,
            );
            p.eye_alpha = 0.0;
            // no face at all while thinking: the body *is* a dot
            p.mouth = MouthCfg::REST.with_alpha(0.0);
            p.dots = [0.0_f64, 2.0]
                .iter()
                .map(|&i| {
                    let k = dot_pulse(t, i);
                    DotRender {
                        x: cat.dot.x[i as usize] * emerge,
                        y: 0.0,
                        r: cat.dot.r * (1.0 + (cat.dot.peak - 1.0) * k),
                        opacity: 0.55 + 0.45 * k,
                        ..Default::default()
                    }
                })
                .collect();
            p
        }

        StateId::Wink => {
            let mut p = base();
            p.gaze = HeadGaze {
                yaw: -5.37,
                pitch: 4.55,
                roll: 6.7,
            };
            p.split = 16.25;
            p.eyes = [eye(0.236, 0.464), eye(0.447, 0.089)];
            p
        }

        StateId::Wide => {
            let mut p = base();
            p.gaze = HeadGaze {
                yaw: 6.92,
                pitch: -21.96,
                roll: 11.6,
            };
            p.split = 18.43;
            p.eyes = pair(0.356, 0.875);
            // startled: the mouth stays round while the eyes go wide
            p.mouth = mouth(0.165, 0.012, 0.18, 0.042);
            p
        }

        StateId::Alert => {
            let p0 = clamp(t / 1.5);
            let travel = ease_in_out_cubic(p0) * 0.82 - 0.087;
            let back = if t > 1.6 { clamp((t - 1.6) / 0.4) } else { 0.0 };
            let x = travel * (1.0 - back) + 0.1 * back;
            let buzz = (t * 2.5 * TAU).sin() * 0.005;
            let tilt = (17.7 * std::f64::consts::PI) / 180.0;
            let mut p = base();
            p.sil = bar_italic(tilt, x, -0.325 - buzz, cat);
            p.eye_alpha = 0.0;
            p.mouth = MouthCfg::REST.with_alpha(0.0);
            p.dots = vec![DotRender {
                x: x - tilt.sin() * 0.58,
                y: -0.325 + tilt.cos() * 0.58 + buzz * 2.8,
                r: 0.118,
                d: Some(cat.bars.tear.clone()),
                rot: Some((tilt * 180.0) / std::f64::consts::PI),
                opacity: 1.0,
                ..Default::default()
            }];
            p
        }

        StateId::Notify => {
            let p0 = clamp(t / 0.45);
            let pop =
                1.0 + (cat.notif.pop - 1.0) * (p0 * std::f64::consts::PI).sin() * (1.0 - p0 * 0.35);
            let r = cat.notif.r * if p0 < 1.0 { pop } else { 1.0 };
            let a = (cat.notif.angle * std::f64::consts::PI) / 180.0;
            let mut p = base();
            p.gaze = HeadGaze {
                yaw: -21.94,
                pitch: -5.82,
                roll: -12.2,
            };
            p.split = 18.89;
            p.eyes = pair(0.505, 0.498);
            p.notif = Some(Notif {
                x: a.cos() * cat.notif.dist,
                y: a.sin() * cat.notif.dist,
                r,
                notch: r + cat.notif.margin,
            });
            p
        }

        StateId::Exclaim => {
            let mut p = base();
            p.sil = bar_upright(cat);
            p.eye_alpha = 0.0;
            p.mouth = MouthCfg::REST.with_alpha(0.0);
            p.dots = vec![DotRender {
                x: -0.012,
                y: 0.526,
                r: 0.113,
                opacity: 1.0,
                ..Default::default()
            }];
            p
        }

        StateId::Sleep => {
            let mut p = base();
            p.sil = circle(0.1585, 0.0, 0.11 + (t * (TAU / 0.6)).sin() * 0.19);
            p.eye_alpha = 0.0;
            p.mouth = MouthCfg::REST.with_alpha(0.0);
            p
        }

        StateId::Yawn => {
            // One picture, not a sequence: everything interpolates towards a single
            // peak and back, so the two engines only have to agree on the curve.
            //
            // It begins the instant the pet wakes (the morph is short, so there is no
            // dead beat), but the mouth opens *slowly* — `ease_in_out_cubic` over
            // 1.2 s — so the jaw drops gradually instead of the mouth appearing
            // already open.
            let rise = ease_in_out_cubic(clamp(t / 1.2));
            let fall = ease_in_out_cubic(clamp((t - 2.05) / 1.2));
            let gaping = rise * (1.0 - fall);
            // The eyes are already squeezed shut when it wakes — it has only just
            // opened them — and stay that way through the yawn, opening only once the
            // mouth has closed again. Starting them wide open reads as a face, not as
            // a yawn.
            let squint = 1.0 - ease_in_out_cubic(clamp((t - 1.95) / 1.05));
            // The shiver that makes it a yawn and not just an open mouth: a 1 %
            // wobble that exists only while the mouth is at its widest.
            let held = clamp((t - 1.05) / 0.3) * clamp((2.45 - t) / 0.3);
            let shiver = 1.0 + 0.012 * (t * TAU * 4.0).sin() * held;
            let mut p = base();
            p.gaze = HeadGaze {
                yaw: lerp(REST_GAZE.yaw, 8.0, gaping),
                pitch: lerp(REST_GAZE.pitch, 16.0, gaping),
                roll: lerp(REST_GAZE.roll, -5.0, gaping),
            };
            p.split = lerp(EYE_SPLIT, 18.0, squint);
            p.eyes = pair(lerp(EYE_W, 0.3, squint), lerp(EYE_H, 0.075, squint));
            p.mouth = MouthCfg {
                w: lerp(MouthCfg::REST.w, 0.62, gaping),
                curve: lerp(MouthCfg::REST.curve, -0.18, gaping),
                thickness: lerp(MouthCfg::REST.thickness, 0.042, gaping),
                open: lerp(MouthCfg::REST.open, 0.62, gaping) * shiver,
                alpha: 1.0,
            };
            p.sil = circle(1.0, 0.0, -0.025 * gaping);
            p.sil.sx = 1.0 - 0.045 * gaping;
            p.sil.sy = 1.0 + 0.075 * gaping;
            p
        }

        StateId::Egg => {
            let mut p = base();
            p.sil = silhouette(&cat.profiles.egg, 0.0, 0.0, 0.0);
            p.gaze = HeadGaze {
                yaw: 19.97,
                pitch: 26.01,
                roll: -17.1,
            };
            p.split = 11.07;
            p.eyes = pair(0.164, 0.385);
            p
        }

        StateId::Hexagon => {
            let mut p = base();
            p.sil = silhouette(&cat.profiles.hexagon, 0.0, 0.0, 0.0);
            p.gaze = HeadGaze {
                yaw: 23.11,
                pitch: 24.42,
                roll: -13.3,
            };
            p.split = 13.37;
            p.eyes = pair(0.177, 0.411);
            p
        }

        StateId::Play => {
            let fade = clamp(t / 0.35) * clamp((2.2 - t) / 0.5);
            let mut p = base();
            p.sil = spinning_triangle(0.0, cat);
            p.gaze = HeadGaze {
                yaw: 12.0,
                pitch: -8.0,
                roll: -6.0,
            };
            p.split = 15.0;
            p.eyes = pair(0.18, 0.34);
            p.arcs = cat
                .swoosh
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let mut seed = s.clone();
                    seed.cx = 0.45 - t * 0.42;
                    ArcSpec {
                        id: format!("sw{}", i),
                        seed,
                        t,
                        opacity: fade,
                    }
                })
                .collect();
            p
        }

        StateId::Orbit => {
            let ramp = ease_in_out_cubic(clamp(t / 0.35));
            let rot = -TAU * 1.25 * t * ramp;
            let back = ease_in_out_cubic(clamp((t - 1.6) / 0.9));
            let tri = spinning_triangle(rot, cat);
            let radii: Vec<f64> = tri.radii.iter().map(|r| r + (1.0 - r) * back).collect();
            let fade = clamp(t / 0.8) * clamp((3.6 - t) / 0.9);
            let mut p = base();
            p.sil = Silhouette {
                radii,
                rot,
                cx: tri.cx * (1.0 - back),
                cy: tri.cy * (1.0 - back),
                sx: 1.0,
                sy: 1.0,
            };
            p.gaze = HeadGaze {
                yaw: REST_GAZE.yaw + (t * 6.5).sin() * 65.0 * (1.0 - back),
                pitch: -4.0 + back * 32.0,
                roll: -13.0,
            };
            p.eyes = pair(0.18, 0.34 + back * 0.07);
            p.arcs = cat
                .rings
                .iter()
                .enumerate()
                .map(|(i, s)| ArcSpec {
                    id: format!("rg{}", i),
                    seed: s.clone(),
                    t,
                    opacity: fade * clamp((t - i as f64 * 0.13) / 0.3),
                })
                .collect();
            p
        }

        StateId::Swirl => {
            let mut p = base();
            p.arcs = cat
                .rings
                .iter()
                .take(3)
                .enumerate()
                .map(|(i, s)| ArcSpec {
                    id: format!("sw{}", i),
                    seed: s.clone(),
                    t,
                    opacity: clamp((t - i as f64 * 0.06) / 0.14) * clamp((1.22 - t) / 0.34),
                })
                .collect();
            p
        }

        StateId::Burst => {
            let collapse = 1.0 - 0.834 * ease_out_quint(clamp(t / 0.7));
            let regrow = ease_out_quint(clamp((t - 1.7) / 0.7));
            let mut p = base();
            p.sil = circle(collapse + (1.0 - collapse) * regrow, 0.0, 0.0);
            p.eye_alpha = clamp((t - 1.85) / 0.4);
            // the face comes back in one piece: the mouth lights up with the eyes
            p.mouth = MouthCfg::REST.with_alpha(p.eye_alpha);
            p.dots = particles(t, 1.0);
            p.dots_behind = true;
            p
        }

        StateId::Comet => {
            let collapse = 1.0 - (1.0 - cat.comet_dot) * ease_out_quint(clamp(t / 0.55));
            let regrow = ease_out_quint(clamp((t - 1.85) / 0.6));
            let fade = clamp((t - 0.15) / 0.25) * clamp((1.95 - t) / 0.3);
            let mut p = base();
            p.sil = circle(
                collapse + (1.0 - collapse) * regrow,
                0.0,
                (clamp(t / 1.7) * std::f64::consts::PI).sin() * 0.035,
            );
            p.eye_alpha = clamp((t - 2.0) / 0.35);
            p.mouth = MouthCfg::REST.with_alpha(p.eye_alpha);
            p.arcs = cat
                .comet
                .iter()
                .enumerate()
                .map(|(i, s)| ArcSpec {
                    id: format!("cm{}", i),
                    seed: s.clone(),
                    t,
                    opacity: fade,
                })
                .collect();
            p
        }
    }
}
