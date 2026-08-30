//! Every theme scaled to the window, with per-player render targets and animations, and the
//! cross-fade when a player changes theme. Every theme keeps animation state for every
//! player so a mid-match theme change is seamless.

use crate::animate::attack_ball::AttackBallAnimation;
use crate::animate::debris::{BurstSpec, DebrisArt, Spread};
use crate::animate::event::AnimationEvent;
use crate::animate::nuisance::NuisanceFall;
use crate::animate::PlayerAnimations;
use crate::config::VideoConfig;
use crate::game::geometry::Point as CellPoint;
use crate::game::{CellId, Game, PieceId, PlacedCell};
use crate::particles::field::context::Palette;
use crate::render::layout::BoardLayout;
use crate::render::sound::AudioTheme;
use crate::render::Theme;
use crate::scale::{Scale, ScaleMode};
use crate::session::MatchState;
use rand::prelude::ThreadRng;
use rand::rng;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGBA8888;
use sdl2::rect::{Point, Rect};
use sdl2::render::{
    BlendMode, ScaleMode as TextureScaleMode, Texture, TextureCreator, WindowCanvas,
};
use sdl2::video::WindowContext;
use std::collections::HashMap;
use std::ops::Range;
use std::time::Duration;

const THEME_FADE_DURATION: Duration = Duration::from_millis(1000);
/// how many pieces an arriving attack ball shatters into over the board it hit
const ARRIVAL_SHARDS: usize = 20;
/// moving to another game of a playlist fades more gently
pub const GAME_SWITCH_FADE_DURATION: Duration = Duration::from_millis(1800);

/// themes of the same board size and shape are laid out together
fn board_of(theme: &Theme) -> (u32, u32) {
    (theme.geometry().columns(), theme.geometry().visible_rows())
}

/// each theme's layout, and its place within that layout's group
fn board_layouts(
    all_themes: &[Theme],
    players: u32,
    window_size: (u32, u32),
    video_config: VideoConfig,
) -> Vec<(BoardLayout, usize)> {
    let mut groups: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, theme) in all_themes.iter().enumerate() {
        groups.entry(board_of(theme)).or_default().push(index);
    }
    let mut layouts: Vec<Option<(BoardLayout, usize)>> = vec![None; all_themes.len()];
    for members in groups.into_values() {
        let group = members
            .iter()
            .map(|index| &all_themes[*index])
            .collect::<Vec<&Theme>>();
        let layout = BoardLayout::new(&group, players, window_size, video_config);
        for (within, index) in members.into_iter().enumerate() {
            layouts[index] = Some((layout.clone(), within));
        }
    }
    layouts
        .into_iter()
        .map(|layout| layout.expect("every theme is in exactly one group"))
        .collect()
}

pub struct PlayerTextures<'a> {
    pub background: Texture<'a>,
    pub board: Texture<'a>,
}

impl<'a> PlayerTextures<'a> {
    pub fn new(
        texture_creator: &'a TextureCreator<WindowContext>,
        background_size: (u32, u32),
        board_size: (u32, u32),
    ) -> Result<Self, String> {
        let (bg_width, bg_height) = background_size;
        let mut background = texture_creator
            .create_texture_target(RGBA8888, bg_width, bg_height)
            .map_err(|e| e.to_string())?;
        background.set_blend_mode(BlendMode::Blend);

        let (board_width, board_height) = board_size;
        let mut board = texture_creator
            .create_texture_target(RGBA8888, board_width, board_height)
            .map_err(|e| e.to_string())?;
        board.set_blend_mode(BlendMode::Blend);

        Ok(Self { background, board })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureMode {
    Background(u32),
    Board(u32),
}

#[derive(Clone, Debug)]
struct ThemedPlayer {
    bg_snip: Rect,
    board_snip: Rect,
    game_snip: Rect,
    animations: PlayerAnimations,
}

impl ThemedPlayer {
    /// `game_snip` is where the group agreed to put the playfield: the background hangs off
    /// it by whatever margins this theme's art has around its own board.
    pub fn new(player: u32, theme: &Theme, scale: Scale, game_snip: Rect) -> Self {
        let (theme_width, theme_height) = theme.background_size();
        let playfield = theme.playfield_snip();
        let bg_snip = Rect::new(
            game_snip.x() - scale.scale_coordinate(playfield.x()),
            game_snip.y() - scale.scale_coordinate(playfield.y()),
            scale.scale_length(theme_width),
            scale.scale_length(theme_height),
        );
        let board_snip = scale.scale_and_offset_rect(theme.board_snip(), bg_snip.x(), bg_snip.y());
        let animations = PlayerAnimations::new(player, theme.animation_meta());
        Self {
            bg_snip,
            board_snip,
            game_snip,
            animations,
        }
    }

    pub fn update_animations(&mut self, delta: Duration) -> Vec<AnimationEvent> {
        self.animations.update(delta)
    }
}

pub struct ScaledTheme<'a> {
    theme: &'a Theme<'a>,
    bg_source_snip: Rect,
    board_source_snip: Rect,
    player_themes: Vec<ThemedPlayer>,
    scale: Scale,
}

impl<'a> ScaledTheme<'a> {
    fn new(
        theme: &'a Theme<'a>,
        index: usize,
        players: u32,
        window_size: (u32, u32),
        layout: &BoardLayout,
    ) -> Self {
        let scale = Scale::new(
            players,
            window_size,
            theme.geometry().block_size(),
            layout.scale(index, theme),
        );
        let (theme_width, theme_height) = theme.background_size();
        let bg_source_snip = Rect::new(0, 0, theme_width, theme_height);
        let board_rect = theme.board_snip();
        let board_source_snip = Rect::new(0, 0, board_rect.width(), board_rect.height());
        let player_themes = (0..players)
            .map(|pid| ThemedPlayer::new(pid, theme, scale, layout.playfield(index, theme, pid)))
            .collect::<Vec<ThemedPlayer>>();
        Self {
            theme,
            bg_source_snip,
            board_source_snip,
            player_themes,
            scale,
        }
    }

