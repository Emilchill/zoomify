//! The settings menu, replacing Zoomify's YACL screen (`SettingsGuiFactory.kt`), in the
//! Waypoints look (`tools/gen_menu.py`).
//!
//! Cinnabar renders the menu; the mod sends the values and reacts to events.
//! Event values: toggle 0/1, slider the new value, choice the option index, button and
//! keybind 1.

use crate::config::{
    Preset, Settings, SpyglassRequirement, ZoomKeyBehaviour, valid_key,
};
use crate::menu::{self, Value};
use crate::transition::{PANEL_TRANSITIONS, Transition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bind {
    Zoom,
    StepIn,
    StepOut,
    SettingsPanel,
}

impl Bind {
    fn id(self) -> &'static str {
        match self {
            Bind::Zoom => "bind_zoom",
            Bind::StepIn => "bind_step_in",
            Bind::StepOut => "bind_step_out",
            Bind::SettingsPanel => "bind_settings",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        [
            Bind::Zoom,
            Bind::StepIn,
            Bind::StepOut,
            Bind::SettingsPanel,
        ]
        .into_iter()
        .find(|bind| bind.id() == id)
    }

    fn slot(self, settings: &mut Settings) -> &mut String {
        match self {
            Bind::Zoom => &mut settings.keys.zoom,
            Bind::StepIn => &mut settings.keys.step_in,
            Bind::StepOut => &mut settings.keys.step_out,
            Bind::SettingsPanel => &mut settings.keys.settings_panel,
        }
    }
}

/// What a panel interaction changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effect {
    /// Settings need saving.
    pub save: bool,
    /// The panel JSON must be sent again.
    pub republish: bool,
    /// The set of reserved keys may have changed.
    pub rebind: bool,
}

#[derive(Debug, Default)]
pub struct PanelState {
    capturing: Option<Bind>,
    /// The open tab of the menu.
    tab: usize,
    /// The preset button that was just applied, and how long its "Done!" stays.
    done: Option<(&'static str, f32)>,
    notice: Option<&'static str>,
}

impl PanelState {
    /// Counts down the "Done!" label; republishes once when it expires.
    pub fn tick(&mut self, seconds: f32) -> Effect {
        let Some((id, left)) = self.done else {
            return Effect::default();
        };
        let left = left - seconds.clamp(0.0, 0.25);
        if left <= 0.0 {
            self.done = None;
            return Effect { republish: true, ..Effect::default() };
        }
        self.done = Some((id, left));
        Effect::default()
    }

    /// A short chat line confirming the last preset, if one was just applied.
    pub fn take_notice(&mut self) -> Option<&'static str> {
        self.notice.take()
    }

