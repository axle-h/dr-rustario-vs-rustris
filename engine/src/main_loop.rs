//! Drives the per-frame tick: a plain loop on desktop, the browser's animation-frame callback
//! under emscripten. The app never blocks, so both drivers run the same code.

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

/// Runs `tick` once per animation frame and never returns, since emscripten unwinds `main`.
/// Exit or an error calls `emscripten_force_exit`, which fires the page's `Module.onExit`.
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

    // leaked, since it must outlive `main`
    let holder: *mut Holder<S> = Box::into_raw(Box::new((state, Box::new(tick))));
    unsafe {
        // fps 0 = requestAnimationFrame; simulate_infinite_loop = 1 never returns here
        emscripten_set_main_loop_arg(trampoline::<S>, holder as *mut c_void, 0, 1);
    }
    unreachable!("emscripten_set_main_loop_arg(simulate_infinite_loop = 1) does not return")
}
