//! Every theme scaled to the window, with per-player render targets and animations, and the
//! cross-fade when a player changes theme. Every theme keeps animation state for every player so a
//! mid-match theme change is seamless.

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
    /// `game_snip` is where the group put the playfield; the background hangs off it by this
    /// theme's own margins.
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

    /// how far above the board an attack starts is this theme's geometry, not the game's
    pub fn animate_nuisance(&mut self, player: u32, cells: &[PlacedCell], fall: NuisanceFall) {
        let hidden_rows = self.theme.geometry().hidden_rows();
        self.animations_mut(player)
            .nuisance_mut()
            .drop_in(cells, hidden_rows, fall);
    }
}

/// The themes one player may use, as a range of indices into the context's theme list.
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
    /// per-player (elapsed, total) theme fade timers
    fades: Vec<Option<(Duration, Duration)>>,
    /// the player whose theme music is playing
    music_player: u32,
    /// the theme index whose music is playing
    music_theme: Option<usize>,
    /// deals a random music track; nothing else reads it and no replay depends on it
    music_rng: ThreadRng,
    window_size: (u32, u32),
    /// the attacks crossing the window, which belong to no one player
    attack_balls: AttackBallAnimation,
    /// the middle and colour of each player's last clear, which is where a ball leaves from
    last_clear: Vec<Option<((f64, f64), CellId)>>,
}

