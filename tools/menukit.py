#!/usr/bin/env python3
"""Shared settings-menu generator for Emil Chill's Cinnabar mods, in the Waypoints look: a dark
rounded window, a title bar with a close button, tabs, and cards of rows (switches, sliders,
dropdowns, key caps and buttons).

A mod describes its menu as tabs of two columns of rows; `generate` writes the JSON-UI document
(`menu.json`, and `menu.escaped` to embed in a JSON string) and `menu_controls.rs`, the backing
controls in the order the document's buttons press them (`mod.control:<index>`).

The host draws the values: it publishes `#row_N_value`, `#row_N_label`, `#row_N_fill`,
`#row_N_toggle_progress`, `#row_N_knob_offset` and `#row_N_hover` for control N, flips
switches, drags sliders, opens dropdowns and eases everything. The mod only publishes the tab
bindings `#tab_N_on` and `#tab_N_col`.

Row kinds:
  ("head", text)                       a small accent heading
  ("toggle", id, label)                a switch; the whole row toggles it
  ("slider", id, label)                label, value and a draggable bar
  ("choice", id, label)                label and a dropdown
  ("key", id, label)                   label and a key cap (the mod captures the next key)
  ("button", id, label[, style])       a full-width button; its text is the control's label
  ("toggle_key", id, key_id, label)    a switch with its own key cap
  ("note", text)                       a muted line of text
"""
import json, pathlib, sys
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import kofi

FONT = "mod_panel"
BG = [0.055, 0.063, 0.082, 0.96]
CARD = [0.105, 0.117, 0.145, 1.0]
RAISED = [0.16, 0.18, 0.22, 1.0]
GHOST = [0.135, 0.15, 0.185, 1.0]
BORDER = [0.24, 0.27, 0.33, 1.0]
TEXT = [0.95, 0.96, 0.98, 1.0]
MUTED = [0.6, 0.65, 0.72, 1.0]
ACCENT = [0.29, 0.62, 1.0, 1.0]
DANGER = [0.35, 0.12, 0.13, 1.0]
WHITE = [1, 1, 1, 1]
TAB_ON = ACCENT
TAB_OFF = GHOST

ROW_H = {"head": 14, "note": 12, "toggle": 20, "slider": 24, "choice": 20, "key": 20, "button": 20, "toggle_key": 20}


def place(node, size, offset, anchor="top_left"):
    node.update({"size": size, "offset": offset, "anchor_from": anchor, "anchor_to": anchor})
    return node


def named(name, node):
    return {name: node}


def rect(size, offset, color, radius=4, color_binding=None, alpha=None, extra=None):
    n = place({"type": "custom", "renderer": "cinnabar_rounded_rectangle", "radius": radius, "color": color}, size, offset)
    b = []
    if color_binding:
        b.append({"binding_name": color_binding, "binding_name_override": "#color"})
    if alpha:
        b.append({"binding_name": alpha, "binding_name_override": "#alpha"})
    if extra:
        b.extend(extra)
    if b:
        n["bindings"] = b
    return n


def text(value, size, offset, color=TEXT, scale=0.8, align="left", bound=False, font=FONT):
    n = place({"type": "label", "text": value, "color": color, "localize": False, "font_type": font,
               "font_scale_factor": scale}, size, offset)
    if align != "left":
        n["text_alignment"] = align
    if bound:
        n["bindings"] = [{"binding_name": value}]
    return n


def panel(size, offset, children, visible=None):
    n = place({"type": "panel", "controls": children}, size, offset)
    if visible:
        n["bindings"] = [{"binding_name": visible, "binding_name_override": "#visible"}]
    return n


