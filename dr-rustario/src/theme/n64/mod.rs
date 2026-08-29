use crate::game::bottle::{BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::pill::VirusColor;
use crate::theme::data::{
    animations, audio, cells, mascot, previews, retro_mascot, spawn_cell, strip, ColorLayout,
    Sounds, N64_GAME_OVER, N64_VICTORY,
};
use engine::animate::frames::FrameAnimationType;
use engine::config::Config;
use engine::game::MetricKind;
use engine::render::animation::AnimationSpriteSheetData;
use engine::render::font::{FontRenderOptions, FontThemeOptions, MetricSnips, ThemedNumeric};
use engine::render::geometry::BoardGeometry;
use engine::render::retro::{retro_theme, RetroThemeOptions};
use engine::render::scene::SceneType;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::{HoldLayout, MascotLayout, PeekLayout, Theme};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

mod sprites {
    pub const VITAMINS: &[u8] = include_bytes!("vitamins.png");
    pub const DR_THROW: &[u8] = include_bytes!("dr/throw.png");
    pub const DR_IDLE: &[u8] = include_bytes!("dr/idle.png");
    pub const DR_GAME_OVER: &[u8] = include_bytes!("dr/game-over.png");
    pub const DR_VICTORY: &[u8] = include_bytes!("dr/victory.png");
    pub const BACKGROUND: &[u8] = include_bytes!("background.png");
    pub const BOTTLES: &[u8] = include_bytes!("bottles.png");
    pub const FONT_SMALL: &[u8] = include_bytes!("font_sm.png");
    pub const FONT_LARGE: &[u8] = include_bytes!("font_lg.png");
    pub const MATCH_END: &[u8] = include_bytes!("match-end.png");
    pub const BACKGROUND_TILE: &[u8] = include_bytes!("background-tile.png");
    pub const BACKGROUND_TILE_YELLOW: &[u8] = include_bytes!("background-tile-yellow.png");
    pub const BACKGROUND_TILE_BLUE: &[u8] = include_bytes!("background-tile-blue.png");
}
mod sound {
    pub const FEVER_INTRO: &[u8] = include_bytes!("fever-intro.ogg");
    pub const FEVER_REPEAT: &[u8] = include_bytes!("fever-repeat.ogg");
    pub const FEVER_NEXT_LEVEL: &[u8] = include_bytes!("fever-next-level.ogg");
    pub const DESTROY_VIRUS: &[u8] = include_bytes!("destroy-virus.ogg");
    pub const DESTROY_VIRUS_COMBO: &[u8] = include_bytes!("destroy-virus-combo.ogg");
    pub const DESTROY_VITAMIN: &[u8] = include_bytes!("destroy-vitamin.ogg");
    pub const DESTROY_VITAMIN_COMBO: &[u8] = include_bytes!("destroy-vitamin-combo.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");
    pub const RECEIVE_GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const SPEED_LEVEL_UP: &[u8] = include_bytes!("speed-level-up.ogg");
    pub const DROP: &[u8] = include_bytes!("drop.ogg");
    pub const MOVE_PILL: &[u8] = include_bytes!("move.ogg");
    pub const PAUSE: &[u8] = include_bytes!("pause.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    pub const VICTORY_INTRO: &[u8] = include_bytes!("victory-intro.ogg");
    pub const VICTORY_REPEAT: &[u8] = include_bytes!("victory-repeat.ogg");
    pub const NEXT_LEVEL_JINGLE: &[u8] = include_bytes!("next-level-jingle.ogg");
}

pub const BLOCK_SIZE: u32 = 10;

fn block(i: i32, j: i32) -> Point {
    Point::new(i * BLOCK_SIZE as i32, j * BLOCK_SIZE as i32)
}

fn layout(j: i32) -> ColorLayout {
    ColorLayout {
        north: [block(2, j), block(3, j)],
        east: [block(0, j), block(1, j)],
        south: [block(3, j), block(2, j)],
        west: [block(1, j), block(0, j)],
        garbage: block(4, j),
    }
}

fn color_animations(
    color: VirusColor,
    j: i32,
) -> Vec<(
    Vec<engine::game::CellId>,
    engine::render::sprite_sheet::CellAnimationData,
)> {
    animations(
        color,
        strip(sprites::VITAMINS, block(7, j), 4, BLOCK_SIZE),
        strip(sprites::VITAMINS, block(11, j), 2, BLOCK_SIZE),
        strip(sprites::VITAMINS, block(5, j), 2, BLOCK_SIZE),
    )
}

pub fn n64_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let geometry = BoardGeometry::new(
        BLOCK_SIZE,
        0,
        (8, 41),
        BOTTLE_WIDTH,
        BOTTLE_HEIGHT,
        BOTTLE_HEIGHT,
    );
    // the bottle room is tiled in a different colour per speed band, as the n64 does per mode
    let tile = |texture| SceneType::Tile { texture };
    let options = RetroThemeOptions {
        name: "n64",
        scenes: vec![
            tile(sprites::BACKGROUND_TILE),
            tile(sprites::BACKGROUND_TILE_YELLOW),
            tile(sprites::BACKGROUND_TILE_BLUE),
        ],
        sprites: BlockSpriteSheetData {
            file: sprites::VITAMINS,
            source_block_size: BLOCK_SIZE,
            cells: cells(
                BLOCK_SIZE,
                [
                    (VirusColor::Yellow, layout(0)),
                    (VirusColor::Red, layout(2)),
                    (VirusColor::Blue, layout(1)),
                ],
            ),
            animations: [
                color_animations(VirusColor::Yellow, 0),
                color_animations(VirusColor::Red, 2),
                color_animations(VirusColor::Blue, 1),
            ]
            .concat(),
            ghost_alpha: 0x50,
            previews: previews(
                sprites::VITAMINS,
                (BLOCK_SIZE * 2, BLOCK_SIZE),
                [
                    block(13, 0),
                    block(15, 0),
                    block(17, 0),
                    block(13, 1),
                    block(15, 1),
                    block(17, 1),
                    block(13, 2),
                    block(15, 2),
                    block(17, 2),
                ],
            ),
            mascot: Some(mascot(
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_THROW, 4),
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_GAME_OVER, 21),
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_VICTORY, 13),
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_IDLE, 6),
            )),
        },
        geometry,
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
                hard_drop: None,
            },
        )?
        .with_game_music(sound::FEVER_INTRO, sound::FEVER_REPEAT)?
        .with_game_over_music(sound::GAME_OVER, None)?
        .with_next_stage_music(sound::FEVER_NEXT_LEVEL, None)?
        .with_victory_music(sound::VICTORY_INTRO, sound::VICTORY_REPEAT)?,
        font: FontThemeOptions::new(
            vec![
                FontRenderOptions::numeric_sprites(sprites::FONT_SMALL, texture_creator, 1)?,
                FontRenderOptions::numeric_sprites(sprites::FONT_LARGE, texture_creator, 0)?,
            ],
            vec![
                (
                    MetricKind::Score,
                    ThemedNumeric::new(
                        0,
                        MetricSnips::zero_fill((116, 104), crate::game::MAX_SCORE),
                    ),
                ),
                (
                    MetricKind::Level,
                    ThemedNumeric::new(
                        1,
                        MetricSnips::zero_fill((134, 140), crate::game::rules::MAX_VIRUS_LEVEL),
                    ),
                ),
                (
                    MetricKind::Viruses,
                    ThemedNumeric::new(
                        1,
                        MetricSnips::zero_fill((134, 180), crate::game::random::MAX_VIRUSES),
                    ),
                ),
            ],
        ),
        board_file: sprites::BOTTLES,
        board_alpha: 0xff,
        board_snips: vec![Rect::new(0, 0, 96, 209)],
        top_padding: 0,
        shadow: None,
        board_point: Point::new(0, 0),
        background_file: sprites::BACKGROUND,
        background_color: Color::BLACK,
        match_end_file: Some(sprites::MATCH_END),
        game_over_points: vec![Point::new(1, 1)],
        interstitial_points: vec![Point::new(82, 1)],
        overlay_size: None,
        hold: Some(HoldLayout::Point {
            point: Point::new(155, 13),
            scale: Some(0.82),
        }),
        peek: PeekLayout::Column {
            point: Point::new(110, 55),
            offset: 10,
            max: 2,
            scale: Some(0.82),
        },
        mascot: Some(MascotLayout {
            hand_point: Point::new(108, 39),
            spawn_point: Point::new(113, 6),
            game_over_point: Point::new(110, 8),
            victory_point: Point::new(113, 6),
            draw_first: true,
        }),
        mascot_animations: Some(retro_mascot(
            FrameAnimationType::YoYo { fps: 10 },
            N64_VICTORY,
            N64_GAME_OVER,
        )),
        spawn_arc: Some((Point::new(108, 39), geometry.point(spawn_cell()))),
        cell_idle_type: FrameAnimationType::YoYo { fps: 5 },
        destroy_style: None,
        game_over_style: None,
        curtain_cell: None,
        // Dr. Rustario and Rustris take a hit as it arrives, so nothing ever waits
        pending: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
    };
    retro_theme(canvas, texture_creator, options)
}
