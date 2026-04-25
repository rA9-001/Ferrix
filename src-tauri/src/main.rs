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
        /// Set an env var only if the user hasn't set it themselves.
        fn set_default(key: &str, value: &str) {
            if std::env::var_os(key).is_none() {
                // Safe: single-threaded, runs before any other code touches env.
                unsafe {
                    std::env::set_var(key, value);
                }
            }
        }

        // GPU compositing — required for the WebKit frame clock to drive
        // the renderer at the monitor's refresh rate instead of the
        // 60Hz CPU paint timer.
        set_default("WEBKIT_FORCE_COMPOSITING_MODE", "1");

        // Defensive: make sure the blank-window workaround is not active —
        // it disables GPU compositing entirely and tanks scroll perf.
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").as_deref() == Ok("1") {
            unsafe {
                std::env::remove_var("WEBKIT_DISABLE_COMPOSITING_MODE");
            }
        }

        // Triple-buffer present on Wayland so the frame clock can hit
        // 144 Hz instead of stalling on FIFO queue depth.
        set_default("MESA_VK_WSI_PRESENT_MODE", "mailbox");

        // Mesa swap interval: 0 disables vblank wait so the WebKit frame
        // clock — not GLX — decides the cadence. The compositor will
        // still vsync the final present, so no tearing on Wayland and
        // very rarely on a modern X11 compositor.
        set_default("vblank_mode", "0");

        // NVIDIA proprietary driver: cap pre-rendered frames so input
        // and scroll latency match the panel refresh.
        set_default("__GL_MaxFramesAllowed", "1");
        // NVIDIA: tear into the compositor's vblank, not GLX's.
        set_default("__GL_SYNC_TO_VBLANK", "0");

        // GTK on Wayland respects per-monitor refresh; on X11 with a
        // compositor that forces 60Hz (picom default, some KDE setups)
        // GDK_BACKEND=x11 still works but tops out at 60. Leaving the
        // user's choice intact — we don't override XDG_SESSION_TYPE.
    }


    tauri_app_lib::run()
}
