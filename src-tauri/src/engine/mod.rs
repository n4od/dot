//! Clock-free Rust port of the TypeScript animation engine in `src/bot/`.
//!
//! `sample(now)` is a pure function of time; replaying a golden script must
//! reproduce the TypeScript frame within tolerance. The numeric tables come from
//! `assets/engine-catalogue.json`, loaded once at compile time.
#![allow(dead_code)]

mod decor;
mod expressions;
mod face;
mod math;
mod shape;
mod states;

pub use shape::Point;

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use decor::{arc_render, ArcSpec};
pub use decor::{ArcRender, DotRender};
use expressions::{blend_expression, Expression};
use face::{blink_scale, eye_poses, liveliness, HeadGaze};
use math::{clamp, ease_out_quint, lerp, r2};
use shape::{blend, capsule_path, closed_path, radius_at_angle, to_points, Silhouette};
use states::{lerp_eye_cfg, lerp_mouth_cfg, pose, Pose};

macro_rules! id_enum {
    ($name:ident { $($variant:ident => $id:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub fn from_id(id: &str) -> Option<Self> {
                match id {
                    $($id => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub fn as_id(self) -> &'static str {
                match self {
                    $(Self::$variant => $id),+
                }
            }
        }
    };
}

id_enum!(StateId {
    Idle => "idle",
    Thinking => "thinking",
    Wink => "wink",
    Wide => "wide",
    Alert => "alert",
    Notify => "notify",
    Exclaim => "exclaim",
    Sleep => "sleep",
    Yawn => "yawn",
    Egg => "egg",
    Hexagon => "hexagon",
    Play => "play",
    Orbit => "orbit",
    Swirl => "swirl",
    Burst => "burst",
    Comet => "comet",
});

id_enum!(ShapeId {
    Cercle => "cercle",
    Galet => "galet",
    Squircle => "squircle",
    Capsule => "capsule",
    Triangle => "triangle",
    Hexagone => "hexagone",
    Nuage => "nuage",
    Goutte => "goutte",
});

id_enum!(ExprId {
    Neutre => "neutre",
    Attentif => "attentif",
    Surpris => "surpris",
    Excite => "excite",
    Heureux => "heureux",
    Hilare => "hilare",
    Colere => "colere",
    Triste => "triste",
    Effraye => "effraye",
    Mefiant => "mefiant",
    Confus => "confus",
    Curieux => "curieux",
    Fier => "fier",
    Timide => "timide",
    Blase => "blase",
    Somnolent => "somnolent",
});

/* --------------------------------------------------------------- catalogue */

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct Profiles {
    pub egg: Vec<f64>,
    pub hexagon: Vec<f64>,
    pub triangle: Vec<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ShapeDef {
    pub id: String,
    pub radii: Vec<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ColorDef {
    pub id: String,
    pub hex: String,
}

#[derive(Debug, Deserialize)]
pub struct ExprDef {
    pub id: String,
    pub gaze: HeadGaze,
    pub split: f64,
    pub eyes: [states::EyeCfg; 2],
    /// Absent from a catalogue written before mouths existed: that pet keeps the
    /// resting mouth rather than failing to load.
    #[serde(default)]
    pub mouth: states::MouthCfg,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateMeta {
    pub id: String,
    pub duration: f64,
    pub min_duration: Option<f64>,
    pub morph: f64,
    pub blink_in: bool,
    pub base_body: bool,
    pub base_face: bool,
}

#[derive(Debug, Deserialize)]
pub struct Particle {
    pub birth: f64,
    pub angle: f64,
    pub rho: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bars {
    pub upright_cy: f64,
    pub upright: Vec<f64>,
    pub italic: Vec<f64>,
    pub tear: String,
}

#[derive(Debug, Deserialize)]
pub struct Dot {
    pub x: [f64; 3],
    pub r: f64,
    pub peak: f64,
}

#[derive(Debug, Deserialize)]
pub struct NotifDef {
    pub angle: f64,
    pub dist: f64,
    pub r: f64,
    pub pop: f64,
    pub margin: f64,
    pub blue: String,
}

/// One tint of a lit style: `to` mixed into the ink by `t`.
#[derive(Debug, Deserialize)]
pub struct ToneRule {
    pub to: String,
    pub t: f64,
}

/// The numbers of a lit style, mirrored from `src/pet/materials.ts`. Colour
/// fields are tints, so both painters derive the tones from the same ink.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRule {
    pub light: ToneRule,
    pub base: Option<ToneRule>,
    pub deep: ToneRule,
    pub lx: f64,
    pub ly: f64,
    pub reach: f64,
    pub mid: f64,
    pub shade: f64,
    pub gloss: f64,
    pub gloss_r: f64,
    pub eye_depth: f64,
    pub orbits: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    pub scale: f64,
    pub profiles: Profiles,
    pub shapes: Vec<ShapeDef>,
    pub colors: Vec<ColorDef>,
    pub expressions: Vec<ExprDef>,
    pub states: Vec<StateMeta>,
    pub rings: Vec<ArcSeed>,
    pub swoosh: Vec<ArcSeed>,
    pub comet: Vec<ArcSeed>,
    pub particles: Vec<Particle>,
    pub blinks: Vec<f64>,
    pub bars: Bars,
    pub tri_orbit: f64,
    pub dot: Dot,
    pub notif: NotifDef,
    pub comet_dot: f64,
    pub decalages: HashMap<String, HashMap<String, Offset>>,
    pub surfaces: HashMap<String, SurfaceRule>,
}

use decor::ArcSeed;

static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();

pub fn catalogue() -> &'static Catalogue {
    CATALOGUE.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/engine-catalogue.json"))
            .expect("engine catalogue must parse")
    })
}

impl Catalogue {
    pub fn shape_radii(&self, id: ShapeId) -> &[f64] {
        self.shapes
            .iter()
            .find(|s| s.id == id.as_id())
            .map(|s| s.radii.as_slice())
            .expect("shape id must exist in the catalogue")
    }

    pub fn expr_def(&self, id: ExprId) -> &ExprDef {
        self.expressions
            .iter()
            .find(|e| e.id == id.as_id())
            .expect("expression id must exist in the catalogue")
    }

    pub fn state_meta(&self, id: StateId) -> &StateMeta {
        self.states
            .iter()
            .find(|s| s.id == id.as_id())
            .expect("state id must exist in the catalogue")
    }

    /// Precomputed eye offset for a shape, state and expression.
    pub fn decalage(&self, shape: ShapeId, state: StateId, expr: Option<ExprId>) -> Offset {
        let zero = Offset { x: 0.0, y: 0.0 };
        let Some(par) = self.decalages.get(shape.as_id()) else {
            return zero;
        };
        let with_expr = format!(
            "{}|{}",
            state.as_id(),
            expr.map(|e| e.as_id()).unwrap_or("")
        );
        par.get(&with_expr)
            .or_else(|| par.get(&format!("{}|", state.as_id())))
            .copied()
            .unwrap_or(zero)
    }
}

/* -------------------------------------------------------------- frame types */

#[derive(Clone, Copy, Debug)]
pub struct EyePose {
    pub x: f64,
    pub y: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub depth: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenderedEye {
    pub d: String,
    pub matrix: String,
    pub alpha: f64,
    /// Capsule size in viewBox units, so a native renderer can build the shape
    /// without parsing the `A` commands of `d`.
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct NotifFrame {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouthFrame {
    /// Centre of the lip line, in viewBox units.
    pub x: f64,
    pub y: f64,
    /// How far the mouth is canted, in degrees, from the eye line. A rolled head
    /// tilts its eyes, so the mouth tilts with them.
    pub rot: f64,
    /// Lip width, bow, thickness and opening, all in viewBox units.
    pub w: f64,
    pub curve: f64,
    pub thickness: f64,
    pub open: f64,
    pub alpha: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BotFrame {
    pub body_path: String,
    /// Geometry for the native hit test; not part of the serialized frame.
    #[serde(skip)]
    pub body_points: Vec<Point>,
    pub body_alpha: f64,
    pub eyes: Vec<RenderedEye>,
    pub mouth: Option<MouthFrame>,
    pub dots: Vec<DotRender>,
    pub dots_behind: bool,
    pub arcs: Vec<ArcRender>,
    pub notif: Option<NotifFrame>,
    pub notch: Option<NotifFrame>,
}

/// Where the bot looks when something external drives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub yaw: f64,
    pub pitch: f64,
    pub mix: f64,
    pub spin: f64,
    pub wander: f64,
}

const NO_LOOK: Look = Look {
    yaw: 0.0,
    pitch: 0.0,
    mix: 0.0,
    spin: 0.0,
    wander: 1.0,
};

fn lerp_look(a: &Look, b: &Look, t: f64) -> Look {
    Look {
        yaw: lerp(a.yaw, b.yaw, t),
        pitch: lerp(a.pitch, b.pitch, t),
        mix: lerp(a.mix, b.mix, t),
        spin: lerp(a.spin, b.spin, t),
        wander: lerp(a.wander, b.wander, t),
    }
}

/// Interpolate two poses. Decor crosses by opacity, not geometry.
fn blend_pose(a: &Pose, b: &Pose, t: f64) -> Pose {
    let out = 1.0 - t;
    let mut dots: Vec<DotRender> = a
        .dots
        .iter()
        .map(|d| DotRender {
            opacity: d.opacity * out,
            ..d.clone()
        })
        .collect();
    dots.extend(b.dots.iter().map(|d| DotRender {
        opacity: d.opacity * t,
        ..d.clone()
    }));

    let mut arcs: Vec<ArcSpec> = a
        .arcs
        .iter()
        .map(|r| ArcSpec {
            id: format!("a{}", r.id),
            seed: r.seed.clone(),
            t: r.t,
            opacity: r.opacity * out,
        })
        .collect();
    arcs.extend(b.arcs.iter().map(|r| ArcSpec {
        id: format!("b{}", r.id),
        seed: r.seed.clone(),
        t: r.t,
        opacity: r.opacity * t,
    }));

    Pose {
        sil: blend(&a.sil, &b.sil, t),
        off_x: lerp(a.off_x, b.off_x, t),
        off_y: lerp(a.off_y, b.off_y, t),
        gaze: HeadGaze {
            yaw: lerp(a.gaze.yaw, b.gaze.yaw, t),
            pitch: lerp(a.gaze.pitch, b.gaze.pitch, t),
            roll: lerp(a.gaze.roll, b.gaze.roll, t),
        },
        split: lerp(a.split, b.split, t),
        eyes: [
            lerp_eye_cfg(&a.eyes[0], &b.eyes[0], t),
            lerp_eye_cfg(&a.eyes[1], &b.eyes[1], t),
        ],
        mouth: lerp_mouth_cfg(&a.mouth, &b.mouth, t),
        eye_alpha: lerp(a.eye_alpha, b.eye_alpha, t),
        body_alpha: lerp(a.body_alpha, b.body_alpha, t),
        dots,
        arcs,
        notif: if t < 0.5 { a.notif } else { b.notif },
        dots_behind: if t < 0.5 {
            a.dots_behind
        } else {
            b.dots_behind
        },
    }
}

/* ------------------------------------------------------------------ engine */

pub struct BotEngine {
    pub scale: f64,
    cur: StateId,
    prev: Option<StateId>,
    depart_fige: Option<Pose>,
    t_cur: f64,
    t_prev: f64,
    blink_at: f64,
    shape: Option<ShapeId>,
    shape_prev: Option<ShapeId>,
    shape_at: f64,
    expr: Option<ExprId>,
    expr_prev: Option<ExprId>,
    expr_at: f64,
    look: Look,
    look_prev: Look,
    look_at: f64,
    look_morph: f64,
    /// Whether the face has a mouth at all. `false` makes every frame's
    /// `mouth: None`, which is the same path the faceless states already take.
    mouth: bool,
}

impl BotEngine {
    pub const SHAPE_MORPH: f64 = 0.45;
    pub const LOOK_MORPH: f64 = 0.24;

    pub fn new(scale: f64, initial: StateId, shape: Option<ShapeId>, expr: Option<ExprId>) -> Self {
        Self {
            scale,
            cur: initial,
            prev: None,
            depart_fige: None,
            t_cur: 0.0,
            t_prev: 0.0,
            blink_at: -10.0,
            shape,
            shape_prev: None,
            shape_at: -10.0,
            expr,
            expr_prev: None,
            expr_at: -10.0,
            look: NO_LOOK,
            look_prev: NO_LOOK,
            look_at: -10.0,
            look_morph: Self::LOOK_MORPH,
            mouth: true,
        }
    }

    /// Whether the pet is drawn with a mouth. It is appearance, so it lives here
    /// with the pose rather than in a painter: every state, the settings stage and
    /// the expression thumbnails then agree by construction.
    pub fn set_mouth(&mut self, on: bool) {
        self.mouth = on;
    }

    pub fn state(&self) -> StateId {
        self.cur
    }

    /// Rewind to `id` with no previous state, like a fresh engine on that state.
    pub fn reset(&mut self, id: StateId, now: f64) {
        self.cur = id;
        self.prev = None;
        self.depart_fige = None;
        self.t_cur = now;
        self.t_prev = now;
        self.blink_at = -10.0;
    }

    pub fn set_state(&mut self, id: StateId, now: f64) {
        if id == self.cur {
            return;
        }
        let cat = catalogue();
        let morph = cat.state_meta(self.cur).morph;
        let en_plein_fondu = self.prev.is_some() && now - self.t_cur < morph;
        self.depart_fige = if en_plein_fondu {
            Some(self.pose_composee(now))
        } else {
            None
        };
        self.prev = Some(self.cur);
        self.t_prev = self.t_cur;
        self.cur = id;
        self.t_cur = now;
        if cat.state_meta(id).blink_in {
            self.blink_at = now;
        }
    }

    pub fn set_shape(&mut self, shape: Option<ShapeId>, now: f64) {
        if shape == self.shape {
            return;
        }
        self.shape_prev = self.shape;
        self.shape = shape;
        self.shape_at = now;
    }

    pub fn set_expression(&mut self, expr: Option<ExprId>, now: f64) {
        if expr == self.expr {
            return;
        }
        self.expr_prev = self.expr;
        self.expr = expr;
        self.expr_at = now;
    }

    pub fn set_look(&mut self, look: Option<Look>, now: f64, morph: f64) {
        if let Some(l) = look {
            let sum = l.yaw + l.pitch + l.mix + l.spin + l.wander;
            if !sum.is_finite() {
                return;
            }
        }
        self.look_prev = self.look_at_time(now);
        self.look = look.unwrap_or(NO_LOOK);
        self.look_at = now;
        self.look_morph = morph;
    }

    fn shape_at_time(&self, now: f64) -> Option<Vec<f64>> {
        let cat = catalogue();
        match (self.shape, self.shape_prev) {
            (Some(to), Some(from)) => {
                let radii_to = cat.shape_radii(to);
                let k = (now - self.shape_at) / Self::SHAPE_MORPH;
                if k >= 1.0 {
                    return Some(radii_to.to_vec());
                }
                let t = ease_out_quint(clamp(k));
                let radii_from = cat.shape_radii(from);
                Some(
                    radii_to
                        .iter()
                        .enumerate()
                        .map(|(i, r)| lerp(radii_from.get(i).copied().unwrap_or(*r), *r, t))
                        .collect(),
                )
            }
            _ => self.shape.map(|s| cat.shape_radii(s).to_vec()),
        }
    }

    fn expr_at_time(&self, now: f64) -> Option<Expression> {
        let cat = catalogue();
        match (self.expr, self.expr_prev) {
            (Some(to), Some(from)) => {
                let to_e = Expression::from_def(cat.expr_def(to));
                let k = (now - self.expr_at) / Self::SHAPE_MORPH;
                if k >= 1.0 {
                    return Some(to_e);
                }
                let from_e = Expression::from_def(cat.expr_def(from));
                Some(blend_expression(&from_e, &to_e, ease_out_quint(clamp(k))))
            }
            _ => self.expr.map(|e| Expression::from_def(cat.expr_def(e))),
        }
    }

    fn look_at_time(&self, now: f64) -> Look {
        let k = (now - self.look_at) / self.look_morph;
        if k >= 1.0 {
            return self.look;
        }
        lerp_look(&self.look_prev, &self.look, ease_out_quint(clamp(k)))
    }

    fn posed(&self, id: StateId, t: f64, shape: Option<&[f64]>, expr: Option<&Expression>) -> Pose {
        let cat = catalogue();
        let meta = cat.state_meta(id);
        let mut p = pose(id, t, cat);
        if meta.base_body {
            if let Some(radii) = shape {
                p.sil.radii = radii.to_vec();
            }
        }
        if meta.base_face {
            if let Some(e) = expr {
                p.gaze = e.gaze;
                p.split = e.split;
                p.eyes = e.eyes;
                p.mouth = e.mouth;
            }
        }
        p
    }

    fn origine(&self, now: f64, shape: Option<&[f64]>, expr: Option<&Expression>) -> Option<Pose> {
        if let Some(p) = &self.depart_fige {
            return Some(p.clone());
        }
        let prev = self.prev?;
        Some(self.posed(prev, (now - self.t_prev).max(0.0), shape, expr))
    }

    fn pose_composee(&self, now: f64) -> Pose {
        let cat = catalogue();
        let id = self.cur;
        let shape = self.shape_at_time(now);
        let expr = self.expr_at_time(now);
        let p = self.posed(
            id,
            (now - self.t_cur).max(0.0),
            shape.as_deref(),
            expr.as_ref(),
        );
        let since = now - self.t_cur;
        let morph = cat.state_meta(id).morph;
        if since >= morph {
            return p;
        }
        match self.origine(now, shape.as_deref(), expr.as_ref()) {
            None => p,
            Some(origine) => blend_pose(&origine, &p, ease_out_quint(clamp(since / morph))),
        }
    }

    fn decalage_at_time(&self, now: f64, state: StateId) -> Offset {
        let cat = catalogue();
        let sur_axe = |debut: f64, duree: f64, a: Offset, b: Offset| -> Offset {
            if a == b {
                return b;
            }
            let k = (now - debut) / duree;
            if k >= 1.0 {
                return b;
            }
            let t = ease_out_quint(clamp(k));
            Offset {
                x: lerp(a.x, b.x, t),
                y: lerp(a.y, b.y, t),
            }
        };

        let par_forme = |shape: Option<ShapeId>| -> Offset {
            let dec = |e: Option<ExprId>| -> Offset {
                match shape {
                    Some(s) => cat.decalage(s, state, e),
                    None => Offset { x: 0.0, y: 0.0 },
                }
            };
            sur_axe(
                self.expr_at,
                Self::SHAPE_MORPH,
                dec(self.expr_prev),
                dec(self.expr),
            )
        };

        sur_axe(
            self.shape_at,
            Self::SHAPE_MORPH,
            par_forme(self.shape_prev),
            par_forme(self.shape),
        )
    }

    pub fn sample(&self, now: f64) -> BotFrame {
        let r = self.scale;
        let cat = catalogue();
        let id = self.cur;
        let shape = self.shape_at_time(now);
        let expr = self.expr_at_time(now);
        let mut p = self.posed(
            id,
            (now - self.t_cur).max(0.0),
            shape.as_deref(),
            expr.as_ref(),
        );
        let mut decalage = self.decalage_at_time(now, id);

        // --- transition ---------------------------------------------------
        let since = now - self.t_cur;
        let morph = cat.state_meta(id).morph;
        let origine = if since < morph {
            self.origine(now, shape.as_deref(), expr.as_ref())
        } else {
            None
        };
        if let Some(origine) = origine {
            let ratio = ease_out_quint(clamp(since / morph));
            p = blend_pose(&origine, &p, ratio);
            if let Some(quitte) = self.prev {
                let avant = self.decalage_at_time(now, quitte);
                decalage = Offset {
                    x: lerp(avant.x, decalage.x, ratio),
                    y: lerp(avant.y, decalage.y, ratio),
                };
            }
        }

        // --- life at rest -------------------------------------------------
        let alive = p.eye_alpha > 0.01;
        let look = self.look_at_time(now);
        let life = liveliness(now, if alive { look.wander } else { 0.0 }, alive, true);

        let gaze = HeadGaze {
            yaw: lerp(p.gaze.yaw, look.yaw, look.mix) + life.d_yaw - look.spin,
            pitch: lerp(p.gaze.pitch, look.pitch, look.mix) + life.d_pitch,
            roll: p.gaze.roll + life.d_roll,
        };

        let forced = clamp((now - self.blink_at) / 0.2);
        let forced_lid = if forced < 1.0 {
            (forced * 2.0 - 1.0).abs()
        } else {
            1.0
        };
        let lid = life.lid.min(forced_lid);

        let off_x = p.off_x + life.drift_x;
        let off_y = p.off_y + life.drift_y;

        // --- body ---------------------------------------------------------
        let sil = Silhouette {
            radii: p.sil.radii.clone(),
            rot: p.sil.rot,
            cx: p.sil.cx + off_x,
            cy: p.sil.cy + off_y,
            sx: p.sil.sx,
            sy: p.sil.sy * life.breath,
        };
        let body_points = to_points(&sil, r);
        let body_path = closed_path(&body_points, 1.0 / 6.0);

        // --- mouth --------------------------------------------------------
        // Anchored to the eyes the engine actually placed: it turns with the
        // head, but only three quarters as far as the eyes do, and it sits 36%
        // of the way from the eye line down to the rim. That held on all eight
        // silhouettes, so a mouth needs no per-shape table of its own.
        let pair = eye_poses(&gaze, r, p.split);
        let mut fitted = [(0.0, 0.0); 2];
        for (i, e) in pair.iter().enumerate() {
            let fit = radius_at_angle(&p.sil.radii, e.y.atan2(e.x) - p.sil.rot);
            fitted[i] = (e.x * fit, e.y * fit);
        }
        let (x0, y0) = fitted[0];
        let (x1, y1) = fitted[1];
        let mx = (x0 + x1) / 2.0;
        let my = (y0 + y1) / 2.0;
        // The eye line's own slope. A rolled head tilts its eyes, so the mouth has
        // to tilt with them; level under a canted face is what reads as a smirk.
        // Kept within a half turn: a gaze that sends the eyes round the sphere must
        // not turn the mouth upside down.
        let mut rot = ((y1 - y0).atan2(x1 - x0) * 180.0) / std::f64::consts::PI;
        if rot > 90.0 {
            rot -= 180.0;
        } else if rot <= -90.0 {
            rot += 180.0;
        }
        let mouth = if self.mouth && p.mouth.alpha > 0.01 {
            Some(MouthFrame {
                x: mx * 0.75 + (off_x + decalage.x) * r,
                y: my + 0.36 * (r - my) + (off_y + decalage.y) * r,
                rot,
                w: p.mouth.w * r,
                curve: p.mouth.curve * r,
                thickness: p.mouth.thickness * r,
                open: p.mouth.open * r,
                alpha: p.mouth.alpha,
            })
        } else {
            None
        };

        // --- eyes ---------------------------------------------------------
        let mut eyes = Vec::new();
        if p.eye_alpha > 0.01 {
            let poses = eye_poses(&gaze, r, p.split);
            for (i, eye) in poses.iter().enumerate() {
                let e = *eye;
                if e.depth <= 0.02 {
                    continue;
                }
                let cfg = p.eyes[i];
                let fit = radius_at_angle(&p.sil.radii, e.y.atan2(e.x) - p.sil.rot);
                let phi = (cfg.tilt * std::f64::consts::PI) / 180.0;
                let cp = phi.cos();
                let sp = phi.sin();
                let ax = e.a * cp + e.c * sp;
                let ay = e.b * cp + e.d * sp;
                let cx2 = -e.a * sp + e.c * cp;
                let cy2 = -e.b * sp + e.d * cp;
                let k = blink_scale(lid.min(cfg.open));
                eyes.push(RenderedEye {
                    d: capsule_path(cfg.w * r, cfg.h * r),
                    w: cfg.w * r,
                    h: cfg.h * r,
                    matrix: format!(
                        "matrix({},{},{},{},{},{})",
                        r2(ax),
                        r2(ay * k),
                        r2(cx2),
                        r2(cy2 * k),
                        r2(e.x * fit + (off_x + decalage.x) * r),
                        r2(e.y * fit + (off_y + decalage.y) * r)
                    ),
                    alpha: p.eye_alpha * clamp(e.depth / 0.12),
                });
            }
        }

        // --- decor --------------------------------------------------------
        let dots: Vec<DotRender> = p
            .dots
            .iter()
            .filter(|q| q.opacity > 0.01 && q.r > 0.0005)
            .map(|q| DotRender {
                x: (q.x + off_x) * r,
                y: (q.y + off_y) * r,
                r: q.r * r,
                opacity: q.opacity,
                color: q.color.clone(),
                depth: q.depth,
                d: q.d.clone(),
                rot: q.rot,
            })
            .collect();

        let n_fit = match p.notif {
            Some(n) => radius_at_angle(&p.sil.radii, n.y.atan2(n.x) - p.sil.rot),
            None => 1.0,
        };
        let (nx, ny) = match p.notif {
            Some(n) => ((n.x * n_fit + off_x) * r, (n.y * n_fit + off_y) * r),
            None => (0.0, 0.0),
        };
        let notif = p.notif.map(|n| NotifFrame {
            x: nx,
            y: ny,
            r: n.r * r,
        });
        let notch = p.notif.map(|n| NotifFrame {
            x: nx,
            y: ny,
            r: n.notch * r,
        });

        let arcs: Vec<ArcRender> = p
            .arcs
            .iter()
            .filter(|a| a.opacity > 0.01)
            .map(|a| arc_render(&a.seed, a.t, r, &a.id, a.opacity))
            .collect();

        BotFrame {
            body_path,
            body_points,
            body_alpha: p.body_alpha,
            eyes,
            mouth,
            dots,
            dots_behind: p.dots_behind,
            arcs,
            notif,
            notch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The switch has to hold everywhere, not only at rest: `wide` and `wink`
    /// carry their own face, and a mouth that came back mid-emote would read as a
    /// bug rather than as a setting.
    #[test]
    fn the_mouth_switch_empties_every_frame() {
        let mut engine = BotEngine::new(100.0, StateId::Idle, Some(ShapeId::Cercle), None);
        assert!(engine.sample(0.7).mouth.is_some());

        engine.set_mouth(false);
        for state in [StateId::Idle, StateId::Wide, StateId::Wink, StateId::Notify] {
            engine.reset(state, 0.0);
            assert!(
                engine.sample(0.8).mouth.is_none(),
                "{state:?} still had a mouth"
            );
        }

        engine.set_mouth(true);
        engine.reset(StateId::Idle, 0.0);
        assert!(engine.sample(0.7).mouth.is_some());
    }
}
