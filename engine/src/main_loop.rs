//! Drives the per-frame tick: a plain loop on desktop, the browser's animation-frame
//! callback under emscripten. The whole app is tick-based (no blocking loops) so both
//! drivers run exactly the same code.

pub enum LoopControl {
    Continue,
    Exit,
}

/// Runs `tick` against `state` until it returns [`LoopControl::Exit`] or an error.
#[cfg(not(target_os = "emscripten"))]
pub fn run<S: 'static>(
    mut state: S,
    mut tick: impl FnMut(&mut S) -> Result<LoopControl, String> + 'static,
) -> Result<(), String> {
    loop {
        if matches!(tick(&mut state)?, LoopControl::Exit) {
            return Ok(());
        }
    }
}

/// Runs `tick` against `state` once per animation frame. This function never returns:
/// emscripten unwinds `main` (skipping the rest of it) and calls back into the leaked
/// state, which lives for the rest of the page. [`LoopControl::Exit`] or an error cancels
/// the callbacks and exits the runtime via `emscripten_force_exit`, which detaches the
/// runtime's event listeners and fires the page's `Module.onExit` with the exit status,
/// so the page can react (e.g. offer a restart) instead of freezing on the last frame.
#[cfg(target_os = "emscripten")]
pub fn run<S: 'static>(
    state: S,
    tick: impl FnMut(&mut S) -> Result<LoopControl, String> + 'static,
) -> Result<(), String> {
    use std::os::raw::{c_int, c_void};

    extern "C" {
        fn emscripten_set_main_loop_arg(
            func: extern "C" fn(*mut c_void),
            arg: *mut c_void,
            fps: c_int,
            simulate_infinite_loop: c_int,
        );
        fn emscripten_cancel_main_loop();
        fn emscripten_force_exit(status: c_int) -> !;
    }

    type Holder<S> = (S, Box<dyn FnMut(&mut S) -> Result<LoopControl, String>>);

    extern "C" fn trampoline<S>(arg: *mut c_void) {
        let holder = unsafe { &mut *(arg as *mut Holder<S>) };
        match (holder.1)(&mut holder.0) {
            Ok(LoopControl::Continue) => {}
            Ok(LoopControl::Exit) => unsafe {
                emscripten_cancel_main_loop();
                emscripten_force_exit(0);
            },
            Err(e) => {
                eprintln!("fatal: {e}");
                unsafe {
                    emscripten_cancel_main_loop();
                    emscripten_force_exit(1);
                }
            }
        }
    }

    // the state deliberately leaks: it must outlive `main`, which emscripten unwinds
    let holder: *mut Holder<S> = Box::into_raw(Box::new((state, Box::new(tick))));
    unsafe {
        // fps 0 = requestAnimationFrame; simulate_infinite_loop = 1 never returns here
        emscripten_set_main_loop_arg(trampoline::<S>, holder as *mut c_void, 0, 1);
    }
    unreachable!("emscripten_set_main_loop_arg(simulate_infinite_loop = 1) does not return")
}
