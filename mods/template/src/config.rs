//! Settings, defaults and presets, ported from Zoomify's `ZoomifySettings.kt` and `Presets.kt`.

use crate::transition::Transition;
use crate::zoom::{MAX_DIVISOR, ZoomParams, secondary_params};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoomKeyBehaviour {
    #[default]
    Hold,
    Toggle,
}

/// Replaces Zoomify's four spyglass behaviours: Cinnabar mods cannot touch the spyglass
/// overlay, sound or vanilla zoom, but they can require the item before zooming.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpyglassRequirement {
    #[default]
    Off,
    OnlyWhileHolding,
    OnlyWhileCarrying,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
    pub zoom: String,
    pub secondary_zoom: String,
    pub step_in: String,
    pub step_out: String,
    pub settings_panel: String,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            zoom: "KeyZ".into(),
            secondary_zoom: "F6".into(),
            step_in: "Equal".into(),
            step_out: "Minus".into(),
            // Empty: F10 opens the menu (Mod Menu).
            settings_panel: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub initial_zoom: f64,
    pub zoom_in_time: f64,
    pub zoom_out_time: f64,
    pub zoom_in_transition: Transition,
    pub zoom_out_transition: Transition,
    pub retain_zoom_steps: bool,
    /// Zoomify's `scrollZoom`. The wheel is not exposed to mods, so this enables the step keys.
    pub step_keys: bool,
    pub scroll_step_count: u32,
    pub zoom_per_step: f64,
    pub scroll_zoom_smoothness: f64,
    pub zoom_key_behaviour: ZoomKeyBehaviour,
    pub relative_sensitivity: f64,
    pub spyglass: SpyglassRequirement,
    pub secondary_zoom_amount: f64,
    pub secondary_zoom_in_time: f64,
    pub secondary_zoom_out_time: f64,
    /// Master switch: when off the mod never zooms and gives its keys back to the game.
    pub enabled: bool,
    /// Zoomify's cinematic camera, approximated as a heavier camera while zooming (0-100).
    pub cinematic_camera: f64,
    /// Off hides the player's crosshair.
    pub show_crosshair: bool,
    /// Hides the crosshair, status-effect icons and scoreboard while zooming.
    pub hide_hud_on_zoom: bool,
    pub keys: Keys,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            initial_zoom: 4.0,
            zoom_in_time: 1.0,
            zoom_out_time: 0.5,
            zoom_in_transition: Transition::EaseOutExp,
            zoom_out_transition: Transition::EaseOutExp,
            retain_zoom_steps: false,
            step_keys: true,
            scroll_step_count: 10,
            zoom_per_step: 150.0,
            scroll_zoom_smoothness: 70.0,
            zoom_key_behaviour: ZoomKeyBehaviour::Hold,
            relative_sensitivity: 100.0,
            spyglass: SpyglassRequirement::Off,
            secondary_zoom_amount: 4.0,
            secondary_zoom_in_time: 10.0,
            secondary_zoom_out_time: 1.0,
            enabled: true,
            cinematic_camera: 0.0,
            show_crosshair: true,
            hide_hud_on_zoom: false,
            keys: Keys::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Default,
    Optifine,
    OkZoomer,
}

