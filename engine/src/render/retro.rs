//! Builds a [`Theme`] from pre-drawn art: a background, board frame(s), a sprite sheet and
//! optional match-end overlays and mascot.

use crate::animate::destroy::DestroyStyle;
use crate::animate::frames::FrameAnimationType;
use crate::animate::game_over::GameOverStyle;
use crate::animate::mascot::MascotAnimationTypes;
use crate::animate::spawn::SpawnArc;
use crate::animate::{AnimationMeta, PopDebris};
use crate::game::CellId;
use crate::render::font::{FontThemeOptions, PopupFont};
use crate::render::geometry::BoardGeometry;
use crate::render::helper::{TextureFactory, TextureQuery};
use crate::render::scene::SceneType;
use crate::render::sound::AudioTheme;
use crate::render::sprite_sheet::{BlockSpriteSheet, BlockSpriteSheetData, GhostStyle, MascotKind};
use crate::render::{AttackBallData, AttackBallSprites};
use crate::render::{
    HoldLayout, MascotLayout, MatchEndSprites, OverlayFit, PanelShadow, PeekLayout, PendingLayout,
    Theme, ThemeFamily,
};
use crate::scale::ScaleMode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

pub struct RetroThemeOptions {
    pub name: &'static str,
    /// one per speed band
    pub scenes: Vec<SceneType>,
    pub sprites: BlockSpriteSheetData,
    pub geometry: BoardGeometry,
    pub audio: AudioTheme,
    pub font: FontThemeOptions,
    pub board_file: &'static [u8],
    /// board frame opacity; below 0xff the scene shows through the board
    pub board_alpha: u8,
    /// the board frame per speed band within `board_file`; empty for the whole file
    pub board_snips: Vec<Rect>,
    /// transparent rows added above the board and background art, for a visible buffer
    /// above the skyline
    pub top_padding: u32,
    /// what the panel casts on the scene behind it; `None` for a theme whose panel fills the
    /// window, or whose scene is busy enough that a shadow would only muddy it
    pub shadow: Option<PanelShadow>,
    /// where the board frame sits in the background
    pub board_point: Point,
    pub background_file: &'static [u8],
    pub background_color: Color,
    /// full-board overlays: game over and stage-clear frames
    pub match_end_file: Option<&'static [u8]>,
    pub game_over_points: Vec<Point>,
    pub interstitial_points: Vec<Point>,
    /// overlays either the size of the board or drawn centred at this size
    pub overlay_size: Option<(u32, u32)>,
    pub hold: Option<HoldLayout>,
    pub peek: PeekLayout,
    /// where attacks queued against the player are drawn; `None` for a game that takes its
    /// hits the moment they arrive and so never has any waiting
    pub pending: Option<PendingLayout>,
    pub mascot: Option<MascotLayout>,
    pub mascot_animations: Option<MascotAnimationTypes>,
    /// where a spawning piece is thrown from and to, in background coordinates
    pub spawn_arc: Option<(Point, Point)>,
    pub cell_idle_type: FrameAnimationType,
    /// defaults to popping each cell with its own strip
    pub destroy_style: Option<DestroyStyle>,
    /// defaults to the game over overlay frames
    pub game_over_style: Option<GameOverStyle>,
    pub curtain_cell: Option<CellId>,
    pub ghost_style: GhostStyle,
    /// rows the hard drop trail falls per 4ms frame; see `animate::hard_drop`
    pub hard_drop_rows_per_frame: f64,
    /// what a popping cell throws off, for a theme that bursts rather than simply vanishing
    pub pop_debris: Option<PopDebris>,
    /// what an attack crossing the window is drawn as; without it, the popped cell's sprite
    pub attack_ball: Option<AttackBallData>,
    /// how hard the board shakes when nuisance lands, if the theme wants it to at all
    pub nuisance_rumble: Option<(f64, Duration)>,
}

