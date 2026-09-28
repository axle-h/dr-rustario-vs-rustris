//! Playing Dr. Rustario headless, as fast as the machine will go, to score a genome.

use crate::game::ai::agent::{DrAiAgent, Hold};
use crate::game::ai::models::DrNeuralNetwork;
use crate::game::ai::run::{going_nowhere, survived_the_budget, PROBE_SEEDS, TOP_TRAINING_LEVEL};
use crate::game::random::{viruses_at_level, GameRandom, RandomMode};
use crate::game::{Game, GameSpeed};
use engine::ai::{EndGame, GameResult, Seed};
use engine::game::{Game as _, GameEvent};
use std::time::Duration;

/// how long the clear animation holds the game up for, matched to the real one
const CLEAR_DURATION: Duration = Duration::from_millis(400);

pub struct HeadlessGame {
    agent: DrAiAgent,
    game: Game,
    end_game: EndGame,
    options: HeadlessGameOptions,
    duration: Duration,
    game_over: bool,
    pills: u32,
    stages: u32,
    /// viruses destroyed, counted as the bottle's own count resets with every new bottle
    viruses: u32,
    /// pills placed since the last virus went, so a game going nowhere can be called off
    pills_since_clear: u32,
}

impl HeadlessGame {
    pub fn new(
        game: Game,
        agent: DrAiAgent,
        options: HeadlessGameOptions,
        end_game: EndGame,
    ) -> Self {
        Self {
            agent,
            game,
            duration: Duration::ZERO,
            game_over: false,
            pills: 0,
            stages: 0,
            viruses: 0,
            pills_since_clear: 0,
            options,
            end_game,
        }
    }

    pub fn play(&mut self) -> GameResult {
        loop {
            if let Some(result) = self.update() {
                return result;
            }
        }
    }

    fn update(&mut self) -> Option<GameResult> {
        self.duration += self.options.step;

        // a game called off for going nowhere counts as buried
        if self.pills_since_clear >= self.options.stall_pills {
            self.game_over = true;
        }
        let result = self.result();
        if self.game_over || self.end_game.is_end_game(result, self.duration) {
            return Some(result);
        }

        let viruses_before = self.game.bottle().virus_count();

        self.agent.act(&mut self.game, self.options.step);
        let mut events = self.game.drain_events();
        self.game.update(self.options.step);
        events.extend(self.game.drain_events());

        // the bottle's virus count only ever falls within a stage, so the drop is what was killed
        let killed = viruses_before.saturating_sub(self.game.bottle().virus_count());
        self.viruses += killed;
        if killed > 0 {
            self.pills_since_clear = 0;
        }

        for event in events {
            match event {
                GameEvent::GameOver => {
                    self.game_over = true;
                    return Some(self.result());
                }
                GameEvent::Clear { .. } => {
                    // simulate the clear animation holding the game up
                    self.duration += self.options.clear_delay;
                }
                GameEvent::Spawn { .. } => {
                    self.pills += 1;
                    self.pills_since_clear += 1;
                }
                GameEvent::StageComplete => {
                    self.stages += 1;
                    self.pills_since_clear = 0;
                    // cleared the last bottle training asks for, so the run is a success
                    if self.stages > self.options.top_level {
                        return Some(self.result());
                    }
                    // the real game shows an interstitial here; training goes straight on
                    if self.game.next_stage().is_err() {
                        return Some(self.result());
                    }
                    self.agent.reset();
                }
                _ => (),
            }
        }

        None
    }

    fn result(&self) -> GameResult {
        GameResult::new(
            self.game.score(),
            self.viruses,
            self.game.completed_stages(),
            self.game_over,
            self.duration,
        )
        .with_pieces(self.pills, self.stages)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeadlessGameOptions {
    pub clear_delay: Duration,
    pub step: Duration,
    pub speed: GameSpeed,
    /// the last bottle a training game plays
    pub top_level: u32,
    /// pills a game may go without destroying a virus before it is called off as a burial
    pub stall_pills: u32,
    /// whether the agent may weigh the held pill; see [`Hold`] for why it is off
    pub hold: Hold,
}

impl Default for HeadlessGameOptions {
    fn default() -> Self {
        Self {
            step: Duration::from_millis(16), // 60hz
            clear_delay: CLEAR_DURATION,
            speed: GameSpeed::Medium,
            top_level: TOP_TRAINING_LEVEL,
            stall_pills: STALL_PILLS,
            hold: Hold::Off,
        }
    }
}

/// every virus in every bottle from the first up to and including [`TOP_TRAINING_LEVEL`]
pub const VIRUSES_TO_CLEAR: u32 = {
    let mut total = 0;
    let mut level = 0;
    while level <= TOP_TRAINING_LEVEL {
        total += viruses_at_level(level);
        level += 1;
    }
    total
};

/// pills without a virus destroyed before a game is called off as going nowhere
const STALL_PILLS: u32 = 200;

pub struct HeadlessGameFixture {
    random_mode: RandomMode,
    seed: Seed,
    seeds_per_game: usize,
    game_options: HeadlessGameOptions,
    end_game: EndGame,
}

impl HeadlessGameFixture {
    pub fn new(
        random_mode: RandomMode,
        seed: Seed,
        game_options: HeadlessGameOptions,
        end_game: EndGame,
    ) -> Self {
        Self {
            random_mode,
            seed,
            seeds_per_game: 1,
            game_options,
            end_game,
        }
    }

    pub fn set_seeds_per_game(&mut self, seeds_per_game: usize) {
        assert!(seeds_per_game > 0, "must play at least one seed per game");
        self.seeds_per_game = seeds_per_game;
    }

    pub fn seeds_per_game(&self) -> usize {
        self.seeds_per_game
    }

    pub fn set_end_game(&mut self, end_game: EndGame) {
        self.end_game = end_game;
    }

    /// advance to the next block of unused seeds
    pub fn next_seed(&mut self) {
        self.seed += Seed::from(self.seeds_per_game as u128);
    }

    pub fn current_seed(&self) -> Seed {
        self.seed
    }

    /// Play one whole game per seed and average them. The average's game over flag is set if any
    /// seed was buried, and a candidate cut after [`PROBE_SEEDS`] is still divided by every seed
    /// it was given, so a cut can only cost it.
    pub fn play(&self, network: DrNeuralNetwork) -> GameResult {
        let results = self.play_run(network, self.seed);
        let total: GameResult = results.iter().copied().sum();
        (total / self.seeds_per_game).with_game_over(!survived_the_budget(&results))
    }

    /// the seeds of one run from `block`, cut short if the probe seeds go nowhere
    pub fn play_run(&self, network: DrNeuralNetwork, block: Seed) -> Vec<GameResult> {
        let mut results: Vec<GameResult> = Vec::with_capacity(self.seeds_per_game);
        for i in 0..self.seeds_per_game as u128 {
            results.push(self.play_seed(network, block + Seed::from(i)));
            if results.len() == PROBE_SEEDS && going_nowhere(&results, self.seeds_per_game) {
                break;
            }
        }
        results
    }

    /// one game, played from the first bottle up until it is buried or runs out of bottles
    pub fn play_seed(&self, network: DrNeuralNetwork, seed: Seed) -> GameResult {
        let random = GameRandom::from_seed(seed.into(), self.random_mode);
        let game = Game::new(0, self.game_options.speed, random).expect("could not deal a bottle");

        HeadlessGame::new(
            game,
            DrAiAgent::new(network).with_hold(self.game_options.hold),
            self.game_options,
            self.end_game,
        )
        .play()
    }
}
