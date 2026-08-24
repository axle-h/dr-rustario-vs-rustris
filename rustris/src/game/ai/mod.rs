//! The Rustris AI. The genetic-algorithm training half (the `ga` subcommand) is not
//! compiled for the browser; the playing agent and its trained network always are.
pub mod recording;
mod apply_inputs;
mod input_search;
pub mod input_sequence;
pub mod action_evaluator;
mod board_features;
pub mod agent;
mod headless_game;
#[cfg(not(target_os = "emscripten"))]
pub mod genetic;
mod game_result;
#[cfg(not(target_os = "emscripten"))]
mod mutation;
#[cfg(not(target_os = "emscripten"))]
mod generation_stats;
mod coefficient;
mod neural;
#[cfg(not(target_os = "emscripten"))]
pub mod harness;
mod genome;
pub mod linear;
#[cfg(not(target_os = "emscripten"))]
mod generation_record;
#[cfg(not(target_os = "emscripten"))]
mod organism;
#[cfg(not(target_os = "emscripten"))]
mod objective;

