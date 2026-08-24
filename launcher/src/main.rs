#![windows_subsystem = "windows"]

mod games;
mod modes;
mod shell;

use crate::shell::Shell;

mod build_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() -> Result<(), String> {
    // the ga training subcommand is not compiled for the browser
    #[cfg(not(target_os = "emscripten"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("ga") {
            use rustris::game::ai::{genetic, harness};
            return match args.get(1).map(String::as_str) {
                None | Some("auto") => genetic::ga_main_auto(),
                Some("survival") => genetic::ga_main_survival(),
                Some("score") => genetic::ga_main_score(),
                Some("diagnose") => genetic::ga_diagnose(),
                Some("play") => harness::harness_main(&args[2..]),
                Some(other) => Err(format!(
                    "unknown ga mode '{}', expected: auto, survival, score, diagnose or play",
                    other
                )),
            };
        }
    }

    engine::app_info::init(engine::app_info::AppInfo {
        name: build_info::PKG_NAME,
        version: build_info::PKG_VERSION,
        authors: build_info::PKG_AUTHORS,
    });

    engine::main_loop::run(Shell::new()?, Shell::tick)
}