pub fn retro_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    options: RetroThemeOptions,
) -> Result<Theme<'a>, String> {
    let sprites = BlockSpriteSheet::new(canvas, texture_creator, &options.sprites, None)?;
    let mut board_texture = padded_texture(
        canvas,
        texture_creator,
        options.board_file,
        options.top_padding,
    )?;
    if options.board_alpha < 0xff {
        // written verbatim into the transparent board target (no blend) so the scene
        // is blended through exactly once, when the board target is composited
        board_texture.set_blend_mode(sdl2::render::BlendMode::None);
        board_texture.set_alpha_mod(options.board_alpha);
    }

    let background_texture = padded_texture(
        canvas,
        texture_creator,
        options.background_file,
        options.top_padding,
    )?;
    let background_size = background_texture.size();

    let font = options.font.build(texture_creator)?;

    let board_snips = if options.board_snips.is_empty() {
        let (w, h) = board_texture.size();
        vec![Rect::new(0, 0, w, h)]
    } else {
        options.board_snips
    };
    let board_size = (board_snips[0].width(), board_snips[0].height());

    let match_end = match options.match_end_file {
        Some(file) => {
            let (w, h) = options
                .overlay_size
                .unwrap_or((options.geometry.width(), options.geometry.height()));
            let overlay = |p: &Point| Rect::new(p.x, p.y, w, h);
            Some(MatchEndSprites {
                texture: texture_creator.load_texture_bytes_blended(file)?,
                game_over_snips: options.game_over_points.iter().map(overlay).collect(),
                interstitial_snips: options.interstitial_points.iter().map(overlay).collect(),
                fit: if options.overlay_size.is_some() {
                    OverlayFit::Center
                } else {
                    OverlayFit::Stretch
                },
            })
        }
        None => None,
    };

    let mascot = match (options.mascot_animations, sprites.mascot()) {
        (Some(types), Some(mascot)) => Some(types.with_frames(
            mascot.sheet(MascotKind::Idle).frame_count(),
            mascot.sheet(MascotKind::Spawn).frame_count(),
            mascot.sheet(MascotKind::Victory).frame_count(),
            mascot.sheet(MascotKind::GameOver).frame_count(),
        )),
        _ => None,
    };

    let animation_meta = AnimationMeta {
        destroy: options.destroy_style.unwrap_or_else(|| sprites.pop_style()),
        game_over: options.game_over_style.unwrap_or(GameOverStyle::Screen {
            frames: options.game_over_points.len().max(1),
        }),
        interstitial_frames: options.interstitial_points.len().max(1),
        cell_idle_type: options.cell_idle_type,
        cell_idle: sprites.idle_cells(),
        spawn_arc: options.spawn_arc.map(|(start, end)| SpawnArc {
            start,
            end,
            block_size: options.geometry.block_size(),
        }),
        mascot,
        hard_drop_rows_per_frame: options.hard_drop_rows_per_frame,
        pop_debris: options.pop_debris,
        nuisance_rumble: options.nuisance_rumble,
    };

    let mut scenes = vec![];
    for scene in options.scenes.iter() {
        scenes.push(scene.build(canvas, texture_creator)?);
    }
    assert!(!scenes.is_empty(), "a theme needs at least one scene");

    let popup_font = PopupFont::new(canvas, texture_creator, options.geometry.block_size())?;
    // a theme that cut no ball art falls back to the popped cell's own sprite, which every
    // theme has - the same rule the pending tray follows
    let attack_ball = match options.attack_ball {
        Some(data) => Some(AttackBallSprites::new(
            data.sheet.sprite_sheet(texture_creator)?,
            data.scale,
            data.big_attack,
        )),
        None => None,
    };
    Ok(Theme {
        name: options.name,
        scenes,
        sprites,
        geometry: options.geometry,
        audio: options.audio,
        popup_font,
        font,
        board_texture,
        board_snips,
        background_texture,
        board_bg_snip: Rect::new(
            options.board_point.x(),
            options.board_point.y(),
            board_size.0,
            board_size.1,
        ),
        background_size,
        background_color: options.background_color,
        mascot: options.mascot,
        animation_meta,
        match_end,
        curtain_cell: options.curtain_cell,
        hold: options.hold,
        peek: options.peek,
        pending: options.pending,
        attack_ball,
        ghost_style: options.ghost_style,
        particle_color: None,
        particle_palette: vec![],
        family: ThemeFamily::Retro,
        scale_mode: ScaleMode::Source,
        // the band above the skyline is where pieces spawn, so it is not empty
        top_slack: 0,
        shadow: options.shadow,
    })
}

/// load a PNG with `padding` transparent pixels added above it
fn padded_texture<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    file: &'static [u8],
    padding: u32,
) -> Result<Texture<'a>, String> {
    let raw = texture_creator.load_texture_bytes_blended(file)?;
    if padding == 0 {
        return Ok(raw);
    }
    let (width, height) = raw.size();
    let mut texture = texture_creator.create_texture_target_blended(width, height + padding)?;
    canvas
        .with_texture_canvas(&mut texture, |c| {
            c.set_draw_color(Color::RGBA(0, 0, 0, 0));
            c.clear();
            c.copy(&raw, None, Rect::new(0, padding as i32, width, height))
                .unwrap();
        })
        .map_err(|e| e.to_string())?;
    Ok(texture)
}
