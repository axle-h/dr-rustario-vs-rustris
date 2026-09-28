//! One match, ticked frame by frame: input, game updates, animations, events, playlist
//! switches, music and rendering.

use crate::animate::event::{AnimationEvent, AnimationType};
use crate::app::{App, MatchSettings, PostGameAction, StageChange, ThemeMode};
use crate::frame_rate::FrameRate;
use crate::game::geometry::Point as CellPoint;
use crate::game::{ids, Cell, Game, GameEvent, MetricKind, StageState, StageTransition};
use crate::game_input::{GameInputContext, GameInputKey};
use crate::particles::field::context::{PlayerRegion, SceneContext};
use crate::particles::field::reaction::FieldEvent;
use crate::particles::prescribed::PlayerTargetedParticles;
use crate::particles::render::ParticleRender;
use crate::particles::scale::Scale as ParticleScale;
use crate::render::context::{PlayerTextures, TextureMode, ThemeContext};
use crate::render::pause::PausedScreen;
use crate::render::timer::TimerRender;
use crate::render::{GameRender, Theme};
use crate::session::{Match, MatchState};
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGBA8888;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::time::Duration;

/// how long a finished playlist stage stays on screen before the next game fades in
const GAME_SWITCH_HOLD: Duration = Duration::from_millis(800);

pub struct MatchScreen<'a, G: Game + GameRender> {
    inputs: GameInputContext,
    fixture: Match<G>,
    settings: MatchSettings,
    themes: ThemeContext<'a>,
    player_textures: Vec<PlayerTextures<'a>>,
    /// every frame renders here first, so theme fades can snapshot the previous frame without
    /// reading the backbuffer
    frame_buffer: Texture<'a>,
    paused_screen: PausedScreen<'a>,
    timer: Option<TimerRender<'a>>,
    /// they play for the players they name instead of the keyboard
    controllers: Vec<(u32, Controller<G>)>,
    frame_rate: FrameRate,
    /// per player: time since they finished a playlist stage
    pending_switches: Vec<Option<Duration>>,
    /// per player: games set aside during a playlist, resumed where they left off
    parked: Vec<Vec<G>>,
    completed_stages: Vec<u32>,
    players: u32,
    is_single_player: bool,
}

type Controller<G> = Box<dyn FnMut(&mut G, Duration) + 'static>;