impl Settings {
    /// Parses stored JSON, falling back to defaults for anything missing or invalid.
    pub fn from_json(text: &str) -> Self {
        let mut settings = serde_json::from_str::<Settings>(text).unwrap_or_default();
        settings.sanitize();
        settings
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    /// Keeps every value inside what the panel and the camera API accept.
    pub fn sanitize(&mut self) {
        fn fix(value: &mut f64, min: f64, max: f64, default: f64) {
            *value = if value.is_finite() {
                value.clamp(min, max)
            } else {
                default
            };
        }
        let d = Settings::default();
        fix(&mut self.initial_zoom, 1.0, MAX_DIVISOR, d.initial_zoom);
        fix(&mut self.zoom_in_time, 0.0, 5.0, d.zoom_in_time);
        fix(&mut self.zoom_out_time, 0.0, 5.0, d.zoom_out_time);
        fix(&mut self.zoom_per_step, 100.0, 300.0, d.zoom_per_step);
        fix(&mut self.scroll_zoom_smoothness, 0.0, 100.0, d.scroll_zoom_smoothness);
        fix(&mut self.relative_sensitivity, 0.0, 100.0, d.relative_sensitivity);
        fix(&mut self.secondary_zoom_amount, 1.0, MAX_DIVISOR, d.secondary_zoom_amount);
        fix(&mut self.secondary_zoom_in_time, 0.0, 30.0, d.secondary_zoom_in_time);
        fix(&mut self.secondary_zoom_out_time, 0.0, 30.0, d.secondary_zoom_out_time);
        fix(&mut self.cinematic_camera, 0.0, 100.0, d.cinematic_camera);
        self.scroll_step_count = self.scroll_step_count.clamp(1, 20);
        let defaults = Keys::default();
        for (key, default) in [
            (&mut self.keys.zoom, defaults.zoom),
            (&mut self.keys.secondary_zoom, defaults.secondary_zoom),
            (&mut self.keys.step_in, defaults.step_in),
            (&mut self.keys.step_out, defaults.step_out),
        ] {
            if !valid_key(key) {
                *key = default;
            }
        }
        if !valid_key(&self.keys.settings_panel) {
            self.keys.settings_panel.clear();
        }
    }

    pub fn apply_preset(&mut self, preset: Preset) {
        let keys = self.keys.clone();
        let (enabled, show_crosshair, hide_hud) = (self.enabled, self.show_crosshair, self.hide_hud_on_zoom);
        *self = Settings::default();
        self.keys = keys;
        self.enabled = enabled;
        self.show_crosshair = show_crosshair;
        self.hide_hud_on_zoom = hide_hud;
        match preset {
            Preset::Default => {}
            Preset::Optifine => {
                self.zoom_in_transition = Transition::Instant;
                self.zoom_out_transition = Transition::Instant;
                self.step_keys = false;
                self.relative_sensitivity = 0.0;
            }
            Preset::OkZoomer => {
                self.zoom_in_time = 0.25;
                self.zoom_out_time = 0.25;
                self.relative_sensitivity = 50.0;
                self.scroll_zoom_smoothness = 25.0;
                self.cinematic_camera = 100.0;
            }
        }
    }

    pub fn primary_params(&self) -> ZoomParams {
        ZoomParams {
            initial_zoom: self.initial_zoom,
            transition_in: self.zoom_in_transition,
            transition_out: self.zoom_out_transition,
            time_in: self.zoom_in_time,
            time_out: self.zoom_out_time,
            zoom_per_step: self.zoom_per_step,
            max_steps: if self.step_keys {
                self.scroll_step_count
            } else {
                0
            },
            // Zoomify: lerp(scrollZoomSmoothness / 100, 1.0, 0.1)
            step_smoothness: 1.0 + (self.scroll_zoom_smoothness / 100.0) * (0.1 - 1.0),
        }
    }

    pub fn secondary_params(&self) -> ZoomParams {
        secondary_params(
            self.secondary_zoom_amount,
            self.secondary_zoom_in_time,
            self.secondary_zoom_out_time,
        )
    }

    /// Every physical key the mod must keep away from ordinary gameplay input.
    pub fn reserved_keys(&self) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }
        let mut keys = vec![self.keys.zoom.clone()];
        if self.step_keys {
            keys.push(self.keys.step_in.clone());
            keys.push(self.keys.step_out.clone());
        }
        keys.sort();
        keys.dedup();
        keys
    }
}