    pub fn update_animations(&mut self, delta: Duration) -> Vec<AnimationEvent> {
        self.player_themes
            .iter_mut()
            .flat_map(|p| p.update_animations(delta))
            .collect()
    }

    pub fn animations_mut(&mut self, player: u32) -> &mut PlayerAnimations {
        &mut self
            .player_themes
            .get_mut(player as usize)
            .unwrap()
            .animations
    }

    pub fn is_pause_required_for_animation(&self, player: u32) -> bool {
        self.player_themes[player as usize].animations.blocks_tick()
    }

    /// how far above the board an attack starts is this theme's geometry, not the game's:
    /// the same cells fall from the top of whichever board the player is looking at
    pub fn animate_nuisance(&mut self, player: u32, cells: &[PlacedCell], fall: NuisanceFall) {
        let hidden_rows = self.theme.geometry().hidden_rows();
        self.animations_mut(player)
            .nuisance_mut()
            .drop_in(cells, hidden_rows, fall);
    }
}

/// The themes one player may use: a range of indices into the context's theme list, so a
/// player on Dr. Rustario cycles Dr. Rustario themes while another cycles Tetris themes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerThemes {
    pub range: Range<usize>,
    pub initial: usize,
}

impl PlayerThemes {
    pub fn new(range: Range<usize>, initial: usize) -> Self {
        assert!(
            range.contains(&initial),
            "initial theme is outside the player's range"
        );
        Self { range, initial }
    }
}

pub struct ThemeContext<'a> {
    /// the current theme index of each player
    current: Vec<usize>,
    /// the themes each player may use
    ranges: Vec<Range<usize>>,
    themes: Vec<ScaledTheme<'a>>,
    fade_buffer: Texture<'a>,
    /// per-player theme fade timer
    /// per-player (elapsed, total) theme fade timers
    fades: Vec<Option<(Duration, Duration)>>,
    /// the player whose theme music is playing
    music_player: u32,
    /// the theme index whose music is playing
    music_theme: Option<usize>,
    /// which of that theme's tracks the match asked for
    /// dealing a random track is the only thing this rolls for, and it is shared with
    /// nothing: no game reads it and no replay depends on it
    music_rng: ThreadRng,
    window_size: (u32, u32),
    /// the attacks crossing the window, which belong to no one player
    attack_balls: AttackBallAnimation,
    /// the middle and colour of each player's last clear, which is where a ball leaves from
    last_clear: Vec<Option<((f64, f64), CellId)>>,
}

/// Which face a player is dealt out of a cast of `cast`, off one seed.
///
/// The step is at least one and less than the cast, so consecutive players can never land on
/// the same face - which is the property that matters, since a two player match must not put
/// the same character on both panels. A theme with a cast of one deals it to everybody, which
/// is what asking a one-character theme for a deal means.
fn deal_index(seed: u64, player: u32, cast: usize) -> usize {
    if cast <= 1 {
        return 0;
    }
    let first = (seed % cast as u64) as usize;
    let step = 1 + ((seed / cast as u64) % (cast as u64 - 1)) as usize;
    (first + player as usize * step) % cast
}

