//! Browser availability for the public Ironwood Companion integration.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodCompanionFeatureEnabled)]
    fn browser_enabled() -> bool;
}
pub(crate) fn enabled() -> bool {
    #[cfg(target_arch = "wasm32")]
    { browser_enabled() }
    #[cfg(not(target_arch = "wasm32"))]
    { false }
}
