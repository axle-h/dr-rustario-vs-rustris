use crate::game::bottle::{BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::pill::VirusColor;
use crate::theme::data::{
    animations, audio, cells, mascot, previews, spawn_cell, strip, ColorLayout, Sounds, HUD_MAX,
};
use engine::animate::frames::FrameAnimationType;
use engine::animate::mascot::MascotAnimationTypes;
use engine::config::Config;
use engine::game::CellId;
use engine::render::animation::AnimationSpriteSheetData;
use engine::render::modern::{modern_theme, ModernThemeOptions};
use engine::render::scene::ClearParticles;
use engine::render::sprite_sheet::{BlockSpriteSheetData, CellAnimationData, GhostStyle};
use engine::render::Theme;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

pub mod sprites {
    // vitamins
    pub const VITAMINS: &[u8] = include_bytes!("vitamins.png");
    pub const SRC_BLOCK_SIZE: u32 = 100; // TODO I have this upto 400 does it look any nicer?

    // viruses
    pub const VIRUS_RED_IDLE: &[u8] = include_bytes!("viruses/r.png");
    pub const VIRUS_BLUE_IDLE: &[u8] = include_bytes!("viruses/b.png");
    pub const VIRUS_YELLOW_IDLE: &[u8] = include_bytes!("viruses/y.png");

    // dr
    pub const DR_THROW: &[u8] = include_bytes!("dr/throw.png");
    pub const DR_IDLE: &[u8] = include_bytes!("dr/idle.png");
    pub const DR_GAME_OVER: &[u8] = include_bytes!("dr/game-over.png");
    pub const DR_VICTORY: &[u8] = include_bytes!("dr/victory.png");
    pub const DR_FPS: u32 = 60;
}

mod sound {
    pub const DESTROY_VIRUS: &[u8] = include_bytes!("destroy-virus.ogg");
    pub const DESTROY_VIRUS_COMBO: &[u8] = include_bytes!("destroy-virus-combo.ogg");
    pub const DESTROY_VITAMIN: &[u8] = include_bytes!("destroy-vitamin.ogg");
    pub const DESTROY_VITAMIN_COMBO: &[u8] = include_bytes!("destroy-vitamin-combo.ogg");
    pub const DROP: &[u8] = include_bytes!("drop.ogg");
    pub const FEVER_INTRO: &[u8] = include_bytes!("fever-intro.ogg");
    pub const FEVER_REPEAT: &[u8] = include_bytes!("fever-repeat.ogg");
    pub const FEVER_NEXT_LEVEL_INTRO: &[u8] = include_bytes!("fever-next-level-intro.ogg");
    pub const FEVER_NEXT_LEVEL_REPEAT: &[u8] = include_bytes!("fever-next-level-repeat.ogg");
    pub const GAME_OVER_INTRO: &[u8] = include_bytes!("game-over-intro.ogg");
    pub const GAME_OVER_REPEAT: &[u8] = include_bytes!("game-over-repeat.ogg");
    pub const RECEIVE_GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const HARD_DROP: &[u8] = include_bytes!("hard-drop.ogg");
    pub const MOVE_PILL: &[u8] = include_bytes!("move.ogg");
    pub const NEXT_LEVEL_JINGLE: &[u8] = include_bytes!("next-level-jingle.ogg");
    pub const PAUSE: &[u8] = include_bytes!("pause.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    pub const SPEED_LEVEL_UP: &[u8] = include_bytes!("speed-level-up.ogg");
    pub const VICTORY: &[u8] = include_bytes!("victory.ogg");
}

/// what the modern theme radiates into the background particle field: the three vitamin
/// colours. The blue is lifted off the one in the art, which is a navy that all but vanishes
/// as a particle over a black background
const VITAMIN_PALETTE: [Color; 3] = [
    Color::RGB(0xE8, 0x06, 0x06), // red
    Color::RGB(0x2E, 0x7B, 0xD6), // blue
    Color::RGB(0xE1, 0xBE, 0x00), // golden yellow
];

const DR_SCALE_OF_BLOCK: f64 = 6.5;

fn block(i: i32, j: i32) -> Point {
    Point::new(
        i * sprites::SRC_BLOCK_SIZE as i32,
        j * sprites::SRC_BLOCK_SIZE as i32,
    )
}

fn pill(i: i32, j: i32) -> Point {
    Point::new(
        2 * i * sprites::SRC_BLOCK_SIZE as i32,
        j * sprites::SRC_BLOCK_SIZE as i32,
    )
}

fn layout(north_i: i32, east_j: i32, garbage_i: i32) -> ColorLayout {
    ColorLayout {
        north: [block(north_i, 0), block(north_i + 1, 0)],
        east: [block(6, east_j), block(6, east_j + 1)],
        south: [block(north_i + 1, 0), block(north_i, 0)],
        west: [block(6, east_j + 1), block(6, east_j)],
        garbage: block(garbage_i, 3),
    }
}

fn color_animations(
    color: VirusColor,
    virus_idle: &'static [u8],
    garbage_i: i32,
) -> Vec<(Vec<CellId>, CellAnimationData)> {
    animations(
        color,
        AnimationSpriteSheetData::exclusive_square_linear(virus_idle),
        AnimationSpriteSheetData::static_first_square_frame(virus_idle),
        strip(
            sprites::VITAMINS,
            block(garbage_i, 3),
            1,
            sprites::SRC_BLOCK_SIZE,
        ),
    )
}

pub fn modern_dr_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    block_size: u32,
) -> Result<Theme<'a>, String> {
    let mascot_types = MascotAnimationTypes {
        idle: FrameAnimationType::Linear {
            fps: sprites::DR_FPS,
        },
        spawn: FrameAnimationType::Linear {
            fps: sprites::DR_FPS,
        },
        victory: FrameAnimationType::LinearWithPause {
            fps: sprites::DR_FPS,
            pause_for: Duration::from_secs(3),
            resume_from_frame: 98,
        },
        game_over: FrameAnimationType::LinearWithPause {
            fps: sprites::DR_FPS,
            pause_for: Duration::from_secs(3),
            resume_from_frame: 195,
        },
    };
    let options = ModernThemeOptions {
        name: "particle",
        sprites: BlockSpriteSheetData {
            file: sprites::VITAMINS,
            source_block_size: sprites::SRC_BLOCK_SIZE,
            cells: cells(
                sprites::SRC_BLOCK_SIZE,
                [
                    (VirusColor::Yellow, layout(4, 4, 2)),
                    (VirusColor::Red, layout(2, 2, 1)),
                    (VirusColor::Blue, layout(0, 0, 0)),
                ],
            ),
            animations: [
                color_animations(VirusColor::Yellow, sprites::VIRUS_YELLOW_IDLE, 2),
                color_animations(VirusColor::Red, sprites::VIRUS_RED_IDLE, 1),
                color_animations(VirusColor::Blue, sprites::VIRUS_BLUE_IDLE, 0),
            ]
            .concat(),
            ghost_alpha: 0x70,
            // previews are rendered at twice the block size: the builder scales the sheet
            previews: previews(
                sprites::VITAMINS,
                (sprites::SRC_BLOCK_SIZE * 2, sprites::SRC_BLOCK_SIZE),
                [
                    pill(2, 0),
                    pill(2, 2),
                    pill(2, 1),
                    pill(0, 0),
                    pill(0, 1),
                    pill(0, 2),
                    pill(1, 0),
                    pill(1, 1),
                    pill(1, 2),
                ],
            ),
            // all particle dr frames are 478 wide and all except victory are 478 high, victory is 510 high
            mascot: Some(mascot(
                AnimationSpriteSheetData::exclusive_table(sprites::DR_THROW, 7, 7, 46),
                AnimationSpriteSheetData::exclusive_table(sprites::DR_GAME_OVER, 16, 15, 238),
                AnimationSpriteSheetData::exclusive_table(sprites::DR_VICTORY, 14, 14, 184),
                AnimationSpriteSheetData::exclusive_table(sprites::DR_IDLE, 12, 11, 123),
            )),
        },
        audio: audio(
            config.audio,
            Sounds {
                move_pill: sound::MOVE_PILL,
                rotate: sound::ROTATE,
                drop: sound::DROP,
                destroy_virus: sound::DESTROY_VIRUS,
                destroy_virus_combo: sound::DESTROY_VIRUS_COMBO,
                destroy_vitamin: sound::DESTROY_VITAMIN,
                destroy_vitamin_combo: sound::DESTROY_VITAMIN_COMBO,
                paused: sound::PAUSE,
                speed_level_up: sound::SPEED_LEVEL_UP,
                receive_garbage: sound::RECEIVE_GARBAGE,
                next_level_jingle: sound::NEXT_LEVEL_JINGLE,
                hard_drop: Some(sound::HARD_DROP),
            },
        )?
        .with_game_music(sound::FEVER_INTRO, sound::FEVER_REPEAT)?
        .with_game_over_music(sound::GAME_OVER_INTRO, sound::GAME_OVER_REPEAT)?
        .with_next_stage_music(
            sound::FEVER_NEXT_LEVEL_INTRO,
            sound::FEVER_NEXT_LEVEL_REPEAT,
        )?
        .with_looping_victory_music(sound::VICTORY)?,
        columns: BOTTLE_WIDTH,
        rows: BOTTLE_HEIGHT,
        visible_rows: BOTTLE_HEIGHT,
        block_size,
        // the bottle shows every row it simulates: nothing above the neck to give away
        top_buffer_rows: 0,
        metrics: HUD_MAX.to_vec(),
        metrics_left: vec![],
        mascot: Some((mascot_types, DR_SCALE_OF_BLOCK)),
        spawn_cell: spawn_cell(),
        cell_idle_type: FrameAnimationType::Linear { fps: 30 },
        queue_max: 2,
        // nothing waits: an attack lands the moment it arrives
        pending_max: 0,
        particle_color: Color::WHITE,
        particle_palette: VITAMIN_PALETTE.to_vec(),
        clear_particles: ClearParticles::Masked {
            fade_in: Duration::from_millis(250),
        },
        destroy_style: None,
        game_over_style: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
        // no art for a caption, and no captions: see `clear_popup`
        popup_sprites: None,
    };
    modern_theme(canvas, texture_creator, options)
}
