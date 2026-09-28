//! Durable storage: the browser's writes land in memory and are flushed to IndexedDB, the
//! IDBFS mount set up by `web/index.html`.

/// Call after saving anything that must survive the session; a no-op but in the browser.
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
