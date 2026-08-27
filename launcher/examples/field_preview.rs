//! renders the modern themes' background particle field to a png, for eyeballing it without
//! playing a match. `cargo run --example field_preview -- <seconds> <out.png> [players]`

use engine::config::{Config, ParticleDensity};
use engine::font::FontType;
use engine::game::GameId;
use engine::particles::field::context::{Palette, PlayerRegion, SceneContext};
use engine::particles::field::director::Feature;
use engine::particles::field::reaction::{words, FieldEvent};
use engine::particles::field::shapes::{ShapeBank, Verdict};
use engine::particles::geometry::RectF;
use engine::particles::particle::Particle;
use engine::particles::render::ParticleRender;
use engine::particles::scale::Scale as ParticleScale;
use engine::particles::Particles;
use engine::render::font::FontRender;
use engine::render::Theme;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::time::Duration;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let shapes_only = args.get(1).map(|s| s == "shapes").unwrap_or(false);
    let sheet = args.get(1).map(|s| s == "sheet").unwrap_or(false);
    // "features": one png per feature routine, taken part way through its hold
    let features = args.get(1).map(|s| s == "features").unwrap_or(false);
    let times: Vec<f64> = args
        .get(1)
        .filter(|_| !shapes_only && !sheet && !features)
        .cloned()
        .unwrap_or("4".to_string())
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let prefix = args.get(2).cloned().unwrap_or("field".to_string());
    let diagnostic = shapes_only || sheet || features;
    // the features mode takes it too: a half-window canvas is where a routine authored in
    // canvas coordinates goes wrong
    let both_modern: bool = args
        .get(3)
        .filter(|_| !shapes_only && !sheet)
        .map(|s| s.parse().unwrap())
        .unwrap_or(true);
    let events: bool = args
        .get(4)
        .filter(|_| !diagnostic)
        .map(|s| s.parse().unwrap())
        .unwrap_or(true);
    // "dr", "rustris" or "mixed": which game each player is on
    let games = args.get(5).cloned().unwrap_or("mixed".to_string());

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("field-preview", WIDTH, HEIGHT)
        .position_centered()
        .hidden()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .target_texture()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let config = Config::default();

    let mut all = vec![];
    let mut games_built: Vec<PreviewGame> = vec![];
    for (name, id, palette) in PREVIEW_GAMES {
        let start = all.len();
        all.extend(match name {
            "dr" => dr_rustario::theme::all_themes(&mut canvas, &texture_creator, config)?,
            "puyo" => puyo_rusto::theme::all_themes(&mut canvas, &texture_creator, config)?,
            _ => rustris::theme::all_themes(&mut canvas, &texture_creator, config)?,
        });
        games_built.push(PreviewGame {
            name,
            id,
            palette: palette(),
            themes: start..all.len(),
            // the modern theme is the last of each game's set
            modern: all.len() - 1,
        });
    }
    let games_built = games_built;
    // which game a theme in the shared list belongs to
    let game_of = |index: usize| {
        games_built
            .iter()
            .find(|g| g.themes.contains(&index))
            .map(|g| g.name)
            .unwrap_or("?")
    };

    if sheet {
        // every sprite of every theme, outlined and labelled with what the bank made of it,
        // in one png: the only way to eyeball the edge detection itself
        let out = args.get(2).cloned().unwrap_or("sheet.png".to_string());
        let font = FontRender::from_font(
            &mut canvas,
            &texture_creator,
            FontType::Bold,
            14,
            Color::RGB(0xC0, 0xC0, 0xC0),
        )?;
        const COLS: usize = 6;
        const CELL: u32 = 200;
        const LABEL: u32 = 22;
        const HEADER: u32 = 30;
        // outline everything first, so the page can be sized before it is drawn
        let mut pages = vec![];
        for (index, theme) in all.iter().enumerate() {
            let game = game_of(index);
            let mut flat = theme.sprites().flatten(&mut canvas, &texture_creator)?;
            let audited = ShapeBank::audit(&mut canvas, &mut flat, ParticleDensity::High)?;
            let used = audited
                .iter()
                .filter(|a| a.verdict == Verdict::Used)
                .count();
            println!(
                "{game:>7} {:>8}: {} sprites, {used} used, {} duplicates, {} too few",
                theme.name(),
                audited.len(),
                audited
                    .iter()
                    .filter(|a| a.verdict == Verdict::Duplicate)
                    .count(),
                audited
                    .iter()
                    .filter(|a| a.verdict == Verdict::TooFew)
                    .count(),
            );
            pages.push((
                format!(
                    "{game} {}: {} sprites, {used} distinct outlines",
                    theme.name(),
                    audited.len()
                ),
                audited,
            ));
        }
        let width = COLS as u32 * CELL;
        let height: u32 = pages
            .iter()
            .map(|(_, audited)| HEADER + audited.len().div_ceil(COLS) as u32 * (CELL + LABEL))
            .sum();
        let mut target = texture_creator
            .create_texture_target(PixelFormatEnum::RGBA8888, width, height)
            .map_err(|e| e.to_string())?;
        let mut pixels = vec![];
        canvas
            .with_texture_canvas(&mut target, |c| {
                c.set_draw_color(Color::RGB(0x10, 0x10, 0x14));
                c.clear();
                let mut top = 0i32;
                for (title, audited) in pages.iter() {
                    c.set_draw_color(Color::RGB(0x20, 0x20, 0x30));
                    c.fill_rect(Rect::new(0, top, width, HEADER)).unwrap();
                    font.render_string(c, sdl2::rect::Point::new(8, top + 8), title)
                        .unwrap();
                    for (i, audit) in audited.iter().enumerate() {
                        let left = (i % COLS) as i32 * CELL as i32;
                        let cell_top =
                            top + HEADER as i32 + (i / COLS) as i32 * (CELL + LABEL) as i32;
                        c.set_draw_color(Color::RGB(0x22, 0x22, 0x2A));
                        c.draw_rect(Rect::new(left + 2, cell_top + 2, CELL - 4, CELL - 4))
                            .unwrap();
                        c.set_draw_color(match audit.verdict {
                            Verdict::Used => Color::RGB(0x40, 0xE0, 0xFF),
                            Verdict::Duplicate => Color::RGB(0x70, 0x70, 0x78),
                            Verdict::TooFew => Color::RGB(0xE0, 0x50, 0x40),
                        });
                        let (cx, cy) = (left + CELL as i32 / 2, cell_top + CELL as i32 / 2);
                        for point in audit.shape.points() {
                            let x = cx + (point.x() * CELL as f64 * 0.8).round() as i32;
                            let y = cy + (point.y() * CELL as f64 * 0.8).round() as i32;
                            c.fill_rect(Rect::new(x - 1, y - 1, 3, 3)).unwrap();
                        }
                        let label = format!(
                            "{} {} {} pts",
                            audit.label,
                            audit.verdict.label(),
                            audit.shape.len()
                        );
                        font.render_string(
                            c,
                            sdl2::rect::Point::new(left + 6, cell_top + CELL as i32),
                            &label,
                        )
                        .unwrap();
                    }
                    top +=
                        HEADER as i32 + audited.len().div_ceil(COLS) as i32 * (CELL + LABEL) as i32;
                }
                pixels = c
                    .read_pixels(Rect::new(0, 0, width, height), PixelFormatEnum::ABGR8888)
                    .unwrap();
            })
            .map_err(|e| e.to_string())?;
        image::save_buffer(&out, &pixels, width, height, image::ColorType::Rgba8)
            .map_err(|e| e.to_string())?;
        println!("wrote {out}: {width}x{height}");
        return Ok(());
    }

    if shapes_only {
        let mut sheets = vec![];
        for theme in all.iter() {
            sheets.push(theme.sprites().flatten(&mut canvas, &texture_creator)?);
        }
        let mut bank = ShapeBank::new(ParticleDensity::High);
        let indices = (0..all.len()).collect::<Vec<usize>>();
        // build is budgeted per frame, so keep asking until nothing changes
        for _ in 0..200 {
            bank.build(&mut canvas, &mut sheets, &indices)?;
        }
        for (index, theme) in all.iter().enumerate() {
            let game = game_of(index);
            let shapes = bank.shapes(index);
            let points = shapes.iter().map(|s| s.len()).collect::<Vec<usize>>();
            println!(
                "{game:>7} theme {index} {:>9}: {} shapes, points {points:?}",
                theme.name(),
                shapes.len()
            );
        }
        return Ok(());
    }

    let scale = ParticleScale::new((WIDTH, HEIGHT));
    let mut particles = ParticleRender::new(
        &mut canvas,
        Particles::new(100_000),
        &texture_creator,
        scale,
        all.iter().collect::<Vec<&Theme>>(),
        ParticleDensity::High,
    )?;

    // one game named puts both players on it; anything else deals them the first two, which
    // is what "mixed" means whatever the compendium grows to
    let named = games_built.iter().find(|g| g.name == games);
    let left = named.unwrap_or(&games_built[0]);
    let right = named.unwrap_or(&games_built[1 % games_built.len()]);
    let regions = vec![
        region(
            0,
            RectF::new(0.0, 0.0, 0.5, 1.0),
            left.modern,
            left.id,
            true,
            left.palette.clone(),
        ),
        region(
            1,
            RectF::new(0.5, 0.0, 0.5, 1.0),
            right.modern,
            right.id,
            both_modern,
            right.palette.clone(),
        ),
    ];
    // what the match screen offers: the games in play, the level, and VS in a 2-player match
    let scene = SceneContext::with_captions(
        regions,
        [left.title().to_string(), right.title().to_string()]
            .into_iter()
            .chain(["LEVEL 8".to_string(), "VS".to_string()])
            .collect(),
    )
    .unwrap();
    particles.add_field(scene.canvas, (WIDTH, HEIGHT));

    let frame = Duration::from_micros(16_667);

    if features {
        // every feature routine in turn: settle, stage it, and snapshot part way through its
        // hold, which is the only way to see one that comes up once a minute
        for feature in [
            Feature::Sprite,
            Feature::Lattice,
            Feature::Oscilloscope,
            Feature::Weather,
            Feature::Haloes,
            Feature::Spiral,
            Feature::Lissajous,
            Feature::Wells,
            Feature::Text,
        ] {
            for _ in 0..90 {
                particles.update_scene(frame, &mut canvas, &scene)?;
            }
            particles.force_field_feature(feature);
            // past the gather and into the hold
            for _ in 0..170 {
                particles.update_scene(frame, &mut canvas, &scene)?;
            }
            let targeted = particles
                .pools()
                .iter()
                .flat_map(|pool| pool.particles())
                .filter(|p| p.target().is_some())
                .count();
            let total = particles
                .pools()
                .iter()
                .map(|p| p.particles().len())
                .sum::<usize>();
            println!("{feature:?}: {targeted} of {total} on targets");
            let out = format!("{prefix}-{}.png", format!("{feature:?}").to_lowercase());
            snapshot(&mut canvas, &texture_creator, &mut particles, &out)?;
            println!("wrote {out}");
        }
        // and the words the match calls for, which no amount of waiting would produce
        for word in words::ALL {
            for _ in 0..90 {
                particles.update_scene(frame, &mut canvas, &scene)?;
            }
            particles.push_field_event(FieldEvent::Spell { word });
            for _ in 0..170 {
                particles.update_scene(frame, &mut canvas, &scene)?;
            }
            let out = format!(
                "{prefix}-word-{}.png",
                word.to_lowercase().replace(' ', "-")
            );
            snapshot(&mut canvas, &texture_creator, &mut particles, &out)?;
            println!("wrote {out}");
        }
        return Ok(());
    }

    let mut next = 0;
    let last = *times.last().unwrap();
    let frames = (last / frame.as_secs_f64()).round() as u32;
    for i in 0..=frames {
        while next < times.len() && i as f64 * frame.as_secs_f64() >= times[next] {
            let out = format!("{prefix}-{}.png", times[next]);
            report(&particles, &scene);
            snapshot(&mut canvas, &texture_creator, &mut particles, &out)?;
            next += 1;
        }
        // something to react to, so the reactions are exercised too
        if events && i == 30 {
            particles.push_field_event(FieldEvent::Clear {
                player: 0,
                rows: vec![RectF::new(0.1, 0.55, 0.25, 0.03)],
                class: 3,
                count: 4,
                is_combo: false,
            });
        }
        if events && i == 90 {
            particles.push_field_event(FieldEvent::Attack {
                from: 0,
                to: 1,
                strength: 4,
            });
        }
        particles.update_scene(frame, &mut canvas, &scene)?;
    }

    Ok(())
}