class Menu:
    def __init__(self, namespace, title, width=400, height=264, subtitle=None, credit="By Emil Chill & Claude AI",
                 move_hint=None):
        """`move_hint` adds a "move with the mouse" mode: the window hides, the HUD shows under
        the menu, and a bar at the top says `move_hint` with a Done button (`move_done`)."""
        self.move_hint = move_hint
        self.ns, self.title, self.W, self.H = namespace, title, width, height
        self.subtitle, self.credit = subtitle, credit
        self.controls = []  # (kind, id, label)
        self.tabs = []

    def control(self, kind, cid, label):
        for i, (k, c, _) in enumerate(self.controls):
            if c == cid:
                return i
        self.controls.append((kind, cid, label))
        return len(self.controls) - 1

    def button(self, cid, kind, label, size, offset, children, hover=True):
        idx = self.control(kind, cid, label)
        glow = [named("hover", rect(size, [0, 0], [1, 1, 1, 0.08], 4, alpha=f"#row_{idx}_hover"))] if hover else []
        return place({"type": "button", "controls": children + glow,
                      "button_mappings": [{"from_button_id": "button.menu_select", "to_button_id": f"mod.control:{idx}",
                                           "mapping_type": "pressed"}]}, size, offset)

    def tab(self, name, left, right=()):
        self.tabs.append((name, list(left), list(right)))

    # ------------------------------------------------------------------ rows
    def pill(self, idx, x, y):
        """A switch drawn from the host's toggle progress and knob offset."""
        return panel([22, 12], [x, y], [
            named("off", rect([22, 12], [0, 0], RAISED, 6)),
            named("on", rect([22, 12], [0, 0], ACCENT, 6, alpha=f"#row_{idx}_toggle_progress")),
            named("knob", rect([8, 8], [2, 2], WHITE, 4,
                               extra=[{"binding_name": f"#row_{idx}_knob_offset", "binding_name_override": "#offset"}])),
        ])

    def row(self, r, x, y, w):
        kind = r[0]
        out = []
        if kind == "head":
            out.append(text(r[1], [w, 10], [x, y + 2], ACCENT, 0.75))
        elif kind == "note":
            out.append(text(r[1], [w, 10], [x, y + 1], MUTED, 0.68))
        elif kind == "toggle":
            _, cid, label = r
            idx = self.control("toggle", cid, label)
            out.append(self.button(cid, "toggle", label, [w, 18], [x, y], [
                named("l", text(label, [w - 30, 10], [2, 4.5], TEXT, 0.8)),
                named("p", self.pill(idx, w - 24, 3)),
            ]))
        elif kind == "slider":
            _, cid, label = r
            idx = self.control("slider", cid, label)
            out.append(text(label, [w - 46, 10], [x + 2, y + 1], TEXT, 0.8))
            out.append(text(f"#row_{idx}_value", [44, 10], [x + w - 46, y + 1], ACCENT, 0.8, "right", True))
            fill = rect([w - 4, 4], [0, 0], ACCENT, 2,
                        extra=[{"binding_name": f"#row_{idx}_fill", "binding_name_override": "#size_binding_x"}])
            out.append(self.button(cid, "slider", label, [w - 4, 10], [x + 2, y + 11], [
                named("track", rect([w - 4, 4], [0, 3], RAISED, 2)),
                named("fillp", panel([w - 4, 4], [0, 3], [named("fill", fill)])),
            ], hover=False))
        elif kind == "choice":
            _, cid, label = r
            idx = self.control("choice", cid, label)
            bw = 86
            out.append(text(label, [w - bw - 6, 10], [x + 2, y + 4.5], TEXT, 0.8))
            out.append(self.button(cid, "choice", label, [bw, 16], [x + w - bw, y + 1], [
                named("bg", rect([bw, 16], [0, 0], RAISED, 5)),
                named("v", text(f"#row_{idx}_value", [bw - 18, 10], [6, 3.5], TEXT, 0.78, "left", True)),
                named("arrow", text("v", [10, 10], [bw - 13, 3], MUTED, 0.7, "center")),
            ]))
        elif kind == "key":
            _, cid, label = r
            idx = self.control("keybind", cid, label)
            bw = 66
            out.append(text(label, [w - bw - 6, 10], [x + 2, y + 4.5], TEXT, 0.8))
            out.append(self.button(cid, "keybind", label, [bw, 16], [x + w - bw, y + 1], [
                named("bg", rect([bw, 16], [0, 0], RAISED, 5)),
                named("k", text(f"#row_{idx}_value", [bw - 4, 10], [2, 3.5], TEXT, 0.8, "center", True)),
            ]))
        elif kind == "button":
            cid, label = r[1], r[2]
            style = r[3] if len(r) > 3 else "raised"
            idx = self.control("button", cid, label)
            fill = {"raised": RAISED, "accent": ACCENT, "danger": DANGER, "ghost": GHOST}[style]
            out.append(self.button(cid, "button", label, [w, 17], [x, y], [
                named("bg", rect([w, 17], [0, 0], fill, 5)),
                named("t", text(f"#row_{idx}_label", [w - 8, 10], [4, 4], TEXT, 0.8, "center", True)),
            ]))
        elif kind == "toggle_key":
            _, cid, kid, label = r
            t = self.control("toggle", cid, label)
            k = self.control("keybind", kid, label + " key")
            bw = 52
            out.append(self.button(cid + "__label", "alias", label, [w - bw - 30, 18], [x, y], [
                named("l", text(label, [w - bw - 34, 10], [2, 4.5], TEXT, 0.8))]))
            # The label area presses the same switch as the pill.
            out[-1]["button_mappings"][0]["to_button_id"] = f"mod.control:{t}"
            out[-1]["controls"][-1]["hover"]["bindings"][0]["binding_name"] = f"#row_{t}_hover"
            out.append(self.button(kid, "keybind", label + " key", [bw, 16], [x + w - bw - 28, y + 1], [
                named("bg", rect([bw, 16], [0, 0], RAISED, 5)),
                named("k", text(f"#row_{k}_value", [bw - 4, 10], [2, 3.5], TEXT, 0.75, "center", True)),
            ]))
            out.append(self.button(cid, "toggle", label, [26, 16], [x + w - 26, y + 1], [
                named("p", self.pill(t, 2, 2))], hover=False))
        else:
            raise ValueError(kind)
        return out

    # ------------------------------------------------------------------ document
    def build(self):
        W, H = self.W, self.H
        header = [
            named("title", text(self.title, [W - 60, 14], [12, 8], TEXT, 1.05)),
            named("close", place({"type": "button", "controls": [named("x", text("X", [18, 10], [0, 3.5], MUTED, 0.85, "center"))],
                                  "button_mappings": [{"from_button_id": "button.menu_select", "to_button_id": "mod.close",
                                                       "mapping_type": "pressed"}]}, [18, 16], [W - 26, 6])),
            named("rule", rect([W, 1], [0, 27], BORDER, 0)),
        ]
        if self.subtitle:
            header.append(named("sub", text(self.subtitle, [W - 140, 10], [12 + 7 * len(self.title) + 8, 11], MUTED, 0.75)))
        # Tabs first, so their indices are 0..tabs.
        tab_nodes = []
        tw = min(90, (W - 24 - 4 * (len(self.tabs) - 1)) // max(1, len(self.tabs)))
        for i, (name, _, _) in enumerate(self.tabs):
            tab_nodes.append(named(f"tab{i}", self.button(f"tab_{i}", "button", name, [tw, 16], [12 + i * (tw + 4), 33], [
                named("bg", rect([tw, 16], [0, 0], TAB_OFF, 5, color_binding=f"#tab_{i}_col")),
                named("t", text(name, [tw - 6, 10], [3, 3.5], TEXT, 0.8, "center")),
            ])))
        top = 56
        cw = (W - 24 - 8) // 2
        views = []
        for i, (name, left, right) in enumerate(self.tabs):
            kids = []
            for c, rows in enumerate([left, right]):
                if not rows:
                    continue
                cx = 12 + c * (cw + 8)
                height = sum(ROW_H[r[0]] + 3 for r in rows) + 10
                if top + height > H - 16:
                    raise ValueError(f"tab {name} column {c} is too tall: {height}")
                kids.append(named(f"card{c}", rect([cw, height], [cx, top], CARD, 6)))
                y = top + 6
                for n, r in enumerate(rows):
                    for m, node in enumerate(self.row(r, cx + 7, y, cw - 14)):
                        kids.append(named(f"r{c}_{n}_{m}", node))
                    y += ROW_H[r[0]] + 3
            views.append(named(f"view{i}", panel([W, H], [0, 0], kids, visible=f"#tab_{i}_on")))
        footer = [named("credit", text(self.credit, [W, 10], [0, H - 13], MUTED, 0.68, "center")),
                  named("kofi_note", kofi.note(rect, text, W, H, lambda size, off, kids, vis: panel(
                      size, off, [named(n, k) for n, k in kids], visible=vis)))]
        header.append(named("kofi", self.button("kofi", "button", "Support me on Ko-fi", [kofi.WIDTH, kofi.HEIGHT],
                                                [W - 32 - kofi.WIDTH, 6.5],
                                                [named(n, k) for n, k in kofi.nodes(rect, text, RAISED)])))
        window = place({"type": "panel", "controls": [
            named("shadow", rect([W + 6, H + 6], [-3, 0], [0, 0, 0, 0.35], 12)),
            named("edge", rect([W, H], [0, 0], BORDER, 10)),
            named("bg", rect([W - 2, H - 2], [1, 1], BG, 9)),
            *header, *tab_nodes, *views, *footer,
        ]}, [W, H], [0, 0], "center")
        controls = [named("window", window)]
        if self.move_hint:
            window["bindings"] = [{"binding_name": "#v_window", "binding_name_override": "#visible"}]
            bw = 300
            bar = place({"type": "panel", "controls": [
                named("edge", rect([bw, 30], [0, 0], BORDER, 8)),
                named("bg", rect([bw - 2, 28], [1, 1], BG, 7)),
                named("hint", text(self.move_hint, [bw - 80, 10], [10, 10], TEXT, 0.8)),
                named("done", self.button("move_done", "button", "Done", [58, 18], [bw - 66, 6], [
                    named("bg", rect([58, 18], [0, 0], ACCENT, 5)),
                    named("t", text("Done", [54, 10], [2, 4.5], TEXT, 0.8, "center"))])),
            ], "bindings": [{"binding_name": "#v_move", "binding_name_override": "#visible"}]}, [bw, 30], [0, 8], "top_middle")
            controls.append(named("movebar", bar))
        screen = {"type": "screen", "size": ["100%", "100%"], "controls": controls}
        return {"namespace": self.ns, "menu": screen}

    def generate(self, src_dir):
        src = pathlib.Path(src_dir)
        doc = self.build()
        text_doc = json.dumps(tidy(doc), separators=(",", ":"), ensure_ascii=False)
        (src / "menu.json").write_text(text_doc)
        (src / "menu.escaped").write_text(json.dumps(text_doc, ensure_ascii=False)[1:-1])
        kinds = {"toggle": "Kind::Toggle", "slider": "Kind::Slider", "choice": "Kind::Choice",
                 "keybind": "Kind::Keybind", "button": "Kind::Button", "alias": "Kind::Alias"}
        lines = ["//! Generated by tools/gen_menu.py: the menu's backing controls, in the order its",
                 "//! buttons press them (`mod.control:<index>`). Tabs come first.", "",
                 "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
                 "pub enum Kind {", "    Toggle,", "    Slider,", "    Choice,", "    Keybind,", "    Button,",
                 "    /// A second button pressing another control; sent as a plain button nobody presses.", "    #[allow(dead_code)]", "    Alias,", "}", "",
                 f"pub const TABS: [&str; {len(self.tabs)}] = [{', '.join(json.dumps(t[0]) for t in self.tabs)}];", "",
                 f"pub const CONTROLS: [(Kind, &str, &str); {len(self.controls)}] = ["]
        for kind, cid, label in self.controls:
            lines.append(f"    ({kinds[kind]}, {json.dumps(cid)}, {json.dumps(label, ensure_ascii=False)}),")
        lines += ["];", "", "/// The document, already escaped to sit inside a JSON string.",
                  'pub const DOCUMENT_ESCAPED: &str = include_str!("menu.escaped");', "",
                  f"pub const SCREEN: &str = \"{self.ns}.menu\";",
                  "/// Whether the menu has a \"move with the mouse\" mode.",
                  f"pub const MOVE: bool = {'true' if self.move_hint else 'false'};",
                  f"pub const WIDTH: u32 = {self.W};", f"pub const HEIGHT: u32 = {self.H};", ""]
        (src / "menu_controls.rs").write_text("\n".join(lines))
        print(f"{len(self.controls)} controls, {len(text_doc)} bytes, {count(doc['menu'])} nodes")


def tidy(v):
    if isinstance(v, float):
        return round(v, 3) if v != int(v) else int(v)
    if isinstance(v, list):
        return [tidy(x) for x in v]
    if isinstance(v, dict):
        return {k: tidy(x) for k, x in v.items()}
    return v


def count(n):
    c = 1
    for ch in n.get("controls", []):
        c += count(list(ch.values())[0])
    return c
