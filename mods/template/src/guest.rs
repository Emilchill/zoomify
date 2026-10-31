use crate::config::{Settings, SpyglassRequirement, ZoomKeyBehaviour};
use crate::panel::{Effect, PanelState};
use crate::zoom::{FrameInput, Zoomer};
use mod_api::bindings::{
    Guest,
    cinnabar::extension::{camera, hud, input, panel, player_state, settings, touch},
};
use std::cell::RefCell;

const SPYGLASS: &str = "minecraft:spyglass";
/// Bar height at full zoom in GUI pixels: each bar covers 62 of a 240-pixel-high screen.
const LETTERBOX_TARGET: f32 = 62.0;
/// The cinematic bars: two black strips on the mod's HUD layer whose height is `#bar`.
const LETTERBOX_DOCUMENT: &str = concat!(
    r##"{\"namespace\":\"zoomify\",\"hud\":{\"type\":\"screen\",\"size\":[\"100%\",\"100%\"],\"controls\":["##,
    r##"{\"top\":{\"type\":\"custom\",\"renderer\":\"cinnabar_rounded_rectangle\",\"radius\":0,\"color\":[0,0,0,1],"##,
    r##"\"size\":[\"100%\",0],\"anchor_from\":\"top_left\",\"anchor_to\":\"top_left\","##,
    r##"\"bindings\":[{\"binding_name\":\"#bar\",\"binding_name_override\":\"#size_binding_y_absolute\"}]}},"##,
    r##"{\"bottom\":{\"type\":\"custom\",\"renderer\":\"cinnabar_rounded_rectangle\",\"radius\":0,\"color\":[0,0,0,1],"##,
    r##"\"size\":[\"100%\",0],\"anchor_from\":\"bottom_left\",\"anchor_to\":\"bottom_left\","##,
    r##"\"bindings\":[{\"binding_name\":\"#bar\",\"binding_name_override\":\"#size_binding_y_absolute\"}]}}]}}"##
);

struct Mod;

/// The touch button that zooms on phones and tablets.
const TOUCH_ZOOM: &str = "zoomify-zoom";

#[derive(Default)]
struct State {
    settings: Settings,
    panel: PanelState,
    zoomer: Zoomer,
    zooming: bool,
    secondary_zooming: bool,
    /// What the host is currently showing, so each is written only when it changes.
    crosshair_hidden: bool,
    /// Current bar height in unscaled GUI pixels, and the last height written to the host
    /// in tenths of a pixel so the slide is only published when it visibly moves.
    letterbox: f32,
    letterbox_written: u32,
    /// Whether effect icons and the scoreboard are hidden.
    hud_hidden: bool,
    /// The bars' layer has been published.
    layer_up: bool,
    /// The on-screen "Zoom" button for touch controls is set.
    touch_set: bool,
    /// Zoom turned on with the touch button (a tap toggles it: a button cannot be held).
    touch_zoom: bool,

}

