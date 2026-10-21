//! Private preview gate for the Ironwood Companion add-on (not pop-out windows).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodCompanionFeatureEnabled)]
    fn browser_enabled() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodSetCompanionFeatureEnabled)]
    fn browser_set_enabled(value: bool);
}
pub(crate) fn enabled() -> bool {
    #[cfg(target_arch = "wasm32")]
    { browser_enabled() }
    #[cfg(not(target_arch = "wasm32"))]
    { false }
}
pub(crate) fn set_enabled(value: bool) {
    #[cfg(target_arch = "wasm32")]
    browser_set_enabled(value);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = value;
}
