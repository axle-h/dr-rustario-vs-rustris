//! The whole game: `main.rs` calls [`run`], and on Android `android.rs` calls it from
//! `SDL_main`.

#[cfg(target_os = "android")]
mod android;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod cross;
mod games;
mod modes;
mod shell;

use crate::shell::Shell;

mod build_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Runs the game, or with `ga` first the ai tools: `ga dr ...`, `ga puyo rank|play|duel`,
/// `ga cross`, and anything else is Rustris (`ga auto|play|...`). Desktop builds only.
pub fn run() -> Result<(), String> {
    #[cfg(not(any(target_os = "emscripten", target_os = "android")))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("ga") {
            if args.get(1).map(String::as_str) == Some("dr") {
                use dr_rustario::game::ai::{
                    align, duel, explain, genetic, harness, passes, probe,
                };
                return match args.get(2).map(String::as_str) {
                    None | Some("auto") => genetic::ga_main_auto(),
                    Some("pretrain") => genetic::ga_main_pretrain(&args[3..]),
                    Some("screen") => genetic::ga_main_screen(&args[3..]),
                    Some("survive") => genetic::ga_main_survive(),
                    Some("tune") => genetic::ga_main_tune(&args[3..]),
                    Some("diagnose") => genetic::ga_diagnose(),
                    Some("play") => harness::harness_main(&args[3..]),
                    Some("trial") => genetic::ga_main_trial(&args[3..]),
                    Some("garbage") => genetic::ga_main_garbage(&args[3..]),
                    Some("passes") => passes::ga_main_passes(&args[3..]),
                    Some("align") => align::ga_main_align(&args[3..]),
                    Some("compare") => genetic::ga_main_compare(&args[3..]),
                    Some("duel") => duel::ga_main_duel(&args[3..]),
                    Some("probe") => probe::probe_main(&args[3..]),
                    Some("explain") => explain::explain_main(&args[3..]),
                    Some(other) => Err(format!(
                        "unknown ga dr mode '{}', expected: auto, pretrain, screen, survive, tune, \
                         trial, garbage, passes, align, compare, duel, diagnose, play, probe or explain",
                        other
                    )),
                };
            }

            if args.get(1).map(String::as_str) == Some("puyo") {
                use puyo_rusto::game::ai::harness;
                return match args.get(2).map(String::as_str) {
                    None | Some("rank") => harness::rank_main(&args[3..]),
                    Some("play") => harness::harness_main(&args[3..]),
                    Some("duel") => harness::duel_main(&args[3..]),
                    Some(other) => Err(format!(
                        "unknown ga puyo mode '{}', expected: play, rank or duel",
                        other
                    )),
                };
            }

            if args.get(1).map(String::as_str) == Some("cross") {
                return crate::cross::cross_main(&args[2..]);
            }

            use rustris::game::ai::{genetic, harness};
            return match args.get(1).map(String::as_str) {
                None | Some("auto") => genetic::ga_main_auto(),
                Some("survival") => genetic::ga_main_survival(),
                Some("score") => genetic::ga_main_score(),
                Some("diagnose") => genetic::ga_diagnose(),
                Some("play") => harness::harness_main(&args[2..]),
                Some(other) => Err(format!(
                    "unknown ga mode '{}', expected: dr, puyo, cross, auto, survival, score, \
                     diagnose or play",
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
