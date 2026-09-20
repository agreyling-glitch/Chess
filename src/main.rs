#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    battle_chess::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