impl<'a, G: Game + GameRender> MatchScreen<'a, G> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        texture_creator: &'a TextureCreator<WindowContext>,
        all_themes: &'a [Theme<'a>],
        games: Vec<G>,
        settings: MatchSettings,
        fg_particles: &mut ParticleRender,
        bg_particles: &mut ParticleRender,
        controllers: Vec<(u32, Controller<G>)>,
    ) -> Result<Self, String> {
        let inputs = GameInputContext::new(app.config.input);
        let players = games.len() as u32;
        assert_eq!(settings.players.len(), games.len());
        let theme_counts = settings
            .players
            .iter()
            .map(|p| p.themes.len() as u32)
            .collect::<Vec<u32>>();
        let ai_players = controllers.iter().map(|(p, _)| *p).collect::<Vec<u32>>();
        let fixture = Match::new(
            games,
            settings.rules,
            &theme_counts,
            settings.high_score_key.as_ref(),
        )
        .with_ai_players(ai_players);
        let is_single_player = fixture.is_single_player();
        let window_size = app.canvas.window().size();
        let mut themes = ThemeContext::new(
            all_themes,
            texture_creator,
            settings.players.iter().map(|p| p.player_themes()).collect(),
            window_size,
            app.config.video,
        )?;
        let player_textures = (0..players)
            .map(|_| {
                PlayerTextures::new(
                    texture_creator,
                    themes.max_background_size(),
                    themes.max_board_size(),
                )
                .unwrap()
            })
            .collect::<Vec<PlayerTextures>>();

        let mut frame_buffer = texture_creator
            .create_texture_target(RGBA8888, window_size.0, window_size.1)
            .map_err(|e| e.to_string())?;
        // start black so a fade on the first frame has something to fade from
        app.canvas
            .with_texture_canvas(&mut frame_buffer, |c| {
                c.set_draw_color(Color::BLACK);
                c.clear();
            })
            .map_err(|e| e.to_string())?;

        let paused_screen = PausedScreen::new(&mut app.canvas, texture_creator, window_size)?;
        let timer = if settings.rules.is_sprint() {
            Some(TimerRender::new(
                &mut app.canvas,
                texture_creator,
                window_size,
                players,
            )?)
        } else {
            None
        };

        // dealt once per match, so a playlist swapping games and back keeps each player's face
        themes.deal_characters(rand::random::<u64>(), players);

        for player in 0..players {
            let cells = fixture.player(player).game().stage_intro_cells();
            themes.animate_next_stage(player, &cells);
        }

        fg_particles.clear();
        bg_particles.clear();

        themes.sync_music(fixture.leading_player(), fixture.state(), is_single_player)?;

        Ok(Self {
            inputs,
            fixture,
            settings,
            themes,
            player_textures,
            frame_buffer,
            paused_screen,
            timer,
            controllers,
            frame_rate: FrameRate::new(),
            pending_switches: vec![None; players as usize],
            parked: (0..players).map(|_| vec![]).collect(),
            completed_stages: vec![0; players as usize],
            players,
            is_single_player,
        })
    }

    /// What the particle field is told this frame, or `None` when no player is on a particle
    /// scene. Every player is described, since off-field boards still resolve attacks and events.
    fn scene_context(
        themes: &ThemeContext,
        fixture: &Match<G>,
        scale: &ParticleScale,
        players: u32,
    ) -> Option<SceneContext> {
        SceneContext::with_captions(
            (0..players)
                .map(|player| {
                    let game = fixture.player(player).game();
                    PlayerRegion {
                        player,
                        clip: scale.rect_to_particle_space(themes.player_clip(player)),
                        board: scale.rect_to_particle_space(themes.player_board_snip(player)),
                        theme: themes.current_theme_index(player),
                        game: game.game_id(),
                        palette: themes.player_palette(player),
                        in_canvas: themes.player_renders_scene_particles(player),
                        danger: Self::stack_danger(game),
                        speed_index: game.speed_index(),
                        held_up: themes.is_pause_required_for_animation(player),
                    }
                })
                .collect(),
            Self::captions(themes, fixture, players),
        )
    }

    /// Words the field may morph into unprompted: each player's game and level, and `VS` when
    /// there is an opponent. The engine cannot name a game, so the match screen picks them.
    fn captions(themes: &ThemeContext, fixture: &Match<G>, players: u32) -> Vec<String> {
        let mut captions = vec![];
        for player in 0..players {
            if !themes.player_renders_scene_particles(player) {
                continue;
            }
            let game = fixture.player(player).game();
            captions.push(game.name().to_uppercase());
            if let Some(level) = game.metric(MetricKind::Level) {
                captions.push(format!("LEVEL {level}"));
            }
            if players > 1 {
                captions.push("VS".to_string());
            }
        }
        captions.sort();
        captions.dedup();
        captions
    }

    /// How close a player's stack is to the top of their board, 0-1.
    fn stack_danger(game: &G) -> f64 {
        let rows = game.board_height();
        let visible = game.visible_height().max(1);
        let hidden = rows.saturating_sub(visible);
        for j in hidden..rows {
            let occupied = (0..game.board_width()).any(|i| {
                matches!(
                    game.cell(CellPoint::from_u32(i, j)),
                    Cell::Stack(_) | Cell::Garbage(_)
                )
            });
            if occupied {
                return ((rows - 1 - j) as f64 / visible as f64).clamp(0.0, 1.0);
            }
        }
        0.0
    }

    /// One frame of the match. `next_stage` is asked for a player's next game when they complete
    /// a stage; `None` keeps them on the one they are playing.
    pub fn update(
        &mut self,
        app: &mut App,
        fg_particles: &mut ParticleRender,
        bg_particles: &mut ParticleRender,
        mut next_stage: impl FnMut(u32, u32) -> Option<StageChange<G>>,
    ) -> Result<Option<PostGameAction>, String> {
        let Self {
            inputs,
            fixture,
            settings,
            themes,
            player_textures,
            frame_buffer,
            paused_screen,
            timer,
            controllers,
            frame_rate,
            pending_switches,
            parked,
            completed_stages,
            players,
            is_single_player,
        } = self;
        let players = *players;
        let is_single_player = *is_single_player;

        let delta = frame_rate.update()?;
        fixture.unset_flags();

        let mut to_emit_particles: Vec<PlayerTargetedParticles> = vec![];

        let mut field_events: Vec<FieldEvent> = vec![];

        let mut stage_changed = false;
        // read before routing, since a routed attack is already in the tray when its ball is made
        let trays_before: Vec<usize> = (0..players)
            .map(|p| fixture.player(p).game().pending_attacks().len())
            .collect();

        // tagged with the player that caused them, `None` for match-wide events
        let mut events: Vec<(Option<u32>, GameEvent)> = vec![];

        // a controller dismisses its own stage clear card
        let mut players_to_advance: Vec<u32> = controllers
            .iter()
            .map(|(player, _)| *player)
            .filter(|player| {
                pending_switches[*player as usize].is_none()
                    && themes.is_pause_required_for_animation(*player)
            })
            .collect();
        players_to_advance.retain(|player| themes.maybe_dismiss_interstitial(*player));

        for key in inputs.update(delta, app.controllers.poll(&mut app.event_pump)) {
            if let Some(player) = key.player() {
                if pending_switches[player as usize].is_some() {
                    continue;
                }
                if controllers.iter().any(|(p, _)| *p == player)
                    && !themes.is_pause_required_for_animation(player)
                {
                    // a controller plays this player; keys only dismiss animations
                    continue;
                }
                if themes.is_pause_required_for_animation(player) {
                    if themes.maybe_dismiss_interstitial(player) {
                        players_to_advance.push(player);
                    } else {
                        themes.maybe_dismiss_game_over();
                    }
                    continue;
                }
            }

            match key {
                GameInputKey::MoveLeft { player } => fixture.mut_game(player, |g| g.left()),
                GameInputKey::MoveRight { player } => fixture.mut_game(player, |g| g.right()),
                GameInputKey::SoftDrop { player } => {
                    fixture.mut_game(player, |g| g.set_soft_drop(true))
                }
                GameInputKey::HardDrop { player } => fixture.mut_game(player, |g| g.hard_drop()),
                GameInputKey::RotateClockwise { player } => {
                    fixture.mut_game(player, |g| g.rotate(true))
                }
                GameInputKey::RotateAnticlockwise { player } => {
                    fixture.mut_game(player, |g| g.rotate(false))
                }
                GameInputKey::Hold { player } => fixture.mut_game(player, |g| g.hold()),
                GameInputKey::Pause => {
                    if matches!(fixture.state(), MatchState::Normal | MatchState::Paused) {
                        if let Some(paused) = fixture.toggle_paused() {
                            let event = if paused {
                                GameEvent::Paused
                            } else {
                                GameEvent::UnPaused
                            };
                            events.push((None, event));
                        }
                    } else {
                        return Ok(Some(PostGameAction::ReturnToMenu));
                    }
                }
                GameInputKey::Suspend => {
                    if fixture.state() == MatchState::Normal && fixture.toggle_paused().is_some() {
                        events.push((None, GameEvent::Paused));
                    }
                }
                GameInputKey::ReturnToMenu => return Ok(Some(PostGameAction::ReturnToMenu)),
                GameInputKey::Quit => return Ok(Some(PostGameAction::Quit)),
                GameInputKey::NextTheme => {
                    if settings.rules.allow_manual_theme_change() {
                        events.push((None, GameEvent::NextTheme))
                    }
                }
            }
        }

        for player in players_to_advance {
            let game = fixture.player_mut(player).game_mut();
            game.next_stage()?;
            stage_changed = true;

            let completed = game.completed_stages();
            if let Some(change) = next_stage(player, completed) {
                if let Some(next_game) = change.game {
                    fixture.player_mut(player).replace_game(next_game);
                }
                themes.switch_player_themes(
                    player,
                    change.settings.player_themes(),
                    &mut app.canvas,
                    frame_buffer,
                )?;
                settings.players[player as usize] = change.settings;
                if is_single_player {
                    themes.music_audio().play_game_music()?;
                }
            } else if settings.players[player as usize].theme_mode == ThemeMode::All {
                events.push((Some(player), GameEvent::NextTheme));
            } else if is_single_player {
                // single player was playing next-stage music; multiplayer music never stopped
                themes.music_audio().play_game_music()?;
            }
            let cells = fixture.player(player).game().stage_intro_cells();
            themes.animate_next_stage(player, &cells);
        }

        match fixture.state() {
            MatchState::GameOver {
                high_score: Some(high_score),
            } if themes.is_all_post_game_animation_complete() => {
                return Ok(Some(PostGameAction::NewHighScore(high_score)));
            }
            MatchState::GameOver { high_score } if themes.is_any_game_over_dismissed() => {
                return if let Some(high_score) = high_score {
                    Ok(Some(PostGameAction::NewHighScore(high_score)))
                } else {
                    Ok(Some(PostGameAction::ReturnToMenu))
                };
            }
            MatchState::Normal => {
                for (player, controller) in controllers.iter_mut() {
                    if !themes.is_pause_required_for_animation(*player) {
                        fixture.mut_game(*player, |g| controller(g, delta));
                    }
                }
                // the race clock stops only when every player is held up at once
                let anyone_playing = (0..players).any(|player| {
                    !themes.stops_clock(player)
                        && !themes.is_fading(player)
                        && pending_switches[player as usize].is_none()
                });
                if anyone_playing {
                    fixture.add_play_time(delta);
                }
                for player in fixture.players.iter_mut() {
                    let player_id = player.player();
                    if themes.is_pause_required_for_animation(player_id)
                        || themes.is_fading(player_id)
                        || pending_switches[player_id as usize].is_some()
                    {
                        continue;
                    }

                    let mut skip_update = false;
                    let game = player.game_mut();
                    let mut player_events = vec![];
                    player_events.extend(game.drain_events());
                    for event in player_events.iter() {
                        if let GameEvent::HardDrop {
                            cells,
                            dropped_rows,
                        } = event
                        {
                            themes.animate_hard_drop(player_id, cells, *dropped_rows);
                            skip_update = true;
                        }
                    }

                    if !skip_update {
                        game.update(delta);
                        player_events.extend(game.drain_events());
                    }
                    events.extend(player_events.into_iter().map(|e| (Some(player_id), e)));
                }
            }
            _ => {}
        }

        if !fixture.state().is_paused() {
            for player in 0..players {
                let game = fixture.player(player).game();
                let danger = Self::stack_danger(game);
                let pending = !game.pending_attacks().is_empty();
                themes.character_danger(player, danger, pending);
            }
        }

        if !fixture.state().is_paused() {
            let animation_events = themes.update_animations(delta);
            for AnimationEvent::Finished { player, animation } in animation_events {
                if animation == AnimationType::Spawn {
                    events.push((Some(player), GameEvent::Spawned));
                }
            }
        }

        // a seamless stage change may queue a theme change
        let mut deferred_events: Vec<(Option<u32>, GameEvent)> = vec![];
        for (event_player, event) in events {
            // route an attack before anything reacts to it: one that lands on nobody never happened
            if let GameEvent::AttackSent(attack) = &event {
                if !event_player.is_some_and(|player| fixture.send_attack(player, *attack)) {
                    continue;
                }
            }
            // effects play through the causing player's theme, match-wide ones through the music's
            let (audio, clear_class, clear_word, clear_popup) = match event_player {
                Some(player) => (
                    themes.theme(player).audio(),
                    fixture.player(player).game().clear_class(&event),
                    fixture.player(player).game().clear_word(&event),
                    fixture.player(player).game().clear_popup(&event),
                ),
                None => (themes.music_audio(), 0, None, None),
            };
            audio.receive_event(&event, clear_class)?;
            if let Some(player) = event_player {
                let game = fixture.player(player).game();
                if let Some(emit) = themes
                    .theme(player)
                    .scene(game.speed_index())
                    .emit_particles(player, &event, &game.spawn_cells())
                {
                    to_emit_particles.push(emit);
                }
            }
            match (event_player, event) {
                (Some(player), GameEvent::StageComplete) => {
                    field_events.push(FieldEvent::StageComplete { player });
                    if fixture.next_stage_ends_match(player) {
                        fixture.complete_sprint(player);
                    } else if settings.playlist {
                        // the finished board holds, then the next playlist game fades in
                        pending_switches[player as usize] = Some(Duration::ZERO);
                    } else {
                        match fixture.player(player).game().stage_transition() {
                            StageTransition::Interstitial => {
                                if is_single_player {
                                    themes.music_audio().play_next_stage_music()?;
                                } else {
                                    themes.theme(player).audio().play_stage_complete_jingle()?;
                                }
                                themes.animate_interstitial(player);
                            }
                            StageTransition::Seamless => {
                                fixture.player_mut(player).game_mut().next_stage()?;
                                stage_changed = true;
                                if settings.players[player as usize].theme_mode == ThemeMode::All {
                                    deferred_events.push((Some(player), GameEvent::NextTheme));
                                }
                            }
                        }
                    }
                }
                (Some(player), GameEvent::GameOver) => {
                    field_events.push(FieldEvent::GameOver { player });
                    if is_single_player {
                        themes.animate_game_over(player);
                        fixture.maybe_set_game_over();
                        themes.music_audio().play_game_over_music()?;
                    } else {
                        for maybe_winner in 0..players {
                            if maybe_winner != player {
                                fixture.set_winner(maybe_winner);
                            }
                        }
                    }
                }
                (
                    Some(player),
                    GameEvent::Clear {
                        ref cells,
                        count,
                        is_combo,
                        ..
                    },
                ) => {
                    let mut rows = cells.iter().map(|(p, _)| p.y as u32).collect::<Vec<u32>>();
                    rows.sort();
                    rows.dedup();
                    field_events.push(FieldEvent::Clear {
                        player,
                        rows: themes
                            .player_row_snips(player, rows)
                            .into_iter()
                            .map(|rect| app.particle_scale.rect_to_particle_space(rect))
                            .collect(),
                        class: clear_class,
                        count,
                        is_combo,
                    });
                    // the character reacts only to a chain, not to every pop
                    if is_combo {
                        themes.animate_character_chain(player);
                    }
                    if let Some(word) = clear_word {
                        field_events.push(FieldEvent::Spell { word });
                    }
                    if let Some(text) = clear_popup {
                        themes.animate_popup(player, text, cells);
                    }
                    themes.animate_destroy(player, cells);
                }
                (Some(player), GameEvent::Lock { cells, dropped }) => {
                    if dropped {
                        themes.animate_impact(player);
                    }
                    themes.animate_lock(player, &cells);
                }
                (Some(player), GameEvent::Landed { cells }) => {
                    themes.animate_landed(player, &cells);
                }
                (Some(player), GameEvent::Spawn { piece, is_hold, .. }) => {
                    themes.animate_spawn(player, piece, is_hold);
                }
                (Some(player), GameEvent::SpeedUp) => {
                    field_events.push(FieldEvent::SpeedUp { player });
                }
                (Some(player), GameEvent::AttackReceived { cells }) => {
                    // the rules have already landed the tray's attacks; this is the fall,
                    // holding the board until the last is down
                    if let Some(fall) = fixture.player(player).game().attack_fall() {
                        themes.animate_nuisance(player, &cells, fall);
                    }
                    field_events.push(FieldEvent::AttackReceived { player });
                }
                (Some(player), GameEvent::NextTheme) => {
                    themes.fade_into_next_theme(player, &mut app.canvas, frame_buffer)?
                }
                (None, GameEvent::NextTheme) => {
                    themes.fade_all_into_next_theme(&mut app.canvas, frame_buffer)?
                }
                _ => {}
            }
        }
        for (player, _) in deferred_events {
            if let Some(player) = player {
                themes.fade_into_next_theme(player, &mut app.canvas, frame_buffer)?;
            }
        }

        if let Some(winner) = fixture.check_for_winning_player() {
            if fixture.maybe_set_game_over() {
                stage_changed = true;
                field_events.push(FieldEvent::Victory { player: winner });
                themes.animate_victory(winner);
                if let Some(emit) = themes
                    .theme(winner)
                    .scene(fixture.player(winner).game().speed_index())
                    .emit_particles(winner, &GameEvent::Victory, &[])
                {
                    to_emit_particles.push(emit);
                }
                for pid in 0..players {
                    if pid != winner {
                        themes.animate_game_over(pid);
                    }
                }
                // sync starts the victory music if the music moves theme, otherwise start it here
                if !themes.sync_music(
                    fixture.leading_player(),
                    fixture.state(),
                    is_single_player,
                )? {
                    themes.music_audio().play_victory_music()?;
                }
            }
        }

        if fixture.state().is_normal() {
            for player in 0..players {
                let Some(elapsed) = pending_switches[player as usize] else {
                    continue;
                };
                if themes.is_pause_required_for_animation(player) {
                    // the hold starts after the clear finishes
                    continue;
                }
                let elapsed = elapsed + delta;
                if elapsed < GAME_SWITCH_HOLD {
                    pending_switches[player as usize] = Some(elapsed);
                    continue;
                }
                pending_switches[player as usize] = None;
                stage_changed = true;
                let index = player as usize;
                completed_stages[index] += 1;
                let completed = completed_stages[index];
                if let Some(change) = next_stage(player, completed) {
                    let same_game = change.game.is_none();
                    if let Some(next_game) = change.game {
                        let wanted = next_game.game_id();
                        let resumed = match parked[index].iter().position(|g| g.game_id() == wanted)
                        {
                            Some(i) => parked[index].swap_remove(i),
                            None => next_game,
                        };
                        let outgoing = fixture.player_mut(player).replace_game(resumed);
                        parked[index].push(outgoing);
                    }
                    if same_game && change.settings.theme_mode == ThemeMode::All {
                        themes.fade_into_next_theme(player, &mut app.canvas, frame_buffer)?;
                    } else {
                        themes.switch_player_themes(
                            player,
                            change.settings.player_themes(),
                            &mut app.canvas,
                            frame_buffer,
                        )?;
                    }
                    settings.players[player as usize] = change.settings;
                } else if settings.players[player as usize].theme_mode == ThemeMode::All {
                    themes.fade_into_next_theme(player, &mut app.canvas, frame_buffer)?;
                }
                // a game parked mid-stage-change starts its next stage now
                let game = fixture.player_mut(player).game_mut();
                if game.stage_state() == StageState::StageComplete {
                    game.next_stage()?;
                }
                game.set_completed_stages(completed);
                let cells = fixture.player(player).game().stage_intro_cells();
                themes.animate_next_stage(player, &cells);
            }
        }

        // music follows the winning player, re-evaluated between stages
        themes.sync_music(
            if stage_changed {
                fixture.leading_player()
            } else {
                None
            },
            fixture.state(),
            is_single_player,
        )?;

        // the events only name the attacker, and the field wants both ends of the comet
        for route in fixture.drain_attack_routes() {
            // one ball per attack, drawn only when a Puyo game is in the match
            let puyo = |player: u32| fixture.player(player).game().game_id() == ids::PUYO;
            if puyo(route.from) || puyo(route.to) {
                themes.send_attack_ball(
                    route.from,
                    route.to,
                    trays_before[route.to as usize],
                    route.strength,
                );
            }
            field_events.push(FieldEvent::Attack {
                from: route.from,
                to: route.to,
                strength: route.strength,
            });
        }

        let scene = Self::scene_context(themes, fixture, &app.particle_scale, players);
        if let Some(scene) = scene.as_ref() {
            // F2 can move a player onto a particle scene part way through a match
            if !bg_particles.has_field() {
                bg_particles.add_field(scene.canvas, app.canvas.window().size());
            }
        }
        for event in field_events {
            bg_particles.push_field_event(event);
        }
        if !fixture.state().is_paused() {
            fg_particles.update(delta);

            if let Some(scene) = scene.as_ref() {
                bg_particles.update_scene(delta, &mut app.canvas, scene)?;
            }
        }
        for emit in to_emit_particles.into_iter() {
            fg_particles.add_source(emit.into_source(themes, &app.particle_scale));
        }

        // rebuilt per frame, since it borrows the player textures
        let mut texture_refs: Vec<(&mut Texture, TextureMode)> = vec![];
        for (player_index, textures) in player_textures.iter_mut().enumerate() {
            texture_refs.push((&mut textures.board, TextureMode::Board(player_index as u32)));
            texture_refs.push((
                &mut textures.background,
                TextureMode::Background(player_index as u32),
            ));
        }

        app.canvas
            .with_multiple_texture_canvas(texture_refs.iter(), |texture_canvas, texture_mode| {
                match texture_mode {
                    TextureMode::Background(player_id) => {
                        let player = fixture.player(*player_id);
                        let animations = themes.player_animations(*player_id);
                        themes
                            .theme(*player_id)
                            .draw_background(texture_canvas, player.game(), animations)
                            .unwrap();
                    }
                    TextureMode::Board(player_id) => {
                        let player = fixture.player(*player_id);
                        let animations = themes.player_animations(*player_id);
                        themes
                            .theme(*player_id)
                            .draw_board(texture_canvas, player.game(), animations)
                            .unwrap();
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        let games = fixture
            .players
            .iter()
            .map(|p| p.game())
            .collect::<Vec<&G>>();
        let mut frame_result: Result<(), String> = Ok(());
        app.canvas
            .with_texture_canvas(frame_buffer, |c| {
                frame_result = (|| -> Result<(), String> {
                    c.set_draw_color(Color::BLACK);
                    c.clear();

                    themes.draw_scene(c, &games)?;

                    // the field carries its own clip: one pass over every particle-scene player
                    if scene.is_some() {
                        bg_particles.draw(c)?;
                    }

                    themes.draw_players(c, &mut texture_refs, delta)?;

                    if let Some(timer) = timer.as_ref() {
                        timer.draw(c, fixture.play_time())?;
                    }

                    fg_particles.draw(c)?;

                    // board debris travels too far to draw into the board's own texture
                    themes.draw_debris(c)?;

                    // the character's particles leave its box and cross this player's well
                    themes.draw_character_particles(c)?;

                    // attacks crossing between players belong to neither, so are unclipped
                    themes.draw_attack_balls(c)?;

                    // captions go over everything, so are drawn here rather than with their board
                    themes.draw_popups(c)?;

                    if fixture.state().is_paused() {
                        paused_screen.draw(c)?;
                    }
                    Ok(())
                })();
            })
            .map_err(|e| e.to_string())?;
        frame_result?;

        app.canvas.copy(frame_buffer, None, None)?;
        app.canvas.present();
        Ok(None)
    }
}
