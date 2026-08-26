//! The games the launcher can run, as one type the engine's generic match loop accepts.

use engine::game::geometry::Point;
use engine::game::{
    Attack, Cell, Game, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
use engine::render::GameRender;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameKind {
    DrRustario,
    Rustris,
}

pub enum AnyGame {
    DrRustario(dr_rustario::game::Game),
    Rustris(rustris::game::Game),
}

macro_rules! delegate {
    ($self:ident, $game:ident => $body:expr) => {
        match $self {
            AnyGame::DrRustario($game) => $body,
            AnyGame::Rustris($game) => $body,
        }
    };
}

impl AnyGame {
    /// which of the two games this is, which is what a versus playlist deals and what an ai
    /// controller has to dispatch on
    pub fn kind(&self) -> GameKind {
        match self {
            AnyGame::DrRustario(_) => GameKind::DrRustario,
            AnyGame::Rustris(_) => GameKind::Rustris,
        }
    }
}

impl Game for AnyGame {
    fn game_id(&self) -> GameId {
        delegate!(self, g => Game::game_id(g))
    }

    fn update(&mut self, delta: Duration) {
        delegate!(self, g => Game::update(g, delta))
    }

    fn left(&mut self) {
        delegate!(self, g => Game::left(g))
    }

    fn right(&mut self) {
        delegate!(self, g => Game::right(g))
    }

    fn rotate(&mut self, clockwise: bool) {
        delegate!(self, g => Game::rotate(g, clockwise))
    }

    fn set_soft_drop(&mut self, soft_drop: bool) {
        delegate!(self, g => Game::set_soft_drop(g, soft_drop))
    }

    fn hard_drop(&mut self) {
        delegate!(self, g => Game::hard_drop(g))
    }

    fn hold(&mut self) {
        delegate!(self, g => Game::hold(g))
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        delegate!(self, g => Game::drain_events(g))
    }

    fn board_width(&self) -> u32 {
        delegate!(self, g => Game::board_width(g))
    }

    fn board_height(&self) -> u32 {
        delegate!(self, g => Game::board_height(g))
    }

    fn visible_height(&self) -> u32 {
        delegate!(self, g => Game::visible_height(g))
    }

    fn cell(&self, point: Point) -> Cell {
        delegate!(self, g => Game::cell(g, point))
    }

    fn queue(&self) -> Vec<PieceId> {
        delegate!(self, g => Game::queue(g))
    }

    fn held(&self) -> Option<PieceId> {
        delegate!(self, g => Game::held(g))
    }

    fn metric(&self, kind: MetricKind) -> Option<u32> {
        delegate!(self, g => Game::metric(g, kind))
    }

    fn score(&self) -> u32 {
        delegate!(self, g => Game::score(g))
    }

    fn set_score(&mut self, score: u32) {
        delegate!(self, g => Game::set_score(g, score))
    }

    fn speed_index(&self) -> u32 {
        delegate!(self, g => Game::speed_index(g))
    }

    fn set_speed_index(&mut self, index: u32) {
        delegate!(self, g => Game::set_speed_index(g, index))
    }

    fn stage_state(&self) -> StageState {
        delegate!(self, g => Game::stage_state(g))
    }

    fn stage_transition(&self) -> StageTransition {
        delegate!(self, g => Game::stage_transition(g))
    }

    fn completed_stages(&self) -> u32 {
        delegate!(self, g => Game::completed_stages(g))
    }

    fn set_completed_stages(&mut self, stages: u32) {
        delegate!(self, g => Game::set_completed_stages(g, stages))
    }

    fn next_stage(&mut self) -> Result<(), String> {
        delegate!(self, g => Game::next_stage(g))
    }

    fn receive_attack(&mut self, attack: Attack) {
        delegate!(self, g => Game::receive_attack(g, attack))
    }
}

impl GameRender for AnyGame {
    fn name(&self) -> &'static str {
        delegate!(self, g => GameRender::name(g))
    }

    fn clear_class(&self, event: &GameEvent) -> u16 {
        delegate!(self, g => GameRender::clear_class(g, event))
    }

    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        delegate!(self, g => GameRender::clear_word(g, event))
    }

    fn spawn_cells(&self) -> Vec<Point> {
        delegate!(self, g => GameRender::spawn_cells(g))
    }

    fn stage_intro_cells(&self) -> Vec<PlacedCell> {
        delegate!(self, g => GameRender::stage_intro_cells(g))
    }
}
