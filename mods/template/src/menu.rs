//! The settings menu's panel JSON, in the shared Waypoints look (see `tools/menukit.py`).
//!
//! The document is generated once and embedded in the component as its panel bundle (see the
//! pack tool), so the mod never sends it at run time: copying it would cost a callback's worth
//! of fuel. The mod sends `patch`es with its controls' values and which tab is open; `build`
//! is the whole panel, for the bundle and for a changed menu key. The host draws every value,
//! flips switches, drags sliders and opens dropdowns.

use crate::menu_controls::{CONTROLS, DOCUMENT_ESCAPED, HEIGHT, Kind, MOVE, SCREEN, TABS, WIDTH};

/// A control's current value, as the mod sees it.
pub enum Value<'a> {
    Toggle(bool),
    /// Value, minimum, maximum, step.
    Slider(f64, f64, f64, f64),
    /// Selected index and the options.
    Choice(usize, &'a [&'a str]),
    /// The bound key.
    Key(&'a str),
    /// A button's text; `None` keeps the generated label.
    Button(Option<&'a str>),
}

/// A key no keyboard sends, as the panel's own toggle key: Cinnabar needs one, but these menus
/// open from Mods Core, or with a key the player sets.
pub const NO_KEY: &str = "F24";

/// Whether the player set a menu key of their own (F10 belongs to Mods Core).
pub fn has_own_key(key: &str) -> bool {
    !key.is_empty() && key != "None" && key != "F10" && key != NO_KEY
}

/// The chat line the Ko-fi button posts; Cinnabar's chat opens links in the browser.
pub const KOFI_MESSAGE: &str = "Like this mod? Buy me a coffee: https://ko-fi.com/emilchill (open the chat and click the link)";

/// Whether an event is the Ko-fi button.
pub fn is_kofi(id: &str) -> bool {
    id == "kofi"
}

thread_local! {
    /// Seconds the "look at the game chat" note stays up after a Ko-fi press.
    static KOFI_NOTE: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

/// Shows the "look at the game chat" note for a few seconds.
pub fn show_kofi_note() {
    KOFI_NOTE.set(4.0);
}

/// Counts the note down; true when it just went away, so the menu must be sent again.
pub fn kofi_tick(seconds: f32) -> bool {
    let left = KOFI_NOTE.get();
    if left <= 0.0 {
        return false;
    }
    let next = left - if seconds.is_finite() { seconds.clamp(0.0, 0.25) } else { 0.0 };
    KOFI_NOTE.set(next.max(0.0));
    next <= 0.0
}

/// Which tab an event id opens, if it is a tab.
pub fn tab_event(id: &str) -> Option<usize> {
    id.strip_prefix("tab_").and_then(|n| n.parse().ok()).filter(|n| *n < TABS.len())
}

/// The whole panel: `toggle_key` opens it, `capturing` is the keybind waiting for a key.
/// `moving` is the "move with the mouse" mode: the window hides and the HUD shows.
pub fn build<'a>(toggle_key: &str, capturing: Option<&str>, tab: usize, moving: bool, value: &dyn Fn(&str) -> Value<'a>) -> String {
    let mut o = String::with_capacity(DOCUMENT_ESCAPED.len() + 4096);
    o.push_str("{\"toggle_key\":");
    // The panel itself opens with no key: Mods Core opens it, and a key the player sets is
    // handled by the mod (see `has_own_key`).
    let _ = toggle_key;
    q(&mut o, NO_KEY);
    o.push_str(",\"motion\":true,\"capture_key\":");
    o.push_str(if capturing.is_some() { "true" } else { "false" });
    o.push_str(",\"show_gameplay_hud\":");
    o.push_str(if moving { "true" } else { "false" });
    o.push_str(",\"reference_size\":[");
    push_u(&mut o, u64::from(WIDTH + 20));
    o.push(',');
    push_u(&mut o, u64::from(HEIGHT + 20));
    o.push_str("],\"controls\":[");
    for (n, (kind, id, label)) in CONTROLS.iter().enumerate() {
        if n > 0 {
            o.push(',');
        }
        if let Some(t) = tab_event(id) {
            control(&mut o, "button", id, TABS[t]);
            o.push('}');
            continue;
        }
        match (kind, value(id)) {
            (Kind::Toggle, Value::Toggle(on)) => {
                control(&mut o, "toggle", id, label);
                o.push_str(if on { ",\"value\":true}" } else { ",\"value\":false}" });
            }
            (Kind::Slider, Value::Slider(v, min, max, step)) => {
                control(&mut o, "slider", id, label);
                o.push_str(",\"value\":");
                num(&mut o, v.clamp(min, max));
                o.push_str(",\"min\":");
                num(&mut o, min);
                o.push_str(",\"max\":");
                num(&mut o, max);
                o.push_str(",\"step\":");
                num(&mut o, step);
                o.push('}');
            }
            (Kind::Choice, Value::Choice(index, options)) => {
                control(&mut o, "choice", id, label);
                o.push_str(",\"index\":");
                push_u(&mut o, index.min(options.len().saturating_sub(1)) as u64);
                o.push_str(",\"options\":[");
                for (i, option) in options.iter().enumerate() {
                    if i > 0 {
                        o.push(',');
                    }
                    q(&mut o, option);
                }
                o.push_str("]}");
            }
            (Kind::Keybind, Value::Key(key)) => {
                control(&mut o, "keybind", id, label);
                o.push_str(",\"key\":");
                q(&mut o, if key.is_empty() { "None" } else { key });
                o.push_str(if capturing == Some(*id) { ",\"capturing\":true}" } else { ",\"capturing\":false}" });
            }
            (Kind::Button, Value::Button(text)) => {
                control(&mut o, "button", id, text.unwrap_or(label));
                o.push('}');
            }
            // Aliases, and any value that does not match its control, are plain buttons.
            _ => {
                control(&mut o, "button", id, label);
                o.push('}');
            }
        }
    }
    o.push_str("],\"surface\":{\"screen\":\"");
    o.push_str(SCREEN);
    o.push_str("\",\"document\":\"");
    o.push_str(DOCUMENT_ESCAPED);
    o.push_str("\",");
    tab_bindings(&mut o, tab, moving);
    o.push_str("}}");
    o
}

/// Every control's value and the open tab, for `panel.update-content`.
pub fn patch<'a>(capturing: Option<&str>, tab: usize, moving: bool, value: &dyn Fn(&str) -> Value<'a>) -> String {
    let mut o = String::with_capacity(64 + CONTROLS.len() * 48);
    o.push_str("{\"capture_key\":");
    o.push_str(if capturing.is_some() { "true" } else { "false" });
    o.push_str(",\"show_gameplay_hud\":");
    o.push_str(if moving { "true" } else { "false" });
    o.push_str(",\"controls\":[");
    let mut first = true;
    for (kind, id, label) in CONTROLS.iter() {
        if tab_event(id).is_some() {
            continue;
        }
        let v = value(id);
        let start = o.len();
        if !first {
            o.push(',');
        }
        o.push_str("{\"id\":");
        q(&mut o, id);
        match (kind, v) {
            (Kind::Toggle, Value::Toggle(on)) => o.push_str(if on { ",\"value\":true}" } else { ",\"value\":false}" }),
            (Kind::Slider, Value::Slider(v, min, max, _)) => {
                o.push_str(",\"value\":");
                num(&mut o, v.clamp(min, max));
                o.push('}');
            }
            (Kind::Choice, Value::Choice(index, options)) => {
                o.push_str(",\"index\":");
                push_u(&mut o, index.min(options.len().saturating_sub(1)) as u64);
                o.push('}');
            }
            (Kind::Keybind, Value::Key(key)) => {
                o.push_str(",\"key\":");
                q(&mut o, if key.is_empty() { "None" } else { key });
                o.push_str(if capturing == Some(*id) { ",\"capturing\":true}" } else { ",\"capturing\":false}" });
            }
            (Kind::Button, Value::Button(Some(text))) => {
                o.push_str(",\"label\":");
                q(&mut o, text);
                o.push('}');
            }
            (Kind::Button, Value::Button(None)) => {
                o.push_str(",\"label\":");
                q(&mut o, label);
                o.push('}');
            }
            // Aliases and mismatched values change nothing.
            _ => {
                o.truncate(start);
                continue;
            }
        }
        first = false;
    }
    o.push_str("],");
    tab_bindings(&mut o, tab, moving);
    o.push('}');
    o
}