    fn preset_label(&self, id: &str, base: &'static str, done: &'static str) -> &'static str {
        if self.done.is_some_and(|(done_id, _)| done_id == id) { done } else { base }
    }

    #[allow(dead_code)]
    pub fn is_capturing(&self) -> bool {
        self.capturing.is_some()
    }

    /// Handles one panel event.
    pub fn event(&mut self, settings: &mut Settings, id: &str, value: f32) -> Effect {
        let before = settings.clone();
        let mut effect = Effect::default();
        let v = f64::from(value);
        let index = value.max(0.0) as usize;

        if let Some(tab) = menu::tab_event(id) {
            self.tab = tab;
            return Effect { republish: true, ..Effect::default() };
        }
        if let Some(bind) = Bind::from_id(id) {
            self.capturing = if self.capturing == Some(bind) { None } else { Some(bind) };
            return Effect { republish: true, ..Effect::default() };
        }
        match id {
            "enabled" => settings.enabled = value >= 0.5,
            "show_crosshair" => settings.show_crosshair = value >= 0.5,
            "hide_hud_on_zoom" => settings.hide_hud_on_zoom = value >= 0.5,
            "cinematic_camera" => settings.cinematic_camera = if value >= 0.5 { 100.0 } else { 0.0 },
            "initial_zoom" => settings.initial_zoom = v,
            "zoom_in_time" => settings.zoom_in_time = v,
            "zoom_out_time" => settings.zoom_out_time = v,
            "zoom_in_transition" => {
                if let Some(t) = transition_options(settings.zoom_in_transition)[index.min(7)] {
                    settings.zoom_in_transition = t;
                }
            }
            "zoom_out_transition" => {
                if let Some(t) = transition_options(settings.zoom_out_transition)[index.min(7)] {
                    settings.zoom_out_transition = t;
                }
            }
            "step_keys" => settings.step_keys = value >= 0.5,
            "scroll_step_count" => settings.scroll_step_count = v.round().max(1.0) as u32,
            "zoom_per_step" => settings.zoom_per_step = v,
            "scroll_zoom_smoothness" => settings.scroll_zoom_smoothness = v,
            "retain_zoom_steps" => settings.retain_zoom_steps = value >= 0.5,
            "zoom_key_behaviour" => {
                settings.zoom_key_behaviour = if index == 1 {
                    ZoomKeyBehaviour::Toggle
                } else {
                    ZoomKeyBehaviour::Hold
                }
            }
            "relative_sensitivity" => settings.relative_sensitivity = v,
            "spyglass" => {
                settings.spyglass = match index {
                    1 => SpyglassRequirement::OnlyWhileHolding,
                    2 => SpyglassRequirement::OnlyWhileCarrying,
                    _ => SpyglassRequirement::Off,
                }
            }
            "secondary_zoom_amount" => settings.secondary_zoom_amount = v,
            "secondary_zoom_in_time" => settings.secondary_zoom_in_time = v,
            "secondary_zoom_out_time" => settings.secondary_zoom_out_time = v,
            "preset_default" => {
                settings.apply_preset(Preset::Default);
                self.done = Some(("preset_default", 2.5));
                self.notice = Some("Zoomify: Default preset applied. Done!");
            }
            "preset_optifine" => {
                settings.apply_preset(Preset::Optifine);
                self.done = Some(("preset_optifine", 2.5));
                self.notice = Some("Zoomify: OptiFine preset applied. Done!");
            }
            "preset_okzoomer" => {
                settings.apply_preset(Preset::OkZoomer);
                self.done = Some(("preset_okzoomer", 2.5));
                self.notice = Some("Zoomify: OK Zoomer preset applied. Done!");
            }
            _ => return effect,
        }
        settings.sanitize();
        if *settings != before {
            effect.save = true;
            effect.rebind = settings.reserved_keys() != before.reserved_keys();
            // Always republish so the host's retained panel matches the new value.
            effect.republish = true;
        } else if id.starts_with("preset_") {
            effect.republish = true;
        }
        effect
    }

    /// While a keybind is waiting, the next key pressed becomes its binding; Escape cancels.
    pub fn capture(&mut self, settings: &mut Settings, pressed: &[String]) -> Effect {
        let Some(bind) = self.capturing else {
            return Effect::default();
        };
        for key in pressed {
            if key == "Escape" {
                self.capturing = None;
                // Escape clears the menu key: F10 opens the menu again.
                if bind == Bind::SettingsPanel {
                    settings.keys.settings_panel.clear();
                    return Effect { save: true, republish: true, rebind: true };
                }
                return Effect { republish: true, ..Effect::default() };
            }
            // Left click is how the panel itself is operated, including the keycap that
            // starts a capture, so it is never taken as the new binding.
            if key == "MouseLeft" {
                continue;
            }
            if valid_key(key) {
                *bind.slot(settings) = key.clone();
                self.capturing = None;
                return Effect { save: true, republish: true, rebind: true };
            }
        }
        Effect::default()
    }
}

/// The eight choices offered for a transition. `None` keeps a curve the panel can't list.
fn transition_options(current: Transition) -> [Option<Transition>; 8] {
    let mut options = PANEL_TRANSITIONS.map(Some);
    if !PANEL_TRANSITIONS.contains(&current) {
        options[7] = None;
    }
    options
}

fn transition_choice(current: Transition) -> (u32, Vec<String>) {
    let options = transition_options(current);
    let labels = options
        .iter()
        .map(|option| match option {
            Some(t) => t.label().to_owned(),
            None => format!("Custom: {}", current.label()),
        })
        .collect();
    let index = options
        .iter()
        .position(|option| option.is_none_or(|t| t == current))
        .unwrap_or(0) as u32;
    (index, labels)
}

/// The whole menu for the current settings (the bundle, or a changed menu key).
pub fn build(settings: &Settings, state: &PanelState) -> String {
    let names = Names::new(settings);
    let value = |id: &str| value(settings, state, &names, id);
    menu::build(&settings.keys.settings_panel, state.capturing.map(Bind::id), state.tab, false, &value)
}