impl<'a> ThemeContext<'a> {
    /// one [`PlayerThemes`] per player, indexing into `all_themes`
    pub fn new(
        all_themes: &'a [Theme<'a>],
        texture_creator: &'a TextureCreator<WindowContext>,
        player_themes: Vec<PlayerThemes>,
        window_size: (u32, u32),
        video_config: VideoConfig,
    ) -> Result<Self, String> {
        let (window_width, window_height) = window_size;

        let mut fade_buffer = texture_creator
            .create_texture_target(RGBA8888, window_width, window_height)
            .map_err(|e| e.to_string())?;
        fade_buffer.set_blend_mode(BlendMode::Blend);
        let players = player_themes.len();

        // themes of the same board share a layout, so a player switching theme mid-game keeps
        // the same board in the same place. Different boards (a bottle and a well) cannot.
        let layouts = board_layouts(all_themes, players as u32, window_size, video_config);

        Ok(Self {
            current: player_themes.iter().map(|p| p.initial).collect(),
            ranges: player_themes.into_iter().map(|p| p.range).collect(),
            themes: all_themes
                .iter()
                .zip(layouts.iter())
                .map(|(theme, (layout, within))| {
                    ScaledTheme::new(theme, *within, players as u32, window_size, layout)
                })
                .collect(),
            fade_buffer,
            fades: vec![None; players],
            music_player: 0,
            music_theme: None,
            music_rng: rng(),
            window_size,
            attack_balls: AttackBallAnimation::new(),
            last_clear: vec![None; players],
        })
    }

    pub fn max_background_size(&self) -> (u32, u32) {
        let sizes = self
            .themes
            .iter()
            .map(|theme| theme.theme.background_size());
        let width = sizes.clone().map(|(w, _)| w).max().unwrap();
        let height = sizes.clone().map(|(_, h)| h).max().unwrap();
        (width, height)
    }

    /// how many themes a player cycles through
    pub fn theme_count(&self, player: u32) -> usize {
        self.ranges[player as usize].len()
    }

    pub fn max_board_size(&self) -> (u32, u32) {
        let rects = self.themes.iter().map(|theme| theme.theme.board_snip());
        let width = rects.clone().map(|r| r.width()).max().unwrap();
        let height = rects.clone().map(|r| r.height()).max().unwrap();
        (width, height)
    }

    pub fn players(&self) -> u32 {
        self.current.len() as u32
    }

    /// the theme a player is currently on
    pub fn theme(&self, player: u32) -> &Theme<'a> {
        self.themes[self.current[player as usize]].theme
    }

    pub fn current(&self, player: u32) -> &ScaledTheme<'a> {
        &self.themes[self.current[player as usize]]
    }

    /// the audio of the theme whose music is playing: the theme of the winning player
    pub fn music_audio(&self) -> &AudioTheme {
        let index = self
            .music_theme
            .unwrap_or(self.current[self.music_player as usize]);
        self.themes[index].theme.audio()
    }

    /// which of the context's themes a player is on right now, so the particle field can name
    /// the sprites of the theme they are actually playing
    pub fn current_theme_index(&self, player: u32) -> usize {
        self.current[player as usize]
    }

    /// the colours a player radiates into the background particle field: their theme's own,
    /// or - for a retro theme, which has none - those of the first theme of their game that
    /// does. Driven by the game they are playing, not by the theme they are looking at.
    pub fn player_palette(&self, player: u32) -> Palette {
        let current = self.current_theme_index(player);
        let own = self.themes[current].theme.particle_palette();
        if !own.is_empty() {
            return Palette::from_sdl(own);
        }
        self.ranges[player as usize]
            .clone()
            .map(|index| self.themes[index].theme.particle_palette())
            .find(|palette| !palette.is_empty())
            .map(Palette::from_sdl)
            .unwrap_or_default()
    }

    pub fn player_board_snip(&self, player: u32) -> Rect {
        self.current(player).player_themes[player as usize].game_snip
    }

    pub fn player_animations(&self, player: u32) -> &PlayerAnimations {
        &self.current(player).player_themes[player as usize].animations
    }

    pub fn is_pause_required_for_animation(&self, player: u32) -> bool {
        self.current(player).is_pause_required_for_animation(player)
    }

    /// whether a player's animations have stopped their sprint clock, see
    /// [`PlayerAnimations::stops_clock`]
    pub fn stops_clock(&self, player: u32) -> bool {
        self.player_animations(player).stops_clock()
    }

    pub fn update_animations(&mut self, delta: Duration) -> Vec<AnimationEvent> {
        self.attack_balls.update(delta);
        // a ball that has arrived shatters over the board it hit, in its own colour
        for flight in self.attack_balls.arrived().to_vec() {
            let columns = self.current(flight.to_player).theme.geometry().columns();
            let hidden = self
                .current(flight.to_player)
                .theme
                .geometry()
                .hidden_rows();
            for theme in self.themes.iter_mut() {
                theme.animations_mut(flight.to_player).tray_mut().arrive();
                theme
                    .animations_mut(flight.to_player)
                    .debris_mut()
                    .burst(BurstSpec {
                        spread: Spread::AllDirections,
                        // it shatters over the top row and the pieces drop back onto the
                        // board rather than sailing off it: they are drawn on the window, so
                        // one thrown much harder than this ends up out in the bare scene
                        speed: (2.0, 5.0),
                        gravity: 22.0,
                        life: Duration::from_millis(320),
                        // the same size the pop throws: a theme's droplet is cut small and
                        // centred in a whole cell, so this is the cell's size and the piece
                        // inside it comes out at about half of it
                        size: 0.8,
                        ..BurstSpec::burst(
                            (columns as f64 / 2.0, hidden as f64),
                            ARRIVAL_SHARDS,
                            // the theme's own droplet where it cut one, and the whole cell
                            // where it did not - a burst never wants art of its own
                            DebrisArt::Debris(flight.cell),
                        )
                    });
            }
        }
        let mut events = vec![];
        for (id, theme) in self.themes.iter_mut().enumerate() {
            for event in theme.update_animations(delta).into_iter() {
                // only emit from the theme the player is currently on
                let AnimationEvent::Finished { player, .. } = event;
                if self.current[player as usize] == id {
                    events.push(event);
                }
            }
        }
        events
    }

    pub fn animate_destroy(&mut self, player: u32, cells: &[PlacedCell]) {
        // remembered because an attack is routed *after* the chain that earned it has
        // finished, by which time the group that paid for it is off the board - and the ball
        // has to leave from where it was, in the colour it was
        if let Some(clear) = crate::animate::centre_and_modal(cells) {
            self.last_clear[player as usize] = Some(clear);
        }
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .destroy_mut()
                .add(cells.to_vec());
        }
    }

    /// Throw a ball from the group `from` last cleared to `to`'s board.
    ///
    /// One per attack route, which is one per attack. A player who has cleared nothing has
    /// nothing to throw from - an attack can only ever follow a clear - so this is the one
    /// case where nothing is drawn.
    pub fn send_attack_ball(&mut self, from: u32, to: u32, held: usize, strength: u32) {
        let Some((at, cell)) = self.last_clear.get(from as usize).copied().flatten() else {
            return;
        };
        self.attack_balls.send(from, at, to, cell, strength);
        // ... and the receiver's tray holds back whatever it has just been given until the
        // ball carrying it lands, or the icons appear a third of a second before it does
        for theme in self.themes.iter_mut() {
            theme.animations_mut(to).tray_mut().expect(held);
        }
    }

    pub fn attack_balls(&self) -> &AttackBallAnimation {
        &self.attack_balls
    }

    /// say `text` over the middle of `cells`, on whichever theme the player is on when it
    /// is drawn
    pub fn animate_popup(&mut self, player: u32, text: String, cells: &[PlacedCell]) {
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .popup_mut()
                .add(text.clone(), cells);
        }
    }

    pub fn animate_impact(&mut self, player: u32) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).impact_mut().impact();
        }
    }

    pub fn animate_lock(&mut self, player: u32, cells: &[PlacedCell]) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).lock_mut().lock(cells);
        }
    }

    /// a cell came to rest, and every theme that has a squash for it plays one
    pub fn animate_landed(&mut self, player: u32, cells: &[PlacedCell]) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).bounce_mut().land(cells);
        }
    }

    pub fn animate_hard_drop(&mut self, player: u32, cells: &[PlacedCell], dropped_rows: u32) {
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .hard_drop_mut()
                .hard_drop(cells, dropped_rows);
        }
    }

    pub fn animate_spawn(&mut self, player: u32, piece: PieceId, is_hold: bool) {
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .spawn_mut()
                .spawn(piece, is_hold);
        }
    }

    /// Deal a player their character, on every theme in the group.
    ///
    /// Each theme picks out of its **own** cast with its own size, off one seed, so a theme
    /// whose cast is a different length needs no coordination - and a playlist swapping a board
    /// onto this game mid-match hands the player back the face they already had. The two
    /// players of a two player match are never dealt the same one.
    ///
    /// `mirrored` is what makes a character face the other player's board: the cast is drawn
    /// facing left or head on, so the player on the *left* of the window is the one flipped.
    pub fn deal_characters(&mut self, seed: u64, players: u32) {
        for theme in self.themes.iter_mut() {
            for player in 0..players {
                let Some((set, _)) = theme.theme.characters.as_ref() else {
                    continue;
                };
                let cast = set.len();
                if cast == 0 {
                    continue;
                }
                let index = deal_index(seed, player, cast);
                let Some(meta) = set.meta(index) else {
                    continue;
                };
                // built here rather than on the first draw, so the first frame of a match is
                // not the one that pays for the texture
                let _ = set.ensure_built(index);
                let mirrored = players > 1 && player < players / 2;
                theme
                    .animations_mut(player)
                    .character_mut()
                    .deal(meta, index, mirrored);
            }
        }
    }

    /// Deal one player one named character, rather than letting the seed choose.
    ///
    /// For tests and for `character_shot`, which walks the whole cast: a seed cannot be asked
    /// for a particular face.
    pub fn deal_character(&mut self, player: u32, character: usize, mirrored: bool) {
        for theme in self.themes.iter_mut() {
            let Some((set, _)) = theme.theme.characters.as_ref() else {
                continue;
            };
            let Some(meta) = set.meta(character) else {
                continue;
            };
            let _ = set.ensure_built(character);
            theme
                .animations_mut(player)
                .character_mut()
                .deal(meta, character, mirrored);
        }
    }

    /// Where a rect of the theme's own source pixels lands in the window for a player.
    ///
    /// The panel is drawn into a texture and composited, so a caller that wants to look at one
    /// piece of furniture - `character_shot` cropping the mugshot box - has to be told where
    /// that texture ended up.
    pub fn player_source_rect(&self, player: u32, rect: Rect) -> Rect {
        let themed = &self.current(player).player_themes[player as usize];
        self.current(player).scale.scale_and_offset_rect(
            rect,
            themed.bg_snip.x(),
            themed.bg_snip.y(),
        )
    }

    /// how many faces the theme a player is on has, so a caller can walk the cast
    pub fn character_count(&self, player: u32) -> usize {
        self.current(player)
            .theme
            .characters
            .as_ref()
            .map(|(set, _)| set.len())
            .unwrap_or(0)
    }

    /// what the theme calls the character a player was dealt
    pub fn character_name(&self, player: u32) -> Option<&'static str> {
        let (set, _) = self.current(player).theme.characters.as_ref()?;
        set.name(self.player_animations(player).character().character()?)
    }

    /// A clear that chained. One pop is not a reaction: it is most clears, several a minute,
    /// and it sends nothing either - so only a clear the game itself called a combo counts.
    pub fn animate_character_chain(&mut self, player: u32) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).character_mut().chained();
        }
    }

    /// The two numbers with no event between them, read every frame: how high this player's
    /// stack is and whether anything is waiting in their tray.
    pub fn character_danger(&mut self, player: u32, danger: f64, pending: bool) {
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .character_mut()
                .danger(danger, pending);
        }
    }

    pub fn animate_game_over(&mut self, player: u32) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).game_over_mut().game_over();
            theme.animations_mut(player).character_mut().game_over();
        }
    }

    pub fn animate_victory(&mut self, player: u32) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).victory_mut().victory();
            theme.animations_mut(player).character_mut().victory();
        }
    }

    pub fn animate_interstitial(&mut self, player: u32) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).interstitial_mut().display();
        }
    }

    /// an attack that waited in the tray falls in from over the top of the board, at
    /// `rows_per_second`, and holds the game while it does
    pub fn animate_nuisance(&mut self, player: u32, cells: &[PlacedCell], fall: NuisanceFall) {
        for theme in self.themes.iter_mut() {
            theme.animate_nuisance(player, cells, fall);
        }
    }

    pub fn animate_next_stage(&mut self, player: u32, cells: &[PlacedCell]) {
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .next_stage_mut()
                .next_stage(cells);
        }
    }

    pub fn maybe_dismiss_interstitial(&mut self, player: u32) -> bool {
        let mut result = false;
        for index in 0..self.themes.len() {
            let theme_result = self.themes[index]
                .animations_mut(player)
                .interstitial_mut()
                .dismiss();
            if index == self.current[player as usize] {
                result = theme_result;
            }
        }
        result
    }

    pub fn is_animating_interstitial(&self) -> bool {
        (0..self.players()).any(|player| {
            self.player_animations(player)
                .interstitial()
                .state()
                .is_some()
        })
    }

    pub fn maybe_dismiss_game_over(&mut self) {
        for theme in self.themes.iter_mut() {
            for player in theme.player_themes.iter_mut() {
                player.animations.game_over_mut().dismiss();
                player.animations.victory_mut().dismiss();
            }
        }
    }

    pub fn is_any_game_over_dismissed(&self) -> bool {
        (0..self.players()).any(|player| {
            self.player_animations(player)
                .game_over()
                .state()
                .map(|s| s.is_dismissed())
                .unwrap_or(false)
        })
    }

    pub fn is_all_post_game_animation_complete(&self) -> bool {
        for player in 0..self.players() {
            let animations = self.player_animations(player);
            if let Some(game_over) = animations.game_over().state() {
                if !game_over.is_complete() {
                    return false;
                }
            }

            if let Some(victory) = animations.victory().state() {
                if !victory.is_complete() {
                    return false;
                }
            }
        }
        true
    }

    /// advance a single player to their next theme, cross-fading only their side of the screen
    ///
    /// A player with one theme to their name has nowhere to go, and fading is a second of
    /// the board dissolving into itself - so that player is left alone. A game with a single
    /// theme still runs on `ThemeMode::All`, and its stage boundaries ask for the next one.
    pub fn fade_into_next_theme(
        &mut self,
        player: u32,
        canvas: &mut WindowCanvas,
        frame: &Texture,
    ) -> Result<(), String> {
        let index = player as usize;
        if self.ranges[index].len() < 2 {
            return Ok(());
        }
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).reset();
        }
        let range = &self.ranges[index];
        let next = self.current[index] + 1;
        self.current[index] = if range.contains(&next) {
            next
        } else {
            range.start
        };
        self.start_fade(player, canvas, frame)
    }

    /// move a player onto a different set of themes (the next game of a playlist), fading
    /// their side of the screen
    pub fn switch_player_themes(
        &mut self,
        player: u32,
        themes: PlayerThemes,
        canvas: &mut WindowCanvas,
        frame: &Texture,
    ) -> Result<(), String> {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).reset();
        }
        let index = player as usize;
        self.ranges[index] = themes.range;
        self.current[index] = themes.initial;
        self.start_fade_for(player, canvas, frame, GAME_SWITCH_FADE_DURATION)
    }

    pub fn fade_all_into_next_theme(
        &mut self,
        canvas: &mut WindowCanvas,
        frame: &Texture,
    ) -> Result<(), String> {
        for player in 0..self.players() {
            self.fade_into_next_theme(player, canvas, frame)?;
        }
        Ok(())
    }

    /// keep the music on the theme of the winning player. the leader is only re-evaluated when
    /// `reevaluate_leader` is set (between stages), otherwise only the theme itself is checked
    /// i.e. the music owner changed theme. returns true if the music was (re)started.
    pub fn sync_music(
        &mut self,
        leader: Option<u32>,
        state: MatchState,
        is_single_player: bool,
    ) -> Result<bool, String> {
        if let Some(leader) = leader {
            self.music_player = leader;
        }
        let wanted = self.current[self.music_player as usize];
        if self.music_theme == Some(wanted) {
            return Ok(false);
        }
        self.music_theme = Some(wanted);

        let audio = self.themes[wanted].theme.audio();
        // the one place a random track is dealt: this is reached only when the theme the
        // music belongs to has changed, so a match keeps the track it opened on through a
        // pause, a stage clear and a game over, and picks another when the theme moves
        audio.deal_game_music(&mut self.music_rng);
        match state {
            // Only single player uses next-stage *music*; in multiplayer the stage clear is a
            // jingle and game music must keep playing, otherwise another player's still-open
            // interstitial would swap in a play-once track and leave the match silent.
            MatchState::Normal if is_single_player && self.is_animating_interstitial() => {
                audio.play_next_stage_music()?
            }
            MatchState::Normal => audio.play_game_music()?,
            MatchState::Paused => {
                audio.play_game_music()?;
                audio.pause_music()?
            }
            MatchState::GameOver { .. } => {
                if is_single_player {
                    audio.play_game_over_music()?
                } else {
                    audio.play_victory_music()?
                }
            }
        }
        Ok(true)
    }

    /// the vertical strip of the window belonging to a player
    pub fn player_clip(&self, player: u32) -> Rect {
        self.current(player).scale.player_clip(player)
    }

    fn start_fade(
        &mut self,
        player: u32,
        canvas: &mut WindowCanvas,
        frame: &Texture,
    ) -> Result<(), String> {
        self.start_fade_for(player, canvas, frame, THEME_FADE_DURATION)
    }

    fn start_fade_for(
        &mut self,
        player: u32,
        canvas: &mut WindowCanvas,
        frame: &Texture,
        total: Duration,
    ) -> Result<(), String> {
        self.fades[player as usize] = Some((Duration::ZERO, total));

        // snapshot the outgoing frame from the frame texture (never the backbuffer, whose
        // content is undefined after a present under WebGL), and only this player's side
        // so another player's in-progress fade is untouched
        let clip = self.player_clip(player);
        let mut result = Ok(());
        canvas
            .with_texture_canvas(&mut self.fade_buffer, |c| {
                result = c.copy(frame, clip, clip);
            })
            .map_err(|e| e.to_string())?;
        result.map_err(|e| e.to_string())
    }

    pub fn is_fading(&self, player: u32) -> bool {
        self.fades[player as usize].is_some()
    }

    /// draw each player's scene backdrop as if it filled the whole window, clipped to their side
    pub fn draw_scene<G: Game>(
        &self,
        canvas: &mut WindowCanvas,
        games: &[&G],
    ) -> Result<(), String> {
        for player in 0..self.players() {
            let current = self.current(player);
            let speed = games[player as usize].speed_index();
            canvas.set_clip_rect(self.player_clip(player));
            current.theme.scene(speed).draw(canvas, &current.scale)?;
        }
        canvas.set_clip_rect(None);
        Ok(())
    }

    pub fn draw_players(
        &mut self,
        canvas: &mut WindowCanvas,
        texture_refs: &mut [(&mut Texture, TextureMode)],
        delta: Duration,
    ) -> Result<(), String> {
        for (texture, texture_mode) in texture_refs.iter_mut() {
            let (TextureMode::Background(pid) | TextureMode::Board(pid)) = texture_mode;
            // retro pixel art scales up by whole pixels, so keep its hard edges; a Native
            // (modern) theme is built at 1-player size and drawn smaller when the window is
            // shared, and nearest sampling breaks up its anti-aliased text
            texture.set_scale_mode(match self.theme(*pid).scale_mode() {
                ScaleMode::Source => TextureScaleMode::Nearest,
                ScaleMode::Native => TextureScaleMode::Linear,
            });
            match texture_mode {
                TextureMode::Background(pid) => {
                    let current = self.current(*pid);
                    let player = &current.player_themes[*pid as usize];
                    canvas.copy(texture, current.bg_source_snip, player.bg_snip)?;
                }
                TextureMode::Board(pid) => {
                    let current = self.current(*pid);
                    let player = &current.player_themes[*pid as usize];
                    // the panel's shadow goes on the scene first: the board is the first
                    // thing composited for a player and the panel is laid over it, so this
                    // is the one moment both of them are still to come.
                    //
                    // It is cast from the *panel*, and so it does not move with the impact
                    // below: a hard drop jolts the board inside a panel that stays where it
                    // is, which is what every retro theme here has always done, so a shadow
                    // that shook with it would be a shadow of something that had not moved
                    if let Some(shadow) = current.theme.shadow() {
                        shadow.draw(canvas, player.bg_snip, &current.scale)?;
                    }
                    let (offset_x, offset_y) = player.animations.impact().current_offset();
                    let dst = current.scale.offset_proportional_to_block_size(
                        player.board_snip,
                        offset_x,
                        offset_y,
                    );
                    canvas.copy(texture, current.board_source_snip, dst)?;
                }
            }
        }

        // fade out the previous theme on each side that is changing
        for player in 0..self.players() {
            let Some((duration, total)) = self.fades[player as usize] else {
                continue;
            };
            let duration = duration + delta;
            if duration > total {
                self.fades[player as usize] = None;
            } else {
                let alpha = 255.0 * duration.as_millis() as f64 / total.as_millis() as f64;
                self.fade_buffer.set_alpha_mod(255 - alpha as u8);
                let clip = self.player_clip(player);
                canvas.copy(&self.fade_buffer, clip, clip)?;
                self.fades[player as usize] = Some((duration, total));
            }
        }

        Ok(())
    }

    /// Every player's captions, on the window itself and over everything else.
    ///
    /// The board is drawn into a texture and composited, and the foreground particles go on
    /// top of that - so a caption drawn with the board is under the very burst it is about.
    /// This is called last instead, after the particles.
    /// Every player's debris, on the window between the foreground particles and the
    /// captions - so a burst is over the particles and a caption is over the burst.
    ///
    /// Clipped to the player, because a droplet may travel a long way and has no business in
    /// the other player's half.
    pub fn draw_debris(&self, canvas: &mut WindowCanvas) -> Result<(), String> {
        for player in 0..self.players() {
            let current = self.current(player);
            let themed = &current.player_themes[player as usize];
            if themed.animations.debris().pieces().is_empty() {
                continue;
            }
            let (offset_x, offset_y) = themed.animations.impact().current_offset();
            let board = current.scale.offset_proportional_to_block_size(
                themed.board_snip,
                offset_x,
                offset_y,
            );
            canvas.set_clip_rect(current.scale.player_clip(player));
            let result = current.theme.draw_debris(
                canvas,
                &themed.animations,
                &current.scale,
                Point::new(board.x(), board.y()),
            );
            canvas.set_clip_rect(None);
            result?;
        }
        Ok(())
    }

    /// Everything a character has thrown, on the window and clipped to its own player.
    ///
    /// Anchored on the **panel** rather than the board, since the box a character stands in is
    /// panel furniture - which is the only thing this does not share with `draw_debris`. It is
    /// drawn after it, so a spark crosses a droplet rather than the other way about.
    pub fn draw_character_particles(&self, canvas: &mut WindowCanvas) -> Result<(), String> {
        for player in 0..self.players() {
            let current = self.current(player);
            let themed = &current.player_themes[player as usize];
            if themed.animations.character().particles().is_empty() {
                continue;
            }
            canvas.set_clip_rect(current.scale.player_clip(player));
            let result = current.theme.draw_character_particles(
                canvas,
                &themed.animations,
                &current.scale,
                Point::new(themed.bg_snip.x(), themed.bg_snip.y()),
            );
            canvas.set_clip_rect(None);
            result?;
        }
        Ok(())
    }

    /// Every attack in the air, on the window and **unclipped** - it is the one thing here
    /// that crosses between two players, so it belongs to neither one's area.
    ///
    /// Both ends are resolved through whichever theme each player is on right now, so a
    /// theme change mid-flight moves them rather than leaving the ball flying to where a
    /// board used to be.
    pub fn draw_attack_balls(&self, canvas: &mut WindowCanvas) -> Result<(), String> {
        if self.attack_balls.is_empty() {
            return Ok(());
        }
        canvas.set_clip_rect(None);
        for flight in self.attack_balls.flights() {
            let from = self.cell_in_window(flight.from_player, flight.from_cell);
            // ... to just above the top of the board it is going to, which is where the tray
            // is on the theme this was built for
            let to_theme = self.current(flight.to_player);
            let to_columns = to_theme.theme.geometry().columns() as f64;
            let to_hidden = to_theme.theme.geometry().hidden_rows() as f64;
            let to = self.cell_in_window(flight.to_player, (to_columns / 2.0, to_hidden - 1.0));

            let (x, y) = flight.at(from, to, self.window_size.1);
            let block = to_theme
                .scale
                .scale_length(to_theme.theme.geometry().block_size());
            // the sender's theme owns the ball, since it is the sender's own art and palette
            let sender = self.current(flight.from_player).theme;
            let full = block as f64 * sender.attack_ball_scale();
            let size = (full * flight.scale()).round().max(1.0) as u32;
            let dest = Rect::new(
                x.round() as i32 - size as i32 / 2,
                y.round() as i32 - size as i32 / 2,
                size,
                size,
            );
            if !sender.draw_attack_ball(canvas, flight.from_player, flight.strength, dest)? {
                // no ball art: the popped colour's own cell, and a white core over it, which
                // is the nearest a theme with nothing cut can get
                sender.draw_loose_cell(canvas, flight.cell, dest)?;
                let core = flight.core();
                if core > 0.0 {
                    let core_size = (size as f64 * core * 0.6).round().max(1.0) as u32;
                    canvas.set_blend_mode(BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(255, 255, 255, (core * 255.0) as u8));
                    canvas.fill_rect(Rect::from_center(dest.center(), core_size, core_size))?;
                }
            }
        }
        Ok(())
    }

    /// where a point in a player's board cells falls in the window, in pixels
    fn cell_in_window(&self, player: u32, cell: (f64, f64)) -> (f64, f64) {
        let current = self.current(player);
        let themed = &current.player_themes[player as usize];
        let geometry = current.theme.geometry();
        let origin = geometry.point(CellPoint::new(0, geometry.hidden_rows() as i32));
        let block = geometry.block_size() as f64;
        let at = Point::new(
            origin.x() + ((cell.0 + 0.5) * block).round() as i32,
            origin.y() + ((cell.1 - geometry.hidden_rows() as f64 + 0.5) * block).round() as i32,
        );
        let mapped =
            current
                .scale
                .scale_and_offset_point(at, themed.board_snip.x(), themed.board_snip.y());
        (mapped.x() as f64, mapped.y() as f64)
    }

    pub fn draw_popups(&self, canvas: &mut WindowCanvas) -> Result<(), String> {
        for player in 0..self.players() {
            let current = self.current(player);
            let themed = &current.player_themes[player as usize];
            // the board shakes on an impact; a caption over it shakes with it
            let (offset_x, offset_y) = themed.animations.impact().current_offset();
            let board = current.scale.offset_proportional_to_block_size(
                themed.board_snip,
                offset_x,
                offset_y,
            );
            current.theme.draw_popups(
                canvas,
                &themed.animations,
                &current.scale,
                Point::new(board.x(), board.y()),
            )?;
        }
        Ok(())
    }

    pub fn player_row_snips(&self, player: u32, rows: Vec<u32>) -> Vec<Rect> {
        let theme = self.current(player);
        let player = &theme.player_themes[player as usize];
        let geometry = theme.theme.geometry();
        rows.into_iter()
            .map(|j| geometry.row_snip(j))
            .map(|r| {
                theme
                    .scale
                    .scale_and_offset_rect(r, player.board_snip.x(), player.board_snip.y())
            })
            .collect()
    }

    pub fn player_block_snips(&self, player: u32, points: Vec<CellPoint>) -> Vec<Rect> {
        let theme = self.current(player);
        let player = &theme.player_themes[player as usize];
        let geometry = theme.theme.geometry();
        points
            .into_iter()
            .map(|p| geometry.raw_block(p))
            .map(|r| {
                theme
                    .scale
                    .scale_and_offset_rect(r, player.board_snip.x(), player.board_snip.y())
            })
            .collect()
    }

    pub fn player_block_snips_masked(
        &self,
        player: u32,
        cells: Vec<PlacedCell>,
        lattice_spacing: u32,
    ) -> Vec<Point> {
        let theme = self.current(player);
        let player = &theme.player_themes[player as usize];
        let geometry = theme.theme.geometry();
        let sprites = theme.theme.sprites();

        cells
            .into_iter()
            .flat_map(|(point, id)| match sprites.mask(id) {
                Some(mask) => mask.lattice(geometry.point(point), lattice_spacing),
                None => vec![geometry.point(point)],
            })
            .map(|p| {
                theme
                    .scale
                    .scale_and_offset_point(p, player.board_snip.x(), player.board_snip.y())
            })
            .collect()
    }

    pub fn player_renders_scene_particles(&self, player: u32) -> bool {
        self.theme(player).scene(0).is_particles()
    }

    /// true if any player is on a theme with a particle scene
    pub fn render_scene_particles(&self) -> bool {
        (0..self.players()).any(|player| self.player_renders_scene_particles(player))
    }
}

