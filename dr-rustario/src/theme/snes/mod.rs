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
    pub const BACKGROUND_TILE: &[u8] = include_bytes!("background-tile.png");
}
mod sound {
    pub const FEVER_INTRO: &[u8] = include_bytes!("fever-intro.ogg");
    pub const FEVER_REPEAT: &[u8] = include_bytes!("fever-repeat.ogg");
    pub const FEVER_NEXT_LEVEL: &[u8] = include_bytes!("fever-next-level.ogg");
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
    pub const VICTORY_INTRO: &[u8] = include_bytes!("victory-intro.ogg");
    pub const VICTORY_REPEAT: &[u8] = include_bytes!("victory-repeat.ogg");
    pub const NEXT_LEVEL_JINGLE: &[u8] = include_bytes!("next-level-jingle.ogg");
}

pub const BLOCK_SIZE: u32 = 8;

fn block(i: i32, j: i32) -> Point {
    Point::new(i * BLOCK_SIZE as i32, j * BLOCK_SIZE as i32)
}

fn layout(j: i32) -> ColorLayout {
    ColorLayout {
        north: [block(4, j), block(5, j)],
        east: [block(11, j), block(10, j)],
        south: [block(5, j), block(4, j)],
        west: [block(10, j), block(11, j)],
        garbage: block(3, j),
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
        strip(sprites::VITAMINS, block(0, j), 2, BLOCK_SIZE),
        strip(sprites::VITAMINS, block(2, j), 1, BLOCK_SIZE),
        strip(sprites::VITAMINS, block(2, j), 1, BLOCK_SIZE),
    )
}

fn match_end(i: i32, j: i32) -> Point {
    Point::new(i * 65 + 1, j * 129 + 1)
}

pub fn snes_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let geometry = BoardGeometry::new(
        BLOCK_SIZE,
        0,
        (7, 39),
        BOTTLE_WIDTH,
        BOTTLE_HEIGHT,
        BOTTLE_HEIGHT,
    );
    let scene = SceneType::Tile {
        texture: sprites::BACKGROUND_TILE,
    };
    let options = RetroThemeOptions {
        name: "snes",
        scenes: vec![scene],
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
                    block(4, 0),
                    block(6, 0),
                    block(8, 0),
                    block(4, 1),
                    block(6, 1),
                    block(8, 1),
                    block(4, 2),
                    block(6, 2),
                    block(8, 2),
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
        .with_next_stage_music(sound::FEVER_NEXT_LEVEL, None)?
        .with_victory_music(sound::VICTORY_INTRO, sound::VICTORY_REPEAT)?,
        font: FontThemeOptions::simple(
            FontRenderOptions::numeric_sprites(sprites::FONT, texture_creator, 1)?,
            hud(
                MetricSnips::zero_fill((91, 110), crate::game::MAX_SCORE),
                MetricSnips::zero_fill((123, 131), crate::game::rules::MAX_VIRUS_LEVEL),
                MetricSnips::zero_fill((123, 152), crate::game::random::MAX_VIRUSES),
            ),
        ),
        board_file: sprites::BOTTLES,
        board_alpha: 0xff,
        board_snips: vec![Rect::new(0, 0, 79, 175)],
        top_padding: 0,
        shadow: None,
        board_point: Point::new(0, 0),
        background_file: sprites::BACKGROUND,
        background_color: Color::BLACK,
        match_end_file: Some(sprites::MATCH_END),
        game_over_points: vec![match_end(0, 0), match_end(1, 0)],
        interstitial_points: vec![
            match_end(2, 0),
            match_end(3, 0),
            match_end(4, 0),
            match_end(0, 1),
            match_end(1, 1),
            match_end(2, 1),
            match_end(3, 1),
            match_end(4, 1),
        ],
        overlay_size: None,
        hold: Some(HoldLayout::Point {
            point: Point::new(125, 18),
            scale: Some(0.82),
        }),
        peek: PeekLayout::Column {
            point: Point::new(96, 46),
            offset: 10,
            max: 2,
            scale: Some(0.82),
        },
        mascot: Some(MascotLayout {
            hand_point: Point::new(103, 22),
            spawn_point: Point::new(99, 29),
            game_over_point: Point::new(100, 31),
            victory_point: Point::new(105, 31),
            draw_first: false,
        }),
        mascot_animations: Some(retro_mascot(
            FrameAnimationType::Static,
            NES_SNES_VICTORY,
            FrameAnimationType::Static,
        )),
        spawn_arc: Some((Point::new(103, 22), geometry.point(spawn_cell()))),
        cell_idle_type: FrameAnimationType::Linear { fps: 3 },
        destroy_style: None,
        game_over_style: None,
        curtain_cell: None,
        // Dr. Rustario and Rustris take a hit as it arrives, so nothing ever waits
        pending: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
        pop_debris: None,
        nuisance_rumble: None,
        attack_ball: None,
    };
    retro_theme(canvas, texture_creator, options)
}