/// Only the values, for a menu the host already has.
pub fn patch(settings: &Settings, state: &PanelState) -> String {
    let names = Names::new(settings);
    let value = |id: &str| value(settings, state, &names, id);
    menu::patch(state.capturing.map(Bind::id), state.tab, false, &value)
}

/// Which listed curve each transition dropdown shows.
struct Names {
    in_index: usize,
    out_index: usize,
}

impl Names {
    fn new(s: &Settings) -> Self {
        let (in_index, _) = transition_choice(s.zoom_in_transition);
        let (out_index, _) = transition_choice(s.zoom_out_transition);
        Names { in_index: in_index as usize, out_index: out_index as usize }
    }
}

/// One control's value.
fn value<'a>(s: &'a Settings, state: &'a PanelState, names: &'a Names, id: &str) -> Value<'a> {
    match id {
        "enabled" => Value::Toggle(s.enabled),
        "initial_zoom" => Value::Slider(s.initial_zoom, 1.0, 10.0, 0.5),
        "zoom_key_behaviour" => {
            Value::Choice(usize::from(s.zoom_key_behaviour == ZoomKeyBehaviour::Toggle), &["Hold", "Toggle"])
        }
        "spyglass" => Value::Choice(
            match s.spyglass {
                SpyglassRequirement::Off => 0,
                SpyglassRequirement::OnlyWhileHolding => 1,
                SpyglassRequirement::OnlyWhileCarrying => 2,
            },
            &["Never", "While holding", "While carrying"],
        ),
        "zoom_in_time" => Value::Slider(s.zoom_in_time, 0.0, 5.0, 0.05),
        "zoom_out_time" => Value::Slider(s.zoom_out_time, 0.0, 5.0, 0.05),
        "zoom_in_transition" => Value::Choice(names.in_index, &TRANSITION_NAMES),
        "zoom_out_transition" => Value::Choice(names.out_index, &TRANSITION_NAMES),
        "step_keys" => Value::Toggle(s.step_keys),
        "scroll_step_count" => Value::Slider(f64::from(s.scroll_step_count), 1.0, 20.0, 1.0),
        "zoom_per_step" => Value::Slider(s.zoom_per_step, 100.0, 300.0, 5.0),
        "scroll_zoom_smoothness" => Value::Slider(s.scroll_zoom_smoothness, 0.0, 100.0, 1.0),
        "retain_zoom_steps" => Value::Toggle(s.retain_zoom_steps),
        "relative_sensitivity" => Value::Slider(s.relative_sensitivity, 0.0, 100.0, 1.0),
        "cinematic_camera" => Value::Toggle(s.cinematic_camera > 0.0),
        "show_crosshair" => Value::Toggle(s.show_crosshair),
        "hide_hud_on_zoom" => Value::Toggle(s.hide_hud_on_zoom),
        "bind_zoom" => Value::Key(&s.keys.zoom),
        "bind_step_in" => Value::Key(&s.keys.step_in),
        "bind_step_out" => Value::Key(&s.keys.step_out),
        "bind_settings" => Value::Key(&s.keys.settings_panel),
        "preset_default" => Value::Button(Some(state.preset_label("preset_default", "Default", "Default: Done!"))),
        "preset_optifine" => Value::Button(Some(state.preset_label("preset_optifine", "OptiFine", "OptiFine: Done!"))),
        "preset_okzoomer" => Value::Button(Some(state.preset_label("preset_okzoomer", "OK Zoomer", "OK Zoomer: Done!"))),
        _ => Value::Button(None),
    }
}

