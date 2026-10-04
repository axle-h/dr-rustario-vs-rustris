//! Playing Dr. Rustario headless, as fast as the machine will go, to score a genome.

use crate::game::ai::agent::{Choices, DrAiAgent, Hold};
use crate::game::ai::models::DrNeuralNetwork;
use crate::game::ai::run::{
    merit, survived_the_budget, Fire, Incoming, Traffic, TOP_TRAINING_LEVEL,
};
use crate::game::pill::VirusColor;
use crate::game::random::{viruses_at_level, GameRandom, RandomMode};
use crate::game::{Game, GameSpeed};
use engine::ai::{EndGame, GameResult, Seed};
use engine::game::{Attack, Game as _, GameEvent};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::ops::Range;
use std::time::Duration;

/// the stream of a game's seed its incoming garbage is rolled from, apart from what it deals
const INCOMING_STREAM: u64 = 1;

/// how long the clear animation holds the game up for, matched to the real one
const CLEAR_DURATION: Duration = Duration::from_millis(400);

pub struct HeadlessGame {
    agent: DrAiAgent,
    game: Game,
    /// the bottle the game was dealt first, so the one it is on is this plus `stages`
    start_level: u32,
    end_game: EndGame,
    options: HeadlessGameOptions,
    /// what rolls the garbage dropped on it, when [`HeadlessGameOptions::incoming`] is set
    incoming: ChaChaRng,
    traffic: Traffic,
    duration: Duration,
    game_over: bool,
    pills: u32,
    stages: u32,
    /// viruses destroyed, counted as the bottle's own count resets with every new bottle
    viruses: u32,
    /// pills placed since the last virus went, so a game going nowhere can be called off
    pills_since_clear: u32,
    /// whether the game was called off for going nowhere rather than topped out
    stalled: bool,
    /// what the last frame sent, for a duel to deliver
    sent: Vec<Attack>,
}