/// Which face a player is dealt out of a cast of `cast`, off one seed. The step is at least one
/// and less than the cast, so consecutive players never share a face.
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

        // themes of the same board share a layout, so a player switching theme keeps the board in
        // place
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

    /// which of the context's themes a player is on right now
    pub fn current_theme_index(&self, player: u32) -> usize {
        self.current[player as usize]
    }

    /// the colours a player radiates into the background particle field: their theme's own, or for
    /// a retro theme those of the first theme of their game that has some
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
        // a ball that has arrived shatters where it landed, in its own colour
        for flight in self.attack_balls.arrived().to_vec() {
            let at = self.current(flight.to_player).theme.attack_arrival_cell();
            for theme in self.themes.iter_mut() {
                theme.animations_mut(flight.to_player).tray_mut().arrive();
                theme
                    .animations_mut(flight.to_player)
                    .debris_mut()
                    .burst(BurstSpec {
                        spread: Spread::AllDirections,
                        // low enough that the pieces drop back rather than sailing off into the
                        // bare scene
                        speed: (2.0, 5.0),
                        gravity: 22.0,
                        life: Duration::from_millis(320),
                        // the cell's size; a droplet is cut at about half a cell and centred in it
                        size: 0.8,
                        ..BurstSpec::burst(
                            at,
                            ARRIVAL_SHARDS,
                            // the theme's own droplet where it cut one, otherwise the whole cell
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
        // remembered because an attack is routed after its chain has finished and the group is
        // gone, and the ball has to leave from where it was, in its colour
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

    /// Throw a ball from the group `from` last cleared to `to`'s board; nothing is drawn if `from`
    /// has cleared nothing.
    pub fn send_attack_ball(&mut self, from: u32, to: u32, held: usize, strength: u32) {
        let Some((at, cell)) = self.last_clear.get(from as usize).copied().flatten() else {
            return;
        };
        self.attack_balls.send(from, at, to, cell, strength);
        // ... and the receiver's tray holds back what it has just been given until the ball lands
        for theme in self.themes.iter_mut() {
            theme.animations_mut(to).tray_mut().expect(held);
        }
    }

    pub fn attack_balls(&self) -> &AttackBallAnimation {
        &self.attack_balls
    }

    /// say `text` over the middle of `cells`, on whichever theme the player is on when it is drawn
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

    /// Deal a player their character on every theme in the group, each from its own cast off one
    /// seed, so a playlist returning to this game hands back the same face. `mirrored` flips the
    /// player on the left, so each character faces the other player's board.
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
                // built now so the first frame of a match does not pay for the texture
                let _ = set.ensure_built(index);
                let mirrored = players > 1 && player < players / 2;
                theme
                    .animations_mut(player)
                    .character_mut()
                    .deal(meta, index, mirrored);
            }
        }
    }

    /// Deal one player one named character rather than letting the seed choose.
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

    /// Put one player's character into a state and start one named routine of it.
    pub fn play_character_routine(&mut self, player: u32, routine: usize, home: Option<i32>) {
        for theme in self.themes.iter_mut() {
            theme
                .animations_mut(player)
                .character_mut()
                .play(routine, home);
        }
    }

    /// How many routines the character a player was dealt has.
    pub fn character_routines(&self, player: u32) -> usize {
        self.player_animations(player).character().routines()
    }

    /// Whether that character's routine has run out and it is standing between them.
    pub fn character_resting(&self, player: u32) -> bool {
        self.player_animations(player).character().resting()
    }

    /// Where a rect of the theme's own source pixels lands in the window for a player.
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

    /// A clear the game called a combo; a single pop is not a reaction.
    pub fn animate_character_chain(&mut self, player: u32) {
        for theme in self.themes.iter_mut() {
            theme.animations_mut(player).character_mut().chained();
        }
    }

    /// The per-frame danger reading: how high this player's stack is and whether their tray holds
    /// anything.
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

    /// an attack that waited in the tray falls in from over the top of the board at
    /// `rows_per_second`, holding the game while it does
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

    /// Advance a single player to their next theme, cross-fading only their side of the screen. A
    /// player with one theme is left alone.
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

    /// move a player onto a different set of themes (the next game of a playlist), fading their
    /// side of the screen
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

    /// Keep the music on the theme of the winning player; returns true if the music was
    /// (re)started. The leader is only re-evaluated when `reevaluate_leader` is set, between
    /// stages.
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
        // the one place a random track is dealt: reached only when the music's theme changes, so a
        // match keeps its track through a pause, a stage clear and a game over
        audio.deal_game_music(&mut self.music_rng);
        match state {
            // only single player uses next-stage music; in multiplayer another player's
            // interstitial would swap in a play-once track and leave the match silent
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

        // snapshot from the frame texture, never the backbuffer (undefined after a present under
        // WebGL), and only this player's side so another player's fade is untouched
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
            // retro art scales by whole pixels, so keep its hard edges; a Native theme is drawn
            // smaller when the window is shared, and nearest sampling breaks up its anti-aliased
            // text
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
                    // the panel's shadow goes on the scene before the board and panel are
                    // composited over it, and it does not move with the impact, since the panel
                    // does not
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

    /// Every player's debris, on the window between the foreground particles and the captions,
    /// clipped to the player. Also draws the routine characters, so no caller can forget them.
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
        // ... and every character played as routines, over the panel he stands on
        self.draw_placed_characters(canvas)?;
        Ok(())
    }

    /// Every character played as routines, on the window and clipped only to the player's half,
    /// since the game draws him over the stone above the arch.
    fn draw_placed_characters(&self, canvas: &mut WindowCanvas) -> Result<(), String> {
        for player in 0..self.players() {
            let current = self.current(player);
            let themed = &current.player_themes[player as usize];
            let Some((_, layout)) = current.theme.characters.as_ref() else {
                continue;
            };
            let at = self.player_source_rect(player, layout.rect);
            canvas.set_clip_rect(current.scale.player_clip(player));
            let result = current
                .theme
                .draw_character_unclipped(canvas, &themed.animations, at);
            canvas.set_clip_rect(None);
            result?;
        }
        Ok(())
    }

    /// Everything a character has thrown, on the window, clipped to its player and anchored on
    /// the panel. Drawn after `draw_debris`, so a spark crosses a droplet.
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

    /// Every attack in the air, on the window and unclipped, since it crosses between players. Both
    /// ends resolve through each player's current theme, so a theme change mid-flight moves them.
    pub fn draw_attack_balls(&self, canvas: &mut WindowCanvas) -> Result<(), String> {
        if self.attack_balls.is_empty() {
            return Ok(());
        }
        canvas.set_clip_rect(None);
        for flight in self.attack_balls.flights() {
            let from = self.cell_in_window(flight.from_player, flight.from_cell);
            // ... to the tray it is landing in, through the receiver's current theme; a theme with
            // no tray keeps the middle of the board
            let to_theme = self.current(flight.to_player);
            let to = match to_theme.theme.pending_origin() {
                Some(at) => self.background_point_in_window(flight.to_player, at),
                None => {
                    let columns = to_theme.theme.geometry().columns() as f64;
                    let hidden = to_theme.theme.geometry().hidden_rows() as f64;
                    self.cell_in_window(flight.to_player, (columns / 2.0, hidden - 1.0))
                }
            };

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
                // no ball art: the popped colour's own cell with a white core over it
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

    /// A point in a player's theme's own background pixels, on the window, through the same mapping
    /// the background texture is blitted with.
    fn background_point_in_window(&self, player: u32, at: Point) -> (f64, f64) {
        let current = self.current(player);
        let themed = &current.player_themes[player as usize];
        let mapped =
            current
                .scale
                .scale_and_offset_point(at, themed.bg_snip.x(), themed.bg_snip.y());
        (mapped.x() as f64, mapped.y() as f64)
    }

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

    /// Two players are never dealt the same face.
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

    /// A theme with one character hands it to everybody.
    #[test]
    fn a_cast_of_one_deals_it_to_everybody() {
        assert_eq!(deal_index(7, 0, 1), 0);
        assert_eq!(deal_index(7, 1, 1), 0);
    }
}
