// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // ----------------------------------------------------------------
    // Linux / WebKitGTK rendering tuning.
    //
    // In packaged builds (AppImage / .deb / .rpm) WebKitGTK does not
    // always pick the fastest pipeline by default — many distros ship it
    // with accelerated compositing only enabled lazily, which leads to
    // sub-60fps scrolling and visible jank in long lists.
    //
    // The flags below opt in to the fast paths that work on every modern
    // Mesa / NVIDIA / Intel stack:
    //
    //   * WEBKIT_FORCE_COMPOSITING_MODE=1
    //       Always run the accelerated compositor, even when the page
    //       has no triggers that would normally enable it. Scrolling is
    //       then composited on the GPU instead of the CPU.
    //   * WEBKIT_DISABLE_COMPOSITING_MODE   — must NOT be set, it is the
    //       NVIDIA blank-window workaround and forces software paint.
    //
    // Users can still override any of these from their environment.
    // ----------------------------------------------------------------
    #[cfg(target_os = "linux")]
    {
        // Safe: single-threaded, runs before any other code touches env.
        if std::env::var_os("WEBKIT_FORCE_COMPOSITING_MODE").is_none() {
            unsafe {
                std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
            }
        }
        // Defensive: make sure the blank-window workaround is not active —
        // it disables GPU compositing entirely and tanks scroll perf.
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").as_deref() == Ok("1") {
            unsafe {
                std::env::remove_var("WEBKIT_DISABLE_COMPOSITING_MODE");
            }
        }
    }

    tauri_app_lib::run()
}