impl HeadlessGame {
    pub fn new(
        game: Game,
        agent: DrAiAgent,
        options: HeadlessGameOptions,
        end_game: EndGame,
        seed: Seed,
    ) -> Self {
        let mut incoming: ChaChaRng = seed.into();
        incoming.set_stream(INCOMING_STREAM);
        Self {
            agent,
            start_level: game.virus_level(),
            game,
            incoming,
            traffic: Traffic::default(),
            duration: Duration::ZERO,
            game_over: false,
            pills: 0,
            stages: 0,
            viruses: 0,
            pills_since_clear: 0,
            stalled: false,
            sent: vec![],
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

    pub fn choices(&self) -> Choices {
        self.agent.choices()
    }

    pub fn traffic(&self) -> Traffic {
        self.traffic
    }

    pub fn stalled(&self) -> bool {
        self.stalled
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// bottles cleared since it was dealt
    pub fn stages(&self) -> u32 {
        self.stages
    }

    pub fn viruses(&self) -> u32 {
        self.viruses
    }

    pub fn pills(&self) -> u32 {
        self.pills
    }

    /// the attacks the last frame sent
    pub fn sent(&self) -> &[Attack] {
        &self.sent
    }

    /// an opponent's attack, landing as the real game lands one
    pub fn receive_attack(&mut self, attack: Attack) {
        self.traffic.blocks_received += attack.strength;
        self.game.receive_attack(attack);
    }

    /// Roll for an attack as a pill spawns. It lands before the next one, as an opponent's does.
    fn receive(&mut self, incoming: Incoming) {
        if self.incoming.random::<f64>() >= incoming.per_pill {
            return;
        }
        let blocks = incoming.size(self.incoming.random());
        let garbage: Vec<VirusColor> = (0..blocks).map(|_| self.incoming.random()).collect();
        self.game.send_garbage(garbage);
        self.traffic.blocks_received += blocks;
    }

    /// one frame, and the result once the game is over
    pub fn update(&mut self) -> Option<GameResult> {
        self.duration += self.options.step;
        self.sent.clear();

        // a game called off for going nowhere counts as buried
        if self.pills_since_clear >= self.options.stall_pills {
            self.game_over = true;
            self.stalled = true;
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
                    if let Some(fire) = self.options.incoming {
                        self.receive(fire.at(self.start_level));
                    }
                }
                GameEvent::AttackSent(attack) => {
                    self.traffic.sent(attack.strength);
                    self.sent.push(attack);
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
        .with_merit(merit(
            self.viruses,
            self.stages,
            self.traffic.blocks_sent,
            self.game_over,
        ))
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
    /// garbage dropped on the game in place of an opponent's, if any
    pub incoming: Option<Fire>,
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
            incoming: None,
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

/// Games averaged into one result, whose game over flag is set if any of them was buried.
pub fn summarise(results: &[GameResult]) -> GameResult {
    let total: GameResult = results.iter().copied().sum();
    (total / results.len()).with_game_over(!survived_the_budget(results))
}

pub struct HeadlessGameFixture {
    random_mode: RandomMode,
    seed: Seed,
    seeds_per_game: usize,
    /// the bottle each seed of a block starts from, in turn
    levels: &'static [u32],
    game_options: HeadlessGameOptions,
    end_game: EndGame,
}

/// One game and how it was played.
#[derive(Clone, Copy, Debug)]
pub struct Played {
    pub result: GameResult,
    pub choices: Choices,
    pub traffic: Traffic,
    /// the bottle it started from
    pub level: u32,
    /// buried by being called off for going nowhere, not by topping out
    pub stalled: bool,
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
            levels: &[0],
            game_options,
            end_game,
        }
    }

    /// start the seeds of a block from each of `levels` in turn rather than all from the first
    pub fn with_levels(mut self, levels: &'static [u32]) -> Self {
        assert!(!levels.is_empty(), "a game has to start somewhere");
        self.levels = levels;
        self
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

    /// play every seed of the current block and [`summarise`] them
    pub fn play(&self, network: DrNeuralNetwork) -> GameResult {
        summarise(&self.play_games(network, 0..self.seeds_per_game))
    }

    /// seeds `games` of the current block, a result each
    pub fn play_games(&self, network: DrNeuralNetwork, games: Range<usize>) -> Vec<GameResult> {
        games
            .map(|index| {
                self.game(self.network_agent(network), self.seed, index)
                    .result
            })
            .collect()
    }

    /// every seed of `block`, played at once, by the agent `agent` makes
    pub fn games(&self, agent: impl Fn() -> DrAiAgent + Sync, block: Seed) -> Vec<Played> {
        use rayon::prelude::*;
        (0..self.seeds_per_game)
            .into_par_iter()
            .map(|index| self.game(agent(), block, index))
            .collect()
    }

    /// the network as training plays it
    pub fn network_agent(&self, network: DrNeuralNetwork) -> DrAiAgent {
        DrAiAgent::new(network).with_hold(self.game_options.hold)
    }

    /// The `index`th seed of `block`, from its level until it is buried, runs out of bottles or
    /// hits the end game.
    pub fn game(&self, agent: DrAiAgent, block: Seed, index: usize) -> Played {
        let (mut headless, level) = self.headless(agent, block, index);
        let result = headless.play();
        Played {
            result,
            choices: headless.choices(),
            traffic: headless.traffic(),
            level,
            stalled: headless.stalled(),
        }
    }

    /// the `index`th seed of `block` dealt and ready to play, with the level it starts from
    pub fn headless(&self, agent: DrAiAgent, block: Seed, index: usize) -> (HeadlessGame, u32) {
        let seed = block + Seed::from(index as u128);
        let level = self.levels[index % self.levels.len()];
        let random = GameRandom::from_seed(seed.into(), self.random_mode);
        let game =
            Game::new(level, self.game_options.speed, random).expect("could not deal a bottle");
        let headless = HeadlessGame::new(game, agent, self.game_options, self.end_game, seed);
        (headless, level)
    }
}
