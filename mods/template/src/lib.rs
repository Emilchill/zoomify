//! Zoomify for Cinnabar: a port of isXander's Zoomify (LGPL-3.0-or-later).
//!
//! Everything except `guest` is plain Rust, so the zoom maths, settings and panel can be
//! unit-tested on the host with `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

mod config;
mod menu;
mod mods_core;
mod menu_controls;
mod panel;
mod transition;
mod zoom;

#[cfg(target_arch = "wasm32")]
mod guest;

/// The menu with default settings, embedded in the component as its panel bundle.
pub fn menu_panel() -> String {
    panel::build(&config::Settings::default(), &panel::PanelState::default())
}

/// The bundle route of the menu's surface.
pub const MENU_ROUTE: &str = menu::ROUTE;