/// Physical key names are short ASCII alphanumerics such as `KeyC`, `F6` or `Digit1`.
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 32
        && key.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_zoomify() {
        let s = Settings::default();
        assert_eq!(s.initial_zoom, 4.0);
        assert_eq!(s.zoom_in_time, 1.0);
        assert_eq!(s.zoom_out_time, 0.5);
        assert_eq!(s.zoom_in_transition, Transition::EaseOutExp);
        assert_eq!(s.scroll_step_count, 10);
        assert_eq!(s.zoom_per_step, 150.0);
        assert_eq!(s.scroll_zoom_smoothness, 70.0);
        assert_eq!(s.relative_sensitivity, 100.0);
        assert_eq!(s.secondary_zoom_in_time, 10.0);
    }

    #[test]
    fn smoothness_mapping_matches_zoomify() {
        let p = Settings::default().primary_params();
        assert!((p.step_smoothness - 0.37).abs() < 1e-9);
        let mut s = Settings::default();
        s.scroll_zoom_smoothness = 0.0;
        assert_eq!(s.primary_params().step_smoothness, 1.0);
        s.scroll_zoom_smoothness = 100.0;
        assert!((s.primary_params().step_smoothness - 0.1).abs() < 1e-9);
    }

    #[test]
    fn json_round_trip_and_garbage() {
        let mut s = Settings::default();
        s.initial_zoom = 6.5;
        s.keys.zoom = "KeyV".into();
        assert_eq!(Settings::from_json(&s.to_json()), s);
        assert_eq!(Settings::from_json(""), Settings::default());
        assert_eq!(Settings::from_json("not json"), Settings::default());
    }

    #[test]
    fn partial_files_keep_defaults() {
        let s = Settings::from_json(r#"{"initial_zoom": 2.0, "zoom_in_transition": "linear"}"#);
        assert_eq!(s.initial_zoom, 2.0);
        assert_eq!(s.zoom_in_transition, Transition::Linear);
        assert_eq!(s.zoom_out_time, 0.5);
    }

    #[test]
    fn sanitize_clamps_and_repairs_keys() {
        let s = Settings::from_json(
            r#"{"initial_zoom": 99, "zoom_in_time": -3, "scroll_step_count": 0,
                "keys": {"zoom": "Mouse Left!", "step_in": "MouseLeft"}}"#,
        );
        assert_eq!(s.initial_zoom, MAX_DIVISOR);
        assert_eq!(s.zoom_in_time, 0.0);
        assert_eq!(s.scroll_step_count, 1);
        assert_eq!(s.keys.zoom, "KeyZ");
        assert_eq!(s.keys.step_in, "MouseLeft");
    }

    #[test]
    fn presets_follow_zoomify_and_keep_keys() {
        let mut s = Settings::default();
        s.keys.zoom = "KeyV".into();
        s.initial_zoom = 8.0;
        s.apply_preset(Preset::Optifine);
        assert_eq!(s.zoom_in_transition, Transition::Instant);
        assert_eq!(s.zoom_out_transition, Transition::Instant);
        assert!(!s.step_keys);
        assert_eq!(s.relative_sensitivity, 0.0);
        assert_eq!(s.initial_zoom, 4.0);
        assert_eq!(s.keys.zoom, "KeyV");
        s.apply_preset(Preset::OkZoomer);
        assert_eq!(s.zoom_in_time, 0.25);
        assert_eq!(s.relative_sensitivity, 50.0);
        assert_eq!(s.scroll_zoom_smoothness, 25.0);
        assert!(s.step_keys);
        s.apply_preset(Preset::Default);
        assert_eq!(s, Settings { keys: s.keys.clone(), ..Settings::default() });
    }

    #[test]
    fn reserved_keys_follow_step_keys_setting() {
        let mut s = Settings::default();
        assert_eq!(s.reserved_keys(), ["Equal", "KeyZ", "Minus"]);
        s.step_keys = false;
        assert_eq!(s.reserved_keys(), ["KeyZ"]);
    }
}
