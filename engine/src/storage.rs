//! Durable-storage hooks. Native targets write straight to disk, so there is nothing to
//! do; the browser writes land on an in-memory filesystem that must be flushed to
//! IndexedDB (the IDBFS mount set up by the page, see `web/index.html`).

/// Pushes pending writes to durable storage. Call after saving anything that must
/// survive the session; a no-op everywhere but the browser.
pub fn flush() {
    #[cfg(target_os = "emscripten")]
    unsafe {
        extern "C" {
            fn emscripten_run_script(script: *const std::os::raw::c_char);
        }
        emscripten_run_script(
            c"FS.syncfs(false, function (e) { if (e) console.error('syncfs', e); });".as_ptr(),
        );
    }
}
