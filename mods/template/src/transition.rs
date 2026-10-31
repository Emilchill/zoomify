//! Easing curves, ported from Zoomify's `TransitionType.kt`.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

const C: f64 = 1023.0;

/// Names match Zoomify's serialized names (`ease_out_exp`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    Instant,
    Linear,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInExp,
    EaseOutExp,
    EaseInOutExp,
}

impl Transition {
    /// Maps linear progress `t` in 0..=1 to eased progress.
    pub fn apply(self, t: f64) -> f64 {
        use Transition::*;
        match self {
            Instant | Linear => t,
            EaseInSine => 1.0 - (t * PI / 2.0).cos(),
            EaseOutSine => (t * PI / 2.0).sin(),
            EaseInOutSine => -((PI * t).cos() - 1.0) / 2.0,
            EaseInQuad => t * t,
            EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EaseInCubic => t.powi(3),
            EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            EaseInExp => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    2f64.powf(10.0 * t - C.log2()) - 1.0 / C
                }
            }
            EaseOutExp => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2f64.powf(10.0 - C.log2() - 10.0 * t) + 1.0 / C
                }
            }
            EaseInOutExp => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else if t < 0.5 {
                    2f64.powf(20.0 * t - C.log2()) - 1.0 / C
                } else {
                    1.0 - 2f64.powf(10.0 - C.log2() - 10.0 * t) + 1.0 / C
                }
            }
        }
    }

    /// Inverse of [`apply`](Self::apply), when the curve has one (Zoomify's `hasInverse`).
    pub fn inverse(self, x: f64) -> Option<f64> {
        use Transition::*;
        let x = x.clamp(0.0, 1.0);
        let ln2_10 = 10.0 * 2f64.ln();
        Some(match self {
            EaseInSine => (1.0 - x).acos() * 2.0 / PI,
            EaseOutSine => x.asin() * 2.0 / PI,
            EaseInQuad => x.sqrt(),
            EaseOutQuad => 1.0 - (1.0 - x).sqrt(),
            EaseInCubic => x.cbrt(),
            EaseOutCubic => 1.0 - (1.0 - x).cbrt(),
            EaseInExp => {
                if x <= 0.0 {
                    0.0
                } else if x >= 1.0 {
                    1.0
                } else {
                    (C * x + 1.0).ln() / ln2_10
                }
            }
            EaseOutExp => {
                if x <= 0.0 {
                    0.0
                } else if x >= 1.0 {
                    1.0
                } else {
                    -(((-((C * x - 1024.0) / C)).ln() - ln2_10 + C.ln()) / ln2_10)
                }
            }
            Instant | Linear | EaseInOutSine | EaseInOutQuad | EaseInOutCubic | EaseInOutExp => {
                return None;
            }
        })
    }

    /// The curve used (run backwards) for zooming out.
    pub fn opposite(self) -> Self {
        use Transition::*;
        match self {
            Instant => Instant,
            Linear => Linear,
            EaseInSine => EaseOutSine,
            EaseOutSine => EaseInSine,
            EaseInOutSine => EaseInOutSine,
            EaseInQuad => EaseOutQuad,
            EaseOutQuad => EaseInQuad,
            EaseInOutQuad => EaseInOutQuad,
            EaseInCubic => EaseOutCubic,
            EaseOutCubic => EaseInCubic,
            EaseInOutCubic => EaseInOutCubic,
            EaseInExp => EaseOutExp,
            EaseOutExp => EaseInExp,
            EaseInOutExp => EaseInOutExp,
        }
    }

    pub const fn label(self) -> &'static str {
        use Transition::*;
        match self {
            Instant => "Instant",
            Linear => "Linear",
            EaseInSine => "Ease in Sine",
            EaseOutSine => "Ease out Sine",
            EaseInOutSine => "Ease in/out Sine",
            EaseInQuad => "Ease in Quad",
            EaseOutQuad => "Ease out Quad",
            EaseInOutQuad => "Ease in/out Quad",
            EaseInCubic => "Ease in Cubic",
            EaseOutCubic => "Ease out Cubic",
            EaseInOutCubic => "Ease in/out Cubic",
            EaseInExp => "Ease in Exponential",
            EaseOutExp => "Ease out Exponential",
            EaseInOutExp => "Ease in/out Exponential",
        }
    }
}

/// The settings panel's choice control holds at most eight options, so it offers these.
/// The other curves remain selectable by editing the settings file.
pub const PANEL_TRANSITIONS: [Transition; 8] = [
    Transition::Instant,
    Transition::Linear,
    Transition::EaseOutSine,
    Transition::EaseOutQuad,
    Transition::EaseOutCubic,
    Transition::EaseOutExp,
    Transition::EaseInOutSine,
    Transition::EaseInOutCubic,
];

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Transition; 14] = [
        Transition::Instant,
        Transition::Linear,
        Transition::EaseInSine,
        Transition::EaseOutSine,
        Transition::EaseInOutSine,
        Transition::EaseInQuad,
        Transition::EaseOutQuad,
        Transition::EaseInOutQuad,
        Transition::EaseInCubic,
        Transition::EaseOutCubic,
        Transition::EaseInOutCubic,
        Transition::EaseInExp,
        Transition::EaseOutExp,
        Transition::EaseInOutExp,
    ];

    #[test]
    fn endpoints_are_fixed() {
        for tr in ALL {
            assert!(tr.apply(0.0).abs() < 1e-9, "{tr:?} at 0");
            assert!((tr.apply(1.0) - 1.0).abs() < 1e-9, "{tr:?} at 1");
        }
    }

    #[test]
    fn curves_are_monotonic() {
        for tr in ALL {
            let mut prev = tr.apply(0.0);
            for i in 1..=200 {
                let v = tr.apply(i as f64 / 200.0);
                assert!(v + 1e-9 >= prev, "{tr:?} decreases at step {i}");
                prev = v;
            }
        }
    }

    #[test]
    fn inverse_round_trips() {
        for tr in ALL {
            if tr.inverse(0.5).is_none() {
                continue;
            }
            for i in 1..100 {
                let t = i as f64 / 100.0;
                let back = tr.inverse(tr.apply(t)).unwrap();
                assert!((back - t).abs() < 1e-6, "{tr:?}: {t} -> {back}");
            }
        }
    }

    #[test]
    fn opposite_is_an_involution() {
        for tr in ALL {
            assert_eq!(tr.opposite().opposite(), tr);
        }
    }

    #[test]
    fn serialized_names_match_zoomify() {
        let json = serde_json::to_string(&Transition::EaseOutExp).unwrap();
        assert_eq!(json, "\"ease_out_exp\"");
        let parsed: Transition = serde_json::from_str("\"ease_in_out_sine\"").unwrap();
        assert_eq!(parsed, Transition::EaseInOutSine);
    }
}
