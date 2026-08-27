//! prints where every theme of both games puts its board, for a window size and player count

use engine::config::{Config, VideoConfig, VideoMode};
use engine::render::layout::BoardLayout;
use engine::render::Theme;
use engine::scale::player_area;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let width: u32 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(1280);
    let height: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(720);
    let players: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1);
    let integer_scale: bool = args
        .get(4)
        .map(|s| s.parse().unwrap())
        .unwrap_or(Config::default().video.integer_scale);

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("scale-report", width, height)
        .position_centered()
        .hidden()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut config = Config::default();
    config.video = VideoConfig {
        mode: VideoMode::Window { width, height },
        vsync: false,
        disable_screensaver: false,
        integer_scale,
        ..Config::default().video
    };

    // every game's themes in one list, each game's slice of it recorded; one entry per game
    const GAMES: [&str; 2] = ["dr-rustario", "rustris"];
    let mut all = vec![];
    let mut ranges = vec![];
    for game in GAMES {
        let start = all.len();
        all.extend(match game {
            "dr-rustario" => dr_rustario::theme::all_themes(&mut canvas, &texture_creator, config)?,
            _ => rustris::theme::all_themes(&mut canvas, &texture_creator, config)?,
        });
        ranges.push((game, start..all.len()));
    }

    println!(
        "window {}x{} players {} integer {} area {:?}",
        width,
        height,
        players,
        integer_scale,
        player_area(0, players, (width, height))
    );
    for (game, range) in ranges {
        let group = all[range].iter().collect::<Vec<&Theme>>();
        let layout = BoardLayout::new(&group, players, (width, height), config.video);
        println!("  {game}");
        for (index, theme) in group.iter().enumerate() {
            let scale = layout.scale(index, theme);
            let board = layout.playfield(index, theme, 0);
            let (bw, bh) = theme.background_size();
            let snip = theme.playfield_snip();
            let bg_x = board.x() - (snip.x() as f64 * scale).round() as i32;
            let bg_y = board.y() - (snip.y() as f64 * scale).round() as i32;
            println!(
                "    {:8} scale={:.3} block={:2} board=({}, {}, {}, {}) bg=({}, {}, {}, {}) clipped_top={}",
                theme.name(),
                scale,
                board.width() / theme.geometry().columns(),
                board.x(),
                board.y(),
                board.width(),
                board.height(),
                bg_x,
                bg_y,
                (bw as f64 * scale).round() as u32,
                (bh as f64 * scale).round() as u32,
                (-bg_y).max(0),
            );
        }
    }
    Ok(())
}