/// The listed curves' names, in the dropdown's order.
const TRANSITION_NAMES: [&str; 8] = {
    let mut names = [""; 8];
    let mut i = 0;
    while i < 8 {
        names[i] = PANEL_TRANSITIONS[i].label();
        i += 1;
    }
    names
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashSet;

    /// Mirrors the host's `Panel::validate` limits.
    fn assert_valid(json: &str) {
        let doc: Value = serde_json::from_str(json).unwrap();
        assert!(json.len() <= 128 * 1024);
        assert!(doc.get("title").is_none() && doc["surface"]["document"].is_string());
        let toggle_key = doc["toggle_key"].as_str().unwrap();
        assert!(!toggle_key.is_empty() && toggle_key.bytes().all(|b| b.is_ascii_alphanumeric()));
        let controls = doc["controls"].as_array().unwrap();
        assert!(controls.len() <= 512);
        let mut ids = HashSet::new();
        for control in controls {
            let id = control["id"].as_str().unwrap();
            assert!(!id.is_empty() && id.len() <= 48, "{id}");
            assert!(id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b)));
            assert!(ids.insert(id.to_owned()), "duplicate {id}");
            assert!(control["label"].as_str().unwrap().len() <= 96);
            match control["kind"].as_str().unwrap() {
                "slider" => {
                    let f = |k: &str| control[k].as_f64().unwrap();
                    let (value, min, max, step) = (f("value"), f("min"), f("max"), f("step"));
                    assert!(min < max && value >= min && value <= max, "{id}: {value}");
                    assert!(step > 0.0 && step <= max - min && (max - min) / step <= 1_000_000.0);
                }
                "choice" => {
                    let options = control["options"].as_array().unwrap();
                    assert!(!options.is_empty() && options.len() <= 8, "{id}");
                    assert!((control["index"].as_u64().unwrap() as usize) < options.len());
                    assert!(options.iter().all(|o| o.as_str().unwrap().len() <= 96));
                }
                "keybind" => assert!(control["key"].as_str().unwrap().len() <= 48),
                "toggle" | "button" => {}
                other => panic!("unknown kind {other}"),
            }
        }
    }

    #[test]
    fn default_panel_is_valid() {
        assert_valid(&build(&Settings::default(), &PanelState::default()));
    }

    #[test]
    fn extreme_settings_still_make_valid_panels() {
        let mut s = Settings::default();
        s.initial_zoom = 10.0;
        s.zoom_per_step = 300.0;
        s.scroll_step_count = 20;
        s.zoom_in_transition = Transition::EaseInOutExp;
        s.zoom_out_transition = Transition::EaseInQuad;
        s.sanitize();
        assert_valid(&build(&s, &PanelState::default()));
    }

    #[test]
    fn unlisted_transition_is_shown_and_kept() {
        let mut s = Settings::default();
        s.zoom_in_transition = Transition::EaseInOutExp;
        let mut state = PanelState::default();
        let panel: Value = serde_json::from_str(&build(&s, &state)).unwrap();
        let control = panel["controls"].as_array().unwrap().iter().find(|c| c["id"] == "zoom_in_transition").unwrap();
        assert_eq!(control["index"], 7);
        // Choosing the custom entry again changes nothing.
        let effect = state.event(&mut s, "zoom_in_transition", 7.0);
        assert_eq!(effect, Effect::default());
        assert_eq!(s.zoom_in_transition, Transition::EaseInOutExp);
        // Picking a listed curve replaces it.
        state.event(&mut s, "zoom_in_transition", 1.0);
        assert_eq!(s.zoom_in_transition, Transition::Linear);
    }

    #[test]
    fn slider_toggle_and_choice_events_update_settings() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        let effect = state.event(&mut s, "initial_zoom", 6.0);
        assert_eq!(s.initial_zoom, 6.0);
        assert!(effect.save && effect.republish && !effect.rebind);
        let effect = state.event(&mut s, "step_keys", 0.0);
        assert!(!s.step_keys && effect.save && effect.rebind);
        let effect = state.event(&mut s, "zoom_key_behaviour", 1.0);
        assert_eq!(s.zoom_key_behaviour, ZoomKeyBehaviour::Toggle);
        assert!(effect.save && effect.republish);
        state.event(&mut s, "spyglass", 2.0);
        assert_eq!(s.spyglass, SpyglassRequirement::OnlyWhileCarrying);
        state.event(&mut s, "scroll_step_count", 7.4);
        assert_eq!(s.scroll_step_count, 7);
        assert_eq!(state.event(&mut s, "no_such_control", 1.0), Effect::default());
    }

    #[test]
    fn out_of_range_events_are_clamped() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        state.event(&mut s, "initial_zoom", 400.0);
        assert_eq!(s.initial_zoom, 10.0);
        state.event(&mut s, "zoom_in_time", f32::NAN);
        assert!(s.zoom_in_time.is_finite());
    }

    #[test]
    fn presets_republish_even_when_nothing_changes() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        let effect = state.event(&mut s, "preset_default", 1.0);
        assert!(effect.republish && !effect.save);
        let effect = state.event(&mut s, "preset_optifine", 1.0);
        assert!(effect.save && effect.republish && effect.rebind);
        assert!(!s.step_keys);
    }

    #[test]
    fn rebinding_a_key() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        assert!(state.event(&mut s, "bind_zoom", 1.0).republish);
        assert!(state.is_capturing());
        // Clicking the keycap produces a left-click edge, which is never a binding.
        assert_eq!(state.capture(&mut s, &["MouseLeft".into()]), Effect::default());
        assert!(state.is_capturing());
        let effect = state.capture(&mut s, &["KeyV".into()]);
        assert_eq!(s.keys.zoom, "KeyV");
        assert!(effect.save && effect.rebind && effect.republish);
        assert!(!state.is_capturing());
        let panel: Value = serde_json::from_str(&build(&s, &state)).unwrap();
        let zoom = panel["controls"].as_array().unwrap().iter().find(|c| c["id"] == "bind_zoom").unwrap();
        assert_eq!(zoom["key"], "KeyV");
    }

    #[test]
    fn preset_click_shows_done_then_expires() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        assert!(build(&s, &state).contains("\"OptiFine\""));
        let effect = state.event(&mut s, "preset_optifine", 1.0);
        assert!(effect.republish);
        assert!(build(&s, &state).contains("OptiFine: Done!"));
        assert!(state.take_notice().is_some());
        assert!(state.take_notice().is_none());
        assert!(!state.tick(0.1).republish);
        let expired = (0..20).any(|_| state.tick(0.25).republish);
        assert!(expired);
        assert!(build(&s, &state).contains("\"OptiFine\""));
        // Clicking a preset that changes nothing still confirms.
        state.event(&mut s, "preset_optifine", 1.0);
        assert!(build(&s, &state).contains("OptiFine: Done!"));
    }

    #[test]
    fn switches_for_mod_crosshair_and_cinematic() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        let effect = state.event(&mut s, "show_crosshair", 0.0);
        assert!(!s.show_crosshair && effect.save);
        let effect = state.event(&mut s, "enabled", 0.0);
        assert!(!s.enabled && effect.save && effect.rebind);
        assert!(s.reserved_keys().is_empty());
        state.event(&mut s, "cinematic_camera", 1.0);
        assert!(s.cinematic_camera > 0.0);
        state.event(&mut s, "cinematic_camera", 0.0);
        assert_eq!(s.cinematic_camera, 0.0);
        // Presets keep the master switch and the crosshair choice.
        state.event(&mut s, "preset_okzoomer", 1.0);
        assert!(!s.enabled && !s.show_crosshair);
        assert!(s.cinematic_camera > 0.0);
    }

    #[test]
    fn mouse_buttons_other_than_left_can_be_bound() {
        for button in ["MouseRight", "MouseMiddle"] {
            let mut s = Settings::default();
            let mut state = PanelState::default();
            state.event(&mut s, "bind_zoom", 1.0);
            let effect = state.capture(&mut s, &[button.into()]);
            assert_eq!(s.keys.zoom, button);
            assert!(effect.save && effect.rebind);
        }
        // Left click operates the panel and never becomes a binding, however often it arrives.
        let mut s = Settings::default();
        let mut state = PanelState::default();
        state.event(&mut s, "bind_zoom", 1.0);
        for _ in 0..3 {
            assert_eq!(state.capture(&mut s, &["MouseLeft".into()]), Effect::default());
        }
        assert_eq!(s.keys.zoom, "KeyZ");
        assert!(state.is_capturing());
    }

    #[test]
    fn escape_cancels_capture_and_second_click_toggles_it() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        state.event(&mut s, "bind_step_out", 1.0);
        assert!(build(&s, &state).contains("\"capture_key\":true"));
        let effect = state.capture(&mut s, &["Escape".into()]);
        assert!(effect.republish && !effect.save);
        assert_eq!(s.keys.step_out, "Minus");
        state.event(&mut s, "bind_step_in", 1.0);
        state.event(&mut s, "bind_step_in", 1.0);
        assert!(!state.is_capturing());
    }

    #[test]
    fn the_menu_key_is_the_mods_own() {
        let mut s = Settings::default();
        let mut state = PanelState::default();
        state.event(&mut s, "bind_settings", 1.0);
        state.capture(&mut s, &["F9".into()]);
        assert_eq!(s.keys.settings_panel, "F9");
        // The panel opens with no key of its own: the mod opens it on F9, Mods Core on a click.
        let panel: Value = serde_json::from_str(&build(&s, &state)).unwrap();
        assert_eq!(panel["toggle_key"], "F24");
        assert!(crate::menu::has_own_key("F9") && !crate::menu::has_own_key("F10"));
    }
}
