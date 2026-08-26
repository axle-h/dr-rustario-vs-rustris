//! Builds a [`Theme`] procedurally, sized to the window: a bordered board, a HUD table and
//! space for the mascot and queue beside it.

use crate::animate::destroy::DestroyStyle;
use crate::animate::frames::FrameAnimationType;
use crate::animate::game_over::GameOverStyle;
use crate::animate::mascot::MascotAnimationTypes;
use crate::animate::spawn::SpawnArc;
use crate::animate::AnimationMeta;
use crate::font::FontType;
use crate::game::geometry::Point as CellPoint;
use crate::game::MetricKind;
use crate::render::font::{FontRender, FontTheme, ThemedNumeric};
use crate::render::geometry::BoardGeometry;
use crate::render::helper::{TextureFactory, TextureQuery};
use crate::render::metrics_table::{metric_label, GameMetricsTable};
use crate::render::scene::{ClearParticles, SceneType};
use crate::render::sound::AudioTheme;
use crate::render::sprite_sheet::{BlockSpriteSheet, BlockSpriteSheetData, GhostStyle, MascotKind};
use crate::render::{
    HoldLayout, MascotLayout, MatchEndSprites, OverlayFit, PeekLayout, Theme, ThemeFamily,
};
use crate::scale::ScaleMode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

const BOARD_TOP_BUFFER_PCT: f64 = 0.15;
const BOARD_BORDER_PCT_OF_BLOCK: f64 = 0.5;
const BOARD_BORDER_SHADOW: u8 = 0x99;
const VERTICAL_GUTTER_PCT_OF_BLOCK: f64 = 0.2;
const PEEK_SCALE: f64 = 0.8;
/// without a mascot the queue is a column of slots this many blocks square, the first bigger
const SLOT_BLOCKS: f64 = 1.5;
const BIG_SLOT_BLOCKS: f64 = 2.5;
// most pieces are 3 blocks wide: fill the slot with them and let I and O meet in the middle
const SLOT_MAX_SCALE: f64 = SLOT_BLOCKS / 3.0;
const BIG_SLOT_MAX_SCALE: f64 = BIG_SLOT_BLOCKS / 3.0;

pub struct ModernThemeOptions {
    pub name: &'static str,
    /// sprite sources; the sheet is rescaled to the computed block size
    pub sprites: BlockSpriteSheetData,
    pub audio: AudioTheme,
    pub columns: u32,
    pub rows: u32,
    pub visible_rows: u32,
    /// the cell pitch to build at. The board is drawn at its built size or smaller, so pass
    /// the largest block the window can hold: see [`crate::scale::fit`].
    pub block_size: u32,
    /// the topmost visible rows, kept above the skyline for spawning pieces. Like the gap
    /// above the board they may fall off the top of the window rather than cost a whole step.
    pub top_buffer_rows: u32,
    /// HUD rows under the side column, top to bottom, with the largest value each can show
    pub metrics: Vec<(MetricKind, u32)>,
    /// HUD rows under the hold box on the left
    pub metrics_left: Vec<(MetricKind, u32)>,
    /// the mascot's animation types and its height in blocks
    pub mascot: Option<(MascotAnimationTypes, f64)>,
    /// where the spawning piece lands, for the throw arc
    pub spawn_cell: CellPoint,
    pub cell_idle_type: FrameAnimationType,
    pub queue_max: u32,
    pub particle_color: Color,
    /// what this theme radiates into the background particle field, see
    /// [`crate::particles::field`]. `particle_color` stays the fallback, so the foreground
    /// burst effects are unaffected.
    pub particle_palette: Vec<Color>,
    pub clear_particles: ClearParticles,
    pub destroy_style: Option<DestroyStyle>,
    pub game_over_style: Option<GameOverStyle>,
    pub ghost_style: GhostStyle,
    /// rows the hard drop trail falls per 4ms frame; see `animate::hard_drop`
    pub hard_drop_rows_per_frame: f64,
}

