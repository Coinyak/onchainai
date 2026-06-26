//! Browser localStorage helpers (client-only; no-op on SSR).

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
use std::collections::HashMap;

const SIDEBAR_COLLAPSED_KEY: &str = "onchain-ai-sidebar-collapsed";
#[cfg(target_arch = "wasm32")]
const SIDEBAR_SECTIONS_KEY: &str = "onchain-ai-sidebar-sections";

pub fn read_bool(key: &str, default: bool) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        default
    }
    #[cfg(target_arch = "wasm32")]
    {
        if !is_browser() {
            return default;
        }
        let win = window();
        let Ok(Some(storage)) = win.local_storage() else {
            return default;
        };
        let Ok(Some(raw)) = storage.get_item(key) else {
            return default;
        };
        raw == "1" || raw == "true"
    }
}

pub fn write_bool(key: &str, value: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, value);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if !is_browser() {
            return;
        }
        let win = window();
        let Ok(Some(storage)) = win.local_storage() else {
            return;
        };
        let _ = storage.set_item(key, if value { "1" } else { "0" });
    }
}

pub fn read_sidebar_sections(default: HashMap<String, bool>) -> HashMap<String, bool> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        default
    }
    #[cfg(target_arch = "wasm32")]
    {
        if !is_browser() {
            return default;
        }
        let win = window();
        let Ok(Some(storage)) = win.local_storage() else {
            return default;
        };
        let Ok(Some(raw)) = storage.get_item(SIDEBAR_SECTIONS_KEY) else {
            return default;
        };
        serde_json::from_str(&raw).unwrap_or(default)
    }
}

pub fn write_sidebar_sections(map: &HashMap<String, bool>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = map;
    }
    #[cfg(target_arch = "wasm32")]
    {
        if !is_browser() {
            return;
        }
        if let Ok(json) = serde_json::to_string(map) {
            write_raw(SIDEBAR_SECTIONS_KEY, &json);
        }
    }
}

pub fn read_sidebar_collapsed() -> bool {
    read_bool(SIDEBAR_COLLAPSED_KEY, false)
}

pub fn read_sidebar_collapsed_with_default(default: bool) -> bool {
    read_bool(SIDEBAR_COLLAPSED_KEY, default)
}

pub fn sidebar_default_collapsed_for_width(width: f64) -> bool {
    width < 768.0
}

#[cfg(target_arch = "wasm32")]
pub fn sidebar_default_collapsed_for_viewport() -> bool {
    let win = window();
    win.inner_width()
        .ok()
        .and_then(|width| width.as_f64())
        .map(sidebar_default_collapsed_for_width)
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sidebar_default_collapsed_for_viewport() -> bool {
    false
}

pub fn write_sidebar_collapsed(collapsed: bool) {
    write_bool(SIDEBAR_COLLAPSED_KEY, collapsed);
}

#[cfg(target_arch = "wasm32")]
fn write_raw(key: &str, value: &str) {
    if !is_browser() {
        return;
    }
    let win = window();
    if let Ok(Some(storage)) = win.local_storage() {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bool_defaults_on_ssr() {
        assert!(!read_bool("missing-key", false));
        assert!(read_bool("missing-key", true));
    }

    #[test]
    fn mobile_sidebar_defaults_collapsed_below_tablet_width() {
        assert!(sidebar_default_collapsed_for_width(390.0));
        assert!(sidebar_default_collapsed_for_width(767.0));
        assert!(!sidebar_default_collapsed_for_width(768.0));
        assert!(!sidebar_default_collapsed_for_width(1200.0));
    }
}