/// `"bindings":{...}` for the open tab, and the window or the move bar.
fn tab_bindings(o: &mut String, tab: usize, moving: bool) {
    o.push_str("\"bindings\":{");
    for t in 0..TABS.len() {
        if t > 0 {
            o.push(',');
        }
        o.push_str("\"#tab_");
        push_u(o, t as u64);
        o.push_str("_on\":");
        o.push_str(if t == tab { "true" } else { "false" });
        o.push_str(",\"#tab_");
        push_u(o, t as u64);
        o.push_str("_col\":");
        o.push_str(if t == tab { "[0.29,0.62,1,1]" } else { "[0.135,0.15,0.185,1]" });
    }
    o.push_str(if KOFI_NOTE.get() > 0.0 { ",\"#kofi_sent\":true" } else { ",\"#kofi_sent\":false" });
    if MOVE {
        o.push_str(if moving { ",\"#v_window\":false,\"#v_move\":true" } else { ",\"#v_window\":true,\"#v_move\":false" });
    }
    o.push('}');
}

/// The surface's screen name, as a panel bundle names its route.
pub const ROUTE: &str = SCREEN;

fn control(o: &mut String, kind: &str, id: &str, label: &str) {
    o.push_str("{\"kind\":\"");
    o.push_str(kind);
    o.push_str("\",\"id\":");
    q(o, id);
    o.push_str(",\"label\":");
    q(o, label);
}

/// A JSON string.
pub fn q(o: &mut String, s: &str) {
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
}

pub fn push_u(o: &mut String, mut n: u64) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for b in &buf[i..] {
        o.push(*b as char);
    }
}

/// A number with at most three decimals, without float formatting machinery.
pub fn num(o: &mut String, v: f64) {
    let v = if v.is_finite() { v } else { 0.0 };
    let scaled = (v.abs() * 1000.0).round() as u64;
    if v < 0.0 && scaled != 0 {
        o.push('-');
    }
    push_u(o, scaled / 1000);
    let frac = scaled % 1000;
    if frac != 0 {
        o.push('.');
        let digits = [frac / 100, frac / 10 % 10, frac % 10];
        let last = if frac % 100 == 0 { 1 } else if frac % 10 == 0 { 2 } else { 3 };
        for d in &digits[..last] {
            o.push((b'0' + *d as u8) as char);
        }
    }
}