pub fn modern_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    options: ModernThemeOptions,
) -> Result<Theme<'a>, String> {
    let (_, window_height) = canvas.window().size();

    let board_top_buffer = (BOARD_TOP_BUFFER_PCT * window_height as f64).round() as u32;
    let block_size = options.block_size as f64;
    // a retro theme's board frame starts at the skyline with the spawning rows floating
    // above it; the queue and hold sit alongside. Match that.
    let above_skyline = options.top_buffer_rows * options.block_size;
    let skyline = board_top_buffer + above_skyline;
    let border_weight = (block_size * BOARD_BORDER_PCT_OF_BLOCK).round() as u32;
    let vertical_gutter = (VERTICAL_GUTTER_PCT_OF_BLOCK * block_size).round() as u32;
    let slot_size = (SLOT_BLOCKS * block_size).round() as u32;
    let big_slot_size = (BIG_SLOT_BLOCKS * block_size).round() as u32;
    let peek_width = if options.mascot.is_some() {
        (2.0 * PEEK_SCALE * block_size).round() as u32
    } else {
        slot_size
    };
    let block_size = block_size.round() as u32;

    let geometry = BoardGeometry::new(
        block_size,
        0,
        (border_weight as i32, board_top_buffer as i32),
        options.columns,
        options.rows,
        options.visible_rows,
    );

    let font_size = 2 * block_size / 3;
    let font = FontRender::from_font(
        canvas,
        texture_creator,
        FontType::Normal,
        font_size,
        Color::WHITE,
    )?;
    let font_bold = FontRender::from_font(
        canvas,
        texture_creator,
        FontType::Bold,
        font_size,
        Color::WHITE,
    )?;
    let font_match_end = FontRender::from_font(
        canvas,
        texture_creator,
        FontType::Retro,
        font_size * 3,
        Color::WHITE,
    )?;

    let board_snip = Rect::new(
        0,
        0,
        geometry.width() + 2 * border_weight,
        board_top_buffer + geometry.height() + border_weight,
    );
    let board_bg_snip = Rect::new(
        (peek_width + vertical_gutter) as i32,
        0,
        board_snip.width(),
        board_snip.height(),
    );
    let mut metrics_right = GameMetricsTable::new(
        geometry.height() + board_top_buffer,
        &font,
        &font_bold,
        &options.metrics,
    );
    metrics_right.offset_x(board_bg_snip.right() + vertical_gutter as i32);
    let metrics_left = GameMetricsTable::new(
        geometry.height() + board_top_buffer,
        &font,
        &font_bold,
        &options.metrics_left,
    );

    let mut sprite_data = options.sprites;
    if let (Some(mascot), Some((_, height_in_blocks))) =
        (sprite_data.mascot.as_mut(), options.mascot)
    {
        // the mascot is scaled to a height in blocks, taking its tallest strip as reference
        let reference = mascot
            .spawn
            .frame_count()
            .map(|_| mascot.spawn.clone())
            .unwrap_or(mascot.idle.clone());
        let sheet = reference.sprite_sheet(texture_creator)?;
        let (width, _) = sheet.frame_size();
        mascot.scale = Some(height_in_blocks * block_size as f64 / width as f64);
    }
    let sprites = BlockSpriteSheet::new(canvas, texture_creator, &sprite_data, block_size)?;

    let side_y = skyline as i32;
    let side_x = board_bg_snip.right() + vertical_gutter as i32;

    let (mascot_layout, mascot_meta, side_width, hand_point) =
        match (sprites.mascot(), options.mascot) {
            (Some(mascot), Some((types, _))) => {
                let sizes = [
                    MascotKind::Spawn,
                    MascotKind::GameOver,
                    MascotKind::Victory,
                    MascotKind::Idle,
                ]
                .map(|kind| mascot.sheet(kind).frame_size());
                let width = sizes.iter().map(|(w, _)| *w).max().unwrap();
                let height = sizes.iter().map(|(_, h)| *h).max().unwrap();
                let point =
                    |kind_height: u32| Point::new(side_x, side_y + (height - kind_height) as i32);
                // HACK the hand point is empirical... how else would we find it?!
                let hand_point = Point::new(side_x, side_y + 10 * height as i32 / 19);
                let layout = MascotLayout {
                    hand_point,
                    spawn_point: point(sizes[0].1),
                    game_over_point: point(sizes[1].1),
                    victory_point: point(sizes[2].1),
                    draw_first: true,
                };
                let meta = types.with_frames(
                    mascot.sheet(MascotKind::Idle).frame_count(),
                    mascot.sheet(MascotKind::Spawn).frame_count(),
                    mascot.sheet(MascotKind::Victory).frame_count(),
                    mascot.sheet(MascotKind::GameOver).frame_count(),
                );
                (Some(layout), Some(meta), width, hand_point)
            }
            _ => (None, None, big_slot_size, Point::new(side_x, side_y)),
        };

    let mut borders = vec![];
    let step = BOARD_BORDER_SHADOW / border_weight.max(1) as u8;
    for i in 0..border_weight {
        let j = border_weight - i - 1;
        let alpha = if j > 0 {
            BOARD_BORDER_SHADOW - j as u8 * step
        } else {
            0xff
        };
        let rect = Rect::new(
            i as i32,
            skyline as i32,
            geometry.width() - 2 * i + 2 * border_weight,
            geometry.height() - above_skyline - i + border_weight,
        );
        borders.push((rect, alpha))
    }

    let all_metrics = metrics_right.rows();
    let mut board_texture =
        texture_creator.create_texture_target_blended(board_snip.width(), board_snip.height())?;
    canvas
        .with_texture_canvas(&mut board_texture, |c| {
            c.set_draw_color(Color::RGBA(0, 0, 0, 0));
            c.clear();
            for (r, color) in borders.iter().copied() {
                c.set_draw_color(Color::RGBA(color, color, color, color));
                c.draw_rect(r).unwrap();
            }
            // re-clear the board to get rid of the top of the border
            c.set_draw_color(Color::RGBA(0, 0, 0, 0));
            c.fill_rect(Rect::new(
                border_weight as i32,
                0,
                geometry.width(),
                board_top_buffer + geometry.height(),
            ))
            .unwrap();
        })
        .map_err(|e| e.to_string())?;

    let left_width = peek_width.max(metrics_left.width());
    let mut bg_texture = texture_creator.create_texture_target_blended(
        board_bg_snip.right() as u32 + vertical_gutter + side_width.max(metrics_right.width()),
        board_bg_snip.height(),
    )?;
    let background_size = bg_texture.size();
    let left_metrics = metrics_left.rows();
    canvas
        .with_texture_canvas(&mut bg_texture, |c| {
            c.set_draw_color(Color::RGBA(0, 0, 0, 0));
            c.clear();
            for row in all_metrics.iter().chain(left_metrics.iter()) {
                font_bold
                    .render_string(c, row.label(), metric_label(row.metric()))
                    .unwrap();
            }
        })
        .map_err(|e| e.to_string())?;
    let _ = left_width;

    let spawn_arc = mascot_layout.map(|layout| SpawnArc {
        start: layout.hand_point,
        end: geometry
            .point(options.spawn_cell)
            .offset(board_bg_snip.left(), 0),
        block_size: geometry.block_size(),
    });

    let animation_meta = AnimationMeta {
        destroy: options.destroy_style.unwrap_or_else(|| sprites.pop_style()),
        game_over: options
            .game_over_style
            .unwrap_or(GameOverStyle::Screen { frames: 1 }),
        interstitial_frames: 1,
        cell_idle_type: options.cell_idle_type,
        cell_idle: sprites.idle_cells(),
        spawn_arc,
        mascot: mascot_meta,
        hard_drop_rows_per_frame: options.hard_drop_rows_per_frame,
    };

    let mut match_end_texture =
        texture_creator.create_texture_target_blended(geometry.width() * 2, geometry.height())?;
    let game_over_snip = Rect::new(0, 0, geometry.width(), geometry.height());
    let next_stage_snip = Rect::new(
        geometry.width() as i32,
        0,
        geometry.width(),
        geometry.height(),
    );
    canvas
        .with_texture_canvas(&mut match_end_texture, |c| {
            c.set_draw_color(Color::RGBA(0, 0, 0, 100));
            c.clear();
            font_match_end
                .render_string_in_center(c, game_over_snip, "game over")
                .unwrap();
            font_match_end
                .render_string_in_center(c, next_stage_snip, "next level")
                .unwrap();
        })
        .map_err(|e| e.to_string())?;

    let font_theme = FontTheme::new(
        vec![font],
        all_metrics
            .iter()
            .chain(left_metrics.iter())
            .map(|row| (row.metric(), ThemedNumeric::new(0, row.value())))
            .collect(),
    );

    let scene_type = SceneType::Particles {
        base_color: options.particle_color,
        clear: options.clear_particles,
    };

    let (hold, peek) = if mascot_layout.is_some() {
        (
            HoldLayout::Point {
                point: Point::new(0, side_y),
                scale: Some(PEEK_SCALE),
            },
            PeekLayout::Column {
                point: hand_point + Point::new(0, (1.5 * block_size as f64).round() as i32),
                offset: block_size as i32,
                max: options.queue_max,
                scale: Some(PEEK_SCALE),
            },
        )
    } else {
        // a column of slots beside the board, the next piece largest
        let mut slots = vec![Rect::new(side_x, side_y, big_slot_size, big_slot_size)];
        let mut y = side_y + big_slot_size as i32 + vertical_gutter as i32;
        for _ in 1..options.queue_max.max(1) {
            slots.push(Rect::new(side_x, y, slot_size, slot_size));
            y += slot_size as i32 + vertical_gutter as i32;
        }
        (
            HoldLayout::Slot {
                slot: Rect::new(0, side_y, slot_size, slot_size),
                max_scale: SLOT_MAX_SCALE,
            },
            PeekLayout::Slots {
                slots,
                max_scale: BIG_SLOT_MAX_SCALE,
            },
        )
    };

    Ok(Theme {
        name: options.name,
        scenes: vec![scene_type.build(canvas, texture_creator)?],
        sprites,
        geometry,
        audio: options.audio,
        font: font_theme,
        board_texture,
        board_snips: vec![board_snip],
        background_texture: bg_texture,
        board_bg_snip,
        background_size,
        background_color: Color::BLACK,
        mascot: mascot_layout,
        animation_meta,
        match_end: Some(MatchEndSprites {
            texture: match_end_texture,
            game_over_snips: vec![game_over_snip],
            interstitial_snips: vec![next_stage_snip],
            fit: OverlayFit::Stretch,
        }),
        curtain_cell: None,
        hold: Some(hold),
        peek,
        ghost_style: options.ghost_style,
        particle_color: Some(options.particle_color),
        particle_palette: options.particle_palette,
        family: ThemeFamily::Particle,
        scale_mode: ScaleMode::Native,
        // nothing is ever drawn in the gap above the board, unlike the rows below it
        top_slack: board_top_buffer,
    })
}
