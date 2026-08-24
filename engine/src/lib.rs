#[cfg(all(feature = "portmaster", feature = "browser"))]
compile_error!("features `portmaster` and `browser` are mutually exclusive: pick one target flavour per build");
#[cfg(all(feature = "browser", not(target_os = "emscripten")))]
compile_error!("feature `browser` only supports --target wasm32-unknown-emscripten");

pub mod animate;
pub mod app;
pub mod main_loop;
pub mod app_info;
pub mod audio;
pub mod config;
pub mod controller;
pub mod draw;
pub mod font;
pub mod frame_rate;
pub mod game;
pub mod game_input;
pub mod high_score;
pub mod icon;
pub mod menu;
pub mod menu_input;
pub mod particles;
pub mod render;
pub mod scale;
pub mod session;
pub mod storage;