fn report(particles: &ParticleRender, scene: &SceneContext) {
    for pool in particles.pools() {
        let all = pool.particles();
        let inside = all
            .iter()
            .filter(|p| scene.canvas.contains(p.position()))
            .count();
        let targeted = all.iter().filter(|p| p.target().is_some()).count();
        let margin = all
            .iter()
            .filter(|p| !scene.canvas.contains(p.position()))
            .filter(|p| {
                let q = p.position();
                q.x() > -0.1 && q.x() < 1.1 && q.y() > -0.1 && q.y() < 1.1
            })
            .count();
        let speed = all.iter().map(|p| p.velocity().magnitude()).sum::<f64>() / all.len() as f64;
        let far = all.len() - inside - margin;

        let boards = scene
            .players
            .iter()
            .filter(|r| r.in_canvas)
            .map(|r| r.board)
            .collect::<Vec<_>>();
        let over_board = |p: &&Particle| boards.iter().any(|b| b.contains(p.position()));
        let mean_alpha = |ps: Vec<&Particle>| {
            if ps.is_empty() {
                0.0
            } else {
                ps.iter().map(|p| p.alpha()).sum::<f64>() / ps.len() as f64
            }
        };
        let on_board = all.iter().filter(over_board).collect::<Vec<&Particle>>();
        let elsewhere = all
            .iter()
            .filter(|p| !over_board(p))
            .collect::<Vec<&Particle>>();
        eprintln!(
            "  {} particles, {inside} inside, {margin} in the margin, {far} far out, \
             {targeted} on targets, {} links, mean speed {speed:.3}",
            all.len(),
            pool.links().len(),
        );
        eprintln!(
            "    {} over a board at mean alpha {:.3}, {} elsewhere at {:.3}",
            on_board.len(),
            mean_alpha(on_board.clone()),
            elsewhere.len(),
            mean_alpha(elsewhere),
        );
    }
}