#[cfg(test)]
mod character_deal_tests {
    use super::deal_index;

    /// Two players must never be handed the same face; a panel showing the same character as
    /// the one opposite it reads as a bug even though nothing is broken.
    #[test]
    fn two_players_are_never_dealt_the_same_character() {
        for cast in 2..=13usize {
            for seed in 0..2000u64 {
                let a = deal_index(seed, 0, cast);
                let b = deal_index(seed, 1, cast);
                assert_ne!(
                    a, b,
                    "seed {seed} deals {a} to both players of a cast of {cast}"
                );
            }
        }
    }

    #[test]
    fn a_deal_is_the_same_every_time_it_is_asked() {
        for seed in 0..200u64 {
            assert_eq!(deal_index(seed, 0, 13), deal_index(seed, 0, 13));
            assert_eq!(deal_index(seed, 1, 13), deal_index(seed, 1, 13));
        }
    }

    #[test]
    fn every_face_of_the_cast_is_reachable() {
        let mut seen = vec![false; 13];
        for seed in 0..500u64 {
            seen[deal_index(seed, 0, 13)] = true;
        }
        assert!(
            seen.iter().all(|s| *s),
            "some faces are never dealt: {seen:?}"
        );
    }

    /// a theme that has only one character hands it to everybody rather than dividing by zero
    #[test]
    fn a_cast_of_one_deals_it_to_everybody() {
        assert_eq!(deal_index(7, 0, 1), 0);
        assert_eq!(deal_index(7, 1, 1), 0);
    }
}