thread_local! {
    /// This mod's place in Mods Core's list.
    static CORE: RefCell<crate::mods_core::Link> = RefCell::new(crate::mods_core::Link::new(
        "zoomify",
        "Zoomify",
        env!("CARGO_PKG_VERSION"),
        "Smooth zoom with transitions, spyglass zoom, cinematic camera and letterbox bars.",
    ));

    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl Guest for Mod {
    fn init() {
        STATE.with_borrow_mut(|state| {
            state.settings = match settings::load() {
                Ok(text) => Settings::from_json(&text),
                Err(_) => Settings::default(),
            };

            state.apply(Effect {
                save: false,
                republish: true,
                rebind: true,
            });
        });
    }

    fn frame() {
        STATE.with_borrow_mut(State::frame);
    }
}

impl State {
    fn frame(&mut self) {
        let Ok(controls) = input::read_controls() else {
            return;
        };
        // Touch controls: a "Zoom" button (players can move it in the touch layout editor).
        if !self.touch_set {
            let button = touch::Button {
                id: TOUCH_ZOOM.into(),
                label: "Zoom".into(),
                anchor: touch::Anchor::Right,
                offset: touch::Point { x: 6.0, y: -36.0 },
                size: touch::Point { x: 40.0, y: 22.0 },
            };
            self.touch_set = touch::set_buttons(&[button]).is_ok();
        }
        if touch::read_presses().is_ok_and(|taps| taps.iter().any(|id| id == TOUCH_ZOOM)) {
            self.touch_zoom = !self.touch_zoom;
        }

        // A rebound menu key opens and closes the menu (the embedded menu knows the default).
        let menu_key = &self.settings.keys.settings_panel;
        if !self.panel.is_capturing()
            && crate::menu::has_own_key(menu_key)
            && controls.focused
            && controls.keys_pressed.iter().any(|k| k == menu_key)
        {
            core_toggle(controls.panel_open);
        }
        // Settings panel: events from its controls, and key capture for rebinding.
        let mut effect = Effect::default();
        let mut kofi_changed = crate::menu::kofi_tick(controls.seconds);
        // Mods Core: a click on this mod in its list sets and opens the menu.
        let whole = || crate::panel::build(&self.settings, &self.panel);
        if core_frame(controls.panel_open, whole) == crate::mods_core::Event::Opened {
            kofi_changed = true;
        }
        for event in &controls.events {
            if crate::menu::is_kofi(&event.id) {
                let _ = hud::local_message(crate::menu::KOFI_MESSAGE);
                crate::menu::show_kofi_note();
                kofi_changed = true;
                continue;
            }
            let next = self.panel.event(&mut self.settings, &event.id, event.value);
            effect = merge(effect, next);
        }
        let next = self.panel.capture(&mut self.settings, &controls.keys_pressed);
        let mut ticked = self.panel.tick(controls.seconds);
        ticked.republish |= kofi_changed;
        self.apply(merge(merge(effect, next), ticked));
        if let Some(message) = self.panel.take_notice() {
            let _ = hud::local_message(message);
        }

        // Master switch: no zoom, no camera output, and the zoom keys go back to the game.
        if !self.settings.enabled {
            self.zooming = false;
            self.secondary_zooming = false;
            self.zoomer = Zoomer::default();
            self.present(false, 0.0);
            return;
        }

        // Zoom keys only act while the player is in gameplay with nothing open.
        let pressed = |key: &str| controls.keys_pressed.iter().any(|k| k == key);
        let held = |key: &str| controls.keys_held.iter().any(|k| k == key);
        let active = controls.focused && controls.gameplay && !controls.panel_open;
        let behaviour = self.settings.zoom_key_behaviour;
        let mut step_delta = 0;
        if active {
            match behaviour {
                ZoomKeyBehaviour::Hold => self.zooming = held(&self.settings.keys.zoom) || self.touch_zoom,
                ZoomKeyBehaviour::Toggle => {
                    if pressed(&self.settings.keys.zoom) {
                        self.zooming = !self.zooming;
                    }
                    if self.touch_zoom {
                        // The touch button toggles in either mode.
                        self.zooming = !self.zooming;
                        self.touch_zoom = false;
                    }
                }
            }
            // The secondary zoom (F6) is removed: it never turns on.
            if self.settings.step_keys {
                if pressed(&self.settings.keys.step_in) {
                    step_delta += 1;
                }
                if pressed(&self.settings.keys.step_out) {
                    step_delta -= 1;
                }
            }
        } else if behaviour == ZoomKeyBehaviour::Hold {
            self.zooming = false;
        }

        if self.zooming && !spyglass_allows(self.settings.spyglass) {
            self.zooming = false;
            self.touch_zoom = false;
        }

        let seconds = f64::from(controls.seconds);
        let view = self.zoomer.update(
            &FrameInput {
                zooming: self.zooming,
                secondary_zooming: self.secondary_zooming,
                step_delta,
                seconds: if seconds.is_finite() { seconds.clamp(0.0, 0.25) } else { 0.0 },
                retain_steps: self.settings.retain_zoom_steps,
                relative_sensitivity: self.settings.relative_sensitivity / 100.0,
            },
            &self.settings.primary_params(),
            &self.settings.secondary_params(),
        );
        // The host restores the normal view whenever this is not renewed.
        let zoomed = !view.is_neutral();
        if zoomed {
            // Cinematic camera: a heavier, slower look while zoomed (the host exposes no smoothing).
            let heavy = 1.0 - 0.6 * (self.settings.cinematic_camera / 100.0) as f32;
            let look = (view.look_scale * heavy).clamp(0.05, 1.0);
            let _ = camera::set_view_scale(view.fov_scale, look);
        }
        // How far the zoom has travelled toward the configured amount, 0 at rest and 1 once
        // it is fully in. Steps and the secondary zoom push past it, so this is clamped.
        let full = self.settings.initial_zoom.max(1.000_001);
        let fraction = ((view.divisor - 1.0) / (full - 1.0)).clamp(0.0, 1.0) as f32;
        self.present(self.zooming || self.secondary_zooming, fraction);
    }

    /// Hides the crosshair while a zoom key is held and sizes the cinematic bars from
    /// `zoom_fraction` (0 at rest, 1 at the configured zoom), writing to the HUD only when
    /// the bars have visibly moved.
    fn present(&mut self, holding: bool, zoom_fraction: f32) {
        // Cinematic zoom always takes the crosshair with it; otherwise the panel switch decides.
        // Tied to the key itself, not to the view, so the crosshair returns the moment the
        // zoom is released instead of waiting out the zoom-out transition.
        let hide_crosshair = holding
            && (!self.settings.show_crosshair
                || self.settings.cinematic_camera > 0.0
                || self.settings.hide_hud_on_zoom);
        let hide_hud = holding && self.settings.hide_hud_on_zoom;
        if hide_crosshair != self.crosshair_hidden || hide_hud != self.hud_hidden {
            let mut hidden = Vec::with_capacity(3);
            if hide_crosshair {
                hidden.push(hud::VanillaElement::Crosshair);
            }
            if hide_hud {
                hidden.extend([hud::VanillaElement::StatusEffects, hud::VanillaElement::Sidebar]);
            }
            if hud::set_hidden_elements(&hidden).is_ok() {
                self.crosshair_hidden = hide_crosshair;
                self.hud_hidden = hide_hud;
            }
        }
        // The bars track how far the zoom itself has travelled, so they slide in from the
        // edges on the zoom's own curve and retract with it.
        self.letterbox = if self.settings.cinematic_camera > 0.0 {
            LETTERBOX_TARGET * zoom_fraction
        } else {
            0.0
        };
        // Tenths of a GUI pixel: fine enough to look continuous, coarse enough to stop
        // writing once the bars have settled.
        let written = (self.letterbox * 10.0) as u32;
        if written != self.letterbox_written || !self.layer_up {
            let height = written as f32 / 10.0;
            let mut value = String::with_capacity(16);
            crate::menu::num(&mut value, f64::from(height));
            let sent = if self.layer_up {
                hud::update_surface(&format!("{{\"binding_changes\":{{\"#bar\":{value}}}}}"))
            } else {
                hud::set_surface(&format!(
                    "{{\"screen\":\"zoomify.hud\",\"document\":\"{LETTERBOX_DOCUMENT}\",\"bindings\":{{\"#bar\":{value}}}}}"
                ))
            };
            if sent.is_ok() {
                self.layer_up = true;
                self.letterbox_written = written;
            } else {
                self.layer_up = false;
            }
        }
    }

    fn apply(&mut self, effect: Effect) {
        if effect.save {
            let _ = settings::save(&self.settings.to_json());
        }
        if effect.rebind {
            let _ = input::reserve_keys(&self.settings.reserved_keys());
        }
        if effect.republish {
            // Values go as a patch: the embedded menu already has the document. The whole menu
            // is sent only if the host has none.
            if panel::update_content(&crate::panel::patch(&self.settings, &self.panel)).is_err() {
                // No panel while the menu is closed: it is set whole when it opens.
            }
        }
    }
}

fn merge(a: Effect, b: Effect) -> Effect {
    Effect {
        save: a.save || b.save,
        republish: a.republish || b.republish,
        rebind: a.rebind || b.rebind,
    }
}

/// Zoomify's "only zoom while holding/carrying a spyglass" behaviours.
fn spyglass_allows(requirement: SpyglassRequirement) -> bool {
    if requirement == SpyglassRequirement::Off {
        return true;
    }
    let Ok(Some(snapshot)) = player_state::read_snapshot() else {
        return false;
    };
    let is_spyglass = |slot: &player_state::Slot| {
        slot.item
            .as_ref()
            .and_then(|item| item.identifier.as_deref())
            == Some(SPYGLASS)
    };
    match requirement {
        SpyglassRequirement::Off => true,
        SpyglassRequirement::OnlyWhileHolding => {
            is_spyglass(&snapshot.offhand)
                || snapshot
                    .selected_slot
                    .and_then(|slot| snapshot.inventory.get(usize::from(slot)))
                    .is_some_and(is_spyglass)
        }
        SpyglassRequirement::OnlyWhileCarrying => {
            is_spyglass(&snapshot.offhand) || snapshot.inventory.iter().any(is_spyglass)
        }
    }
}

mod_api::bindings::export!(Mod with_types_in mod_api::bindings);

/// Announces this mod to Mods Core, and sets and opens the menu when asked (or drops it when it
/// closes, so only one menu exists at a time; see `mods_core.rs`).
fn core_frame(panel_open: bool, whole_panel: impl FnOnce() -> String) -> crate::mods_core::Event {
    CORE.with_borrow_mut(|link| link.frame(panel_open, whole_panel))
}

/// The player's own menu key: opens the menu through Mods Core's hand-over, or closes it.
fn core_toggle(panel_open: bool) {
    CORE.with_borrow_mut(|link| link.toggle(panel_open));
}
