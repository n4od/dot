//! Port of `src/bot/expressions.ts`: the expression catalogue and its blend.

use super::face::HeadGaze;
use super::math::lerp;
use super::states::{lerp_eye_cfg, EyeCfg};
use super::ExprDef;

#[derive(Clone, Copy, Debug)]
pub struct Expression {
    pub gaze: HeadGaze,
    pub split: f64,
    pub eyes: [EyeCfg; 2],
}

impl Expression {
    pub fn from_def(def: &ExprDef) -> Self {
        Self {
            gaze: def.gaze,
            split: def.split,
            eyes: def.eyes,
        }
    }
}

fn lerp_eye(a: &EyeCfg, b: &EyeCfg, t: f64) -> EyeCfg {
    lerp_eye_cfg(a, b, t)
}

/// Interpolate two expressions: the change glides.
pub fn blend_expression(a: &Expression, b: &Expression, t: f64) -> Expression {
    Expression {
        gaze: HeadGaze {
            yaw: lerp(a.gaze.yaw, b.gaze.yaw, t),
            pitch: lerp(a.gaze.pitch, b.gaze.pitch, t),
            roll: lerp(a.gaze.roll, b.gaze.roll, t),
        },
        split: lerp(a.split, b.split, t),
        eyes: [
            lerp_eye(&a.eyes[0], &b.eyes[0], t),
            lerp_eye(&a.eyes[1], &b.eyes[1], t),
        ],
    }
}
