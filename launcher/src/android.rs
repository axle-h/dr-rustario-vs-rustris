//! The Android entry point. SDL's Java activity loads `liblauncher.so` (see
//! `android/app/src/main/java/.../MainActivity.java`) and calls `SDL_main` on its own thread.

use std::ffi::{c_char, c_int};

#[no_mangle]
pub extern "C" fn SDL_main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    // nothing reads stdout or stderr on Android, so a panic would leave no trace; SDL_Log is
    // logcat, under the tag "SDL/APP"
    std::panic::set_hook(Box::new(|info| sdl2::log::log_error(&info.to_string())));

    let code = match crate::run() {
        Ok(()) => 0,
        Err(e) => {
            sdl2::log::log_error(&e);
            let _ = sdl2::messagebox::show_simple_message_box(
                sdl2::messagebox::MessageBoxFlag::ERROR,
                "Dr. Rustario vs. Rustris",
                &e,
                None,
            );
            1
        }
    };
    // SDL's activity finishes itself when this returns but the process can live on and be
    // handed the next launch, which would find every static already set - the audio thread's
    // command channel among them, with nothing left on the far end. A fresh process is the
    // only clean start.
    std::process::exit(code)
}
