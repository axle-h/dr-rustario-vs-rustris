//! The Android entry point: SDL's Java activity loads `liblauncher.so` and calls `SDL_main`.

use std::ffi::{c_char, c_int};

#[no_mangle]
pub extern "C" fn SDL_main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    // stderr goes nowhere on Android; SDL_Log reaches logcat under the tag "SDL/APP"
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
    // the process can outlive the activity and be handed the next launch with every static
    // still set, including a dead audio channel, so exit for a clean start
    std::process::exit(code)
}
