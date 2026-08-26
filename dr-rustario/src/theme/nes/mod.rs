use crate::game::bottle::{BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::pill::VirusColor;
use crate::theme::data::{
    animations, audio, cells, hud, mascot, previews, retro_mascot, spawn_cell, strip, ColorLayout,
    Sounds, NES_SNES_VICTORY,
};
use engine::animate::frames::FrameAnimationType;
use engine::config::Config;
use engine::render::animation::AnimationSpriteSheetData;
use engine::render::font::{FontRenderOptions, FontThemeOptions, MetricSnips};
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
    pub const FONT: &[u8] = include_bytes!("font.png");
    pub const MATCH_END: &[u8] = include_bytes!("match-end.png");
}
mod sound {
    pub const FEVER_INTRO: &[u8] = include_bytes!("fever-intro.ogg");
    pub const FEVER_REPEAT: &[u8] = include_bytes!("fever-repeat.ogg");
    pub const FEVER_NEXT_LEVEL_INTRO: &[u8] = include_bytes!("fever-next-level-intro.ogg");
    pub const FEVER_NEXT_LEVEL_REPEAT: &[u8] = include_bytes!("fever-next-level-repeat.ogg");
    pub const DESTROY_VIRUS: &[u8] = include_bytes!("destroy-virus.ogg");
    pub const DESTROY_VIRUS_COMBO: &[u8] = include_bytes!("destroy-virus-combo.ogg");
    pub const DESTROY_VITAMIN: &[u8] = include_bytes!("destroy-vitamin.ogg");
    pub const DESTROY_VITAMIN_COMBO: &[u8] = include_bytes!("destroy-vitamin-combo.ogg");
    pub const GAME_OVER_INTRO: &[u8] = include_bytes!("game-over-intro.ogg");
    pub const GAME_OVER_REPEAT: &[u8] = include_bytes!("game-over-repeat.ogg");
    pub const RECEIVE_GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const SPEED_LEVEL_UP: &[u8] = include_bytes!("speed-level-up.ogg");
    pub const DROP: &[u8] = include_bytes!("drop.ogg");
    pub const MOVE_PILL: &[u8] = include_bytes!("move.ogg");
    pub const PAUSE: &[u8] = include_bytes!("pause.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    // const VIRUS_DEAD: &[u8] = include_bytes!("virus-dead.ogg");
    pub const VICTORY_INTRO: &[u8] = include_bytes!("victory-intro.ogg");
    pub const VICTORY_REPEAT: &[u8] = include_bytes!("victory-repeat.ogg");
    pub const NEXT_LEVEL_JINGLE: &[u8] = include_bytes!("next-level-jingle.ogg");
}

pub const BLOCK_SIZE: u32 = 7;

// 2 block wide + 2 outside borders + 1 inside border
const PILL_WIDTH: u32 = BLOCK_SIZE * 2 + 3;
// 1 block high + 2 outside borders
const PILL_HEIGHT: u32 = BLOCK_SIZE + 2;

fn block(i: i32, j: i32) -> Point {
    Point::new(i * BLOCK_SIZE as i32, j * BLOCK_SIZE as i32)
}

fn layout(j: i32) -> ColorLayout {
    ColorLayout {
        north: [block(0, j), block(1, j)],
        east: [block(2, j), block(3, j)],
        south: [block(1, j), block(0, j)],
        west: [block(3, j), block(2, j)],
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
        strip(sprites::VITAMINS, block(6, j), 2, BLOCK_SIZE),
        strip(sprites::VITAMINS, block(5, j), 1, BLOCK_SIZE),
        strip(sprites::VITAMINS, block(5, j), 1, BLOCK_SIZE),
    )
}

fn pill(i: i32, j: i32) -> Point {
    Point::new(57 + i * 17, j * 9)
}

pub fn nes_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let geometry = BoardGeometry::new(7, 1, (8, 40), BOTTLE_WIDTH, BOTTLE_HEIGHT, BOTTLE_HEIGHT);
    // we take 1 away from the throw end as thrown pills have a border but bottle pills do not
    let throw_end = geometry.point(spawn_cell()) + Point::new(-1, -1);
    let checkerboard = |color| SceneType::Checkerboard {
        width: 8,
        height: 8,
        colors: [Color::BLACK, color],
    };
    let options = RetroThemeOptions {
        name: "nes",
        scenes: vec![
            checkerboard(Color::RGB(0x00, 0x3f, 0x00)),
            checkerboard(Color::RGB(0x2d, 0x05, 0x85)),
            checkerboard(Color::RGB(0x58, 0x58, 0x58)),
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
            ghost_alpha: 0x40,
            previews: previews(
                sprites::VITAMINS,
                (PILL_WIDTH, PILL_HEIGHT),
                [
                    pill(0, 0),
                    pill(1, 0),
                    pill(2, 0),
                    pill(0, 1),
                    pill(1, 1),
                    pill(2, 1),
                    pill(0, 2),
                    pill(1, 2),
                    pill(2, 2),
                ],
            ),
            mascot: Some(mascot(
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_THROW, 3),
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_GAME_OVER, 1),
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_VICTORY, 2),
                AnimationSpriteSheetData::exclusive_linear(sprites::DR_IDLE, 1),
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
        .with_game_over_music(sound::GAME_OVER_INTRO, sound::GAME_OVER_REPEAT)?
        .with_next_stage_music(
            sound::FEVER_NEXT_LEVEL_INTRO,
            sound::FEVER_NEXT_LEVEL_REPEAT,
        )?
        .with_victory_music(sound::VICTORY_INTRO, sound::VICTORY_REPEAT)?,
        font: FontThemeOptions::simple(
            FontRenderOptions::numeric_sprites(sprites::FONT, texture_creator, 1)?,
            hud(
                MetricSnips::zero_fill((92, 113), crate::game::MAX_SCORE),
                MetricSnips::zero_fill((123, 134), crate::game::rules::MAX_VIRUS_LEVEL),
                MetricSnips::zero_fill((123, 155), crate::game::random::MAX_VIRUSES),
            ),
        ),
        board_file: sprites::BOTTLES,
        board_alpha: 0xff,
        board_snips: vec![
            Rect::new(81, 0, 80, 176),
            Rect::new(0, 0, 80, 176),
            Rect::new(162, 0, 80, 176),
        ],
        top_padding: 0,
        board_point: Point::new(0, 0),
        background_file: sprites::BACKGROUND,
        background_color: Color::BLACK,
        match_end_file: Some(sprites::MATCH_END),
        game_over_points: vec![Point::new(65, 0), Point::new(65, 129)],
        interstitial_points: vec![Point::new(0, 0), Point::new(0, 129)],
        overlay_size: None,
        hold: Some(HoldLayout::Point {
            point: Point::new(125, 30),
            scale: Some(0.75),
        }),
        peek: PeekLayout::Column {
            point: Point::new(94, 55),
            offset: 10,
            max: 2,
            scale: Some(0.75),
        },
        mascot: Some(MascotLayout {
            hand_point: Point::new(102, 30),
            spawn_point: Point::new(97, 37),
            game_over_point: Point::new(97, 37),
            victory_point: Point::new(102, 37),
            draw_first: false,
        }),
        mascot_animations: Some(retro_mascot(
            FrameAnimationType::Static,
            NES_SNES_VICTORY,
            FrameAnimationType::Static,
        )),
        spawn_arc: Some((Point::new(102, 30), throw_end)),
        cell_idle_type: FrameAnimationType::Linear { fps: 3 },
        destroy_style: None,
        game_over_style: None,
        curtain_cell: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
    };
    retro_theme(canvas, texture_creator, options)
}