fn snapshot(
    canvas: &mut sdl2::render::WindowCanvas,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    particles: &mut ParticleRender,
    out: &str,
) -> Result<(), String> {
    let mut target = texture_creator
        .create_texture_target(PixelFormatEnum::RGBA8888, WIDTH, HEIGHT)
        .map_err(|e| e.to_string())?;
    let mut result = Ok(());
    let mut pixels = vec![];
    canvas
        .with_texture_canvas(&mut target, |c| {
            c.set_draw_color(Color::BLACK);
            c.clear();
            result = particles.draw(c);
            pixels = c
                .read_pixels(Rect::new(0, 0, WIDTH, HEIGHT), PixelFormatEnum::ABGR8888)
                .unwrap();
        })
        .map_err(|e| e.to_string())?;
    result?;
    image::save_buffer(out, &pixels, WIDTH, HEIGHT, image::ColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}

fn region(
    player: u32,
    clip: RectF,
    theme: usize,
    game: GameId,
    in_canvas: bool,
    palette: Palette,
) -> PlayerRegion {
    PlayerRegion {
        player,
        clip,
        board: RectF::new(
            clip.x() + clip.width() * 0.28,
            0.18,
            clip.width() * 0.34,
            0.62,
        ),
        theme,
        game,
        palette,
        in_canvas,
        danger: 0.0,
        speed_index: 3,
        held_up: false,
    }
}

/// Every game a preview can put a player on: its short name (what the `games` argument
/// takes), its [`GameId`] and the colours it radiates into the field. One entry per game.
const PREVIEW_GAMES: [(&str, GameId, fn() -> Palette); 3] = [
    ("dr", engine::game::ids::DR_RUSTARIO, dr_palette),
    ("rustris", engine::game::ids::RUSTRIS, rustris_palette),
    ("puyo", engine::game::ids::PUYO, puyo_palette),
];

/// one of [`PREVIEW_GAMES`], with its themes located in the shared list
struct PreviewGame {
    name: &'static str,
    id: GameId,
    palette: Palette,
    themes: std::ops::Range<usize>,
    /// the particle theme: the last of the game's own set
    modern: usize,
}

impl PreviewGame {
    /// what the field spells out for this game
    fn title(&self) -> &'static str {
        match self.name {
            "dr" => "DR. RUSTARIO",
            _ => "RUSTRIS",
        }
    }
}

fn dr_palette() -> Palette {
    Palette::from_sdl(&[
        Color::RGB(0xE8, 0x06, 0x06),
        Color::RGB(0x2E, 0x7B, 0xD6),
        Color::RGB(0xE1, 0xBE, 0x00),
    ])
}

fn puyo_palette() -> Palette {
    Palette::from_sdl(&[
        Color::RGB(0xF0, 0x44, 0x2E),
        Color::RGB(0x3C, 0xC6, 0x3C),
        Color::RGB(0x2E, 0x6B, 0xF0),
        Color::RGB(0xF0, 0xC4, 0x1E),
        Color::RGB(0xB8, 0x45, 0xE0),
    ])
}

fn rustris_palette() -> Palette {
    Palette::from_sdl(&[
        Color::RGB(0x00, 0xD7, 0xFA),
        Color::RGB(0x2C, 0x00, 0xFA),
        Color::RGB(0xFF, 0x6C, 0x16),
        Color::RGB(0xFF, 0xCE, 0x13),
        Color::RGB(0x68, 0xEF, 0x13),
        Color::RGB(0xAD, 0x00, 0xFA),
        Color::RGB(0xFF, 0x1A, 0x45),
    ])
}
