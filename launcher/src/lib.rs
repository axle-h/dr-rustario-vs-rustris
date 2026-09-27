//! The whole game. `main.rs` is one call into [`run`], and on Android `android.rs` is the
//! same call behind the `SDL_main` SDL's Java activity looks for - see launcher/Cargo.toml.

#[cfg(target_os = "android")]
mod android;
/// `ga cross` goes with the rest of the `ga` subcommand, which the browser and Android builds
/// have none of
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod cross;
mod games;
mod modes;
mod shell;

use crate::shell::Shell;

mod build_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Runs the game, or with `ga` as the first argument the ai training and measurement tools
/// (desktop builds only).
pub fn run() -> Result<(), String> {
    // the ga training subcommand is not compiled for the browser, nor for Android, where an app
    // is given no command line to ask for it and stdout goes nowhere
    #[cfg(not(any(target_os = "emscripten", target_os = "android")))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("ga") {
            // `ga dr ...` trains Dr. Rustario, everything else is Rustris
            if args.get(1).map(String::as_str) == Some("dr") {
                use dr_rustario::game::ai::{explain, genetic, harness, probe};
                return match args.get(2).map(String::as_str) {
                    None | Some("auto") => genetic::ga_main_auto(),
                    Some("pretrain") => genetic::ga_main_pretrain(&args[3..]),
                    Some("screen") => genetic::ga_main_screen(&args[3..]),
                    Some("survive") => genetic::ga_main_survive(),
                    Some("tune") => genetic::ga_main_tune(),
                    Some("diagnose") => genetic::ga_diagnose(),
                    Some("play") => harness::harness_main(&args[3..]),
                    Some("trial") => genetic::ga_main_trial(&args[3..]),
                    Some("probe") => probe::probe_main(&args[3..]),
                    Some("explain") => explain::explain_main(&args[3..]),
                    Some(other) => Err(format!(
                        "unknown ga dr mode '{}', expected: auto, pretrain, screen, survive, tune, \
                         trial, diagnose, play, probe or explain",
                        other
                    )),
                };
            }

            // `ga puyo ...` plays and ranks Puyo Rusto, which trains nothing: its ai is a
            // deterministic scorer and its difficulty ladder is a measurement rather than a
            // model, so there is a `rank` where the other two games have an `auto`
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

            // `ga cross` is the launcher's own: it is the only place that can see all
            // three games at once, which is what pricing an attack between them needs
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
