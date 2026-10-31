#!/usr/bin/env python3
"""Generates the Zoomify menu (src/menu.json, src/menu.escaped, src/menu_controls.rs)."""
import pathlib, sys
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from menukit import Menu

m = Menu("zoomify", "Zoomify", height=286)
m.tab("Zoom",
      [("head", "Zoom"), ("toggle", "enabled", "Mod enabled"), ("slider", "initial_zoom", "Zoom amount"),
       ("choice", "zoom_key_behaviour", "Zoom key"), ("choice", "spyglass", "Need spyglass")],
      [("head", "Animation"), ("slider", "zoom_in_time", "Zoom in time (s)"), ("slider", "zoom_out_time", "Zoom out time (s)"),
       ("choice", "zoom_in_transition", "Zoom in curve"), ("choice", "zoom_out_transition", "Zoom out curve")])
m.tab("Steps & View",
      [("head", "Step zoom"), ("toggle", "step_keys", "Step zoom keys"), ("slider", "scroll_step_count", "Steps"),
       ("slider", "zoom_per_step", "Zoom per step (%)"), ("slider", "scroll_zoom_smoothness", "Smoothness"),
       ("toggle", "retain_zoom_steps", "Remember steps")],
      [("head", "View"), ("slider", "relative_sensitivity", "Sensitivity (%)"), ("toggle", "cinematic_camera", "Cinematic zoom"),
       ("toggle", "show_crosshair", "Crosshair while zooming"), ("toggle", "hide_hud_on_zoom", "Hide HUD while zooming")])
m.tab("Keys & Presets",
      [("head", "Keys"), ("key", "bind_zoom", "Zoom"), ("key", "bind_step_in", "Step in"), ("key", "bind_step_out", "Step out"),
       ("key", "bind_settings", "This menu")],
      [("head", "Presets"), ("button", "preset_default", "Default"), ("button", "preset_optifine", "OptiFine"),
       ("button", "preset_okzoomer", "OK Zoomer")])
m.generate(pathlib.Path(__file__).resolve().parents[1] / "mods/template/src")
