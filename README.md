# Zoomify for Cinnabar

**⬇️ Download:** [`zoomify.wasm`](https://github.com/Emilchill/zoomify/raw/main/download/zoomify.wasm) (Cinnabar v0.1.31 or newer). Import it in **Settings → Mods → Import mod...**

A Cinnabar mod port of [Zoomify](https://github.com/isXander/Zoomify) 2.16.3 by isXander: a smooth,
configurable zoom with transitions, step zoom, relative sensitivity and presets.

Needs a Cinnabar build with the new mod API (October 25, 2026 or later).

Zoomify is licensed **LGPL-3.0-or-later**; this port is a derivative work under the same license
(see `LICENSE`, which is also embedded in the mod and shown by Cinnabar's Mods UI).

## Build and import

Install the Rust toolchain pinned in `rust-toolchain.toml` (1.99.0 with the `wasm32-unknown-unknown`
target), then run:

```powershell
python scripts\build-mod.py
```

The importable component is written to `dist\zoomify.wasm`. In Cinnabar open
**Settings → Mods → Import**, review the permissions below, then enable the mod.

Run the unit tests (zoom maths, settings, panel JSON limits) on any machine with:

```powershell
cargo test -p cinnabar-mod-template
```

## Permissions (`mods\template\pack\grants.json`)

| Grant          | Why                                                                              |
| -------------- | -------------------------------------------------------------------------------- |
| `camera`       | `camera.set-view-scale` applies the zoom (FOV) and the relative look sensitivity. |
| `controls`     | Reads the zoom keys, shows the settings panel, reserves the zoom keys so the game does not also act on them. |
| `settings`     | Saves your settings between sessions.                                            |
| `player_state` | Only read when **Require Spyglass** is not "Never", to look for a spyglass in your inventory. |

No WASI or operating-system imports are used.

## Using it

| Default key | Action                                              |
| ----------- | --------------------------------------------------- |
| `Z`         | Zoom (hold, or toggle in the settings)              |
| `=` / `-`   | Step zoom in / out while zooming                    |
| (none)      | Settings menu: open it from Mods Core (F10), or set a key |

The menu has three tabs (Zoom, Steps & View, Keys & Presets) with switches, sliders and
dropdowns. Every key can be rebound there: click its key cap, then press the new key (Escape
cancels).
Keep the zoom key from conflicting with a Cinnabar/Minecraft binding, as in the original.

The panel has the original's options: initial zoom, zoom in/out time and transition, step count,
zoom per step, step smoothness, remember steps, relative sensitivity and key behavior, and the **Default / OptiFine / OK Zoomer** presets. Settings are kept in
the mod's settings file as JSON, so a transition outside the panel's eight choices (the panel list
holds at most eight) can still be set by editing it, e.g. `"zoom_in_transition": "ease_in_out_exp"`.

## Differences from the Java mod

Cinnabar mods cannot hook the game's internals the way Fabric mixins do, so some features are
different or missing.

| Zoomify (Java)                  | This port                                                                  |
| ------------------------------- | -------------------------------------------------------------------------- |
| Zoom with `fov / divisor`       | Same, through `camera.set-view-scale`. **Maximum 10x** (the API's FOV limit is 0.1). Zooming out past 1x is not possible. |
| Scroll wheel zoom               | **Not available**: the wheel is not exposed to mods. Use the step keys (Zoomify's own "keybind scrolling" mode). |
| Relative sensitivity            | Yes, through the look scale (minimum 5%).                                  |
| Spyglass: combine/override, overlay, sounds | **Not available.** Replaced by "Require Spyglass" (never / while holding / while carrying). |
| Affect hand FOV                 | Not available (the hand cannot be controlled separately).                  |
| Relative view bobbing           | Not available.                                                             |
| Cinematic camera                | A heavier look while zoomed, with black cinematic bars.                    |
| Secondary zoom                  | Removed.                                                                   |
| `/zoomify` command, Mod Menu, Controlify | Mods Core (F10) opens the menu instead.                  |
| YACL screen, 14 transitions     | Cinnabar panel; 8 transitions listed (all 14 work from the settings file). |
| Translations                    | English only.                                                              |

Animation timing is the same, but runs per rendered frame instead of at 20 ticks per second.

## Template files

- `mods\template\src\zoom.rs` — zoom state machine (ported `Interpolator.kt`, `ZoomHelper.kt`).
- `mods\template\src\transition.rs` — easing curves (`TransitionType.kt`).
- `mods\template\src\config.rs` — settings, defaults, presets (`ZoomifySettings.kt`, `Presets.kt`).
- `mods\template\src\panel.rs` — settings panel and key rebinding.
- `mods\template\src\guest.rs` — Cinnabar entry point: `init` and `frame`.
- `mods\template\pack\grants.json` — host permissions.
- `mods\template\pack\src\main.rs` — packs the compiled module into a validated Cinnabar component.
- `assets\base-icon.png` — mod card icon (the template's Cinnabar logo).


## Phones and tablets (Android)

On touch controls this mod adds a **Zoom** button: tap to zoom in, tap again to zoom out. Move it with the touch layout editor.
