//! The games the launcher can run, as one type the engine's generic match loop accepts.

use engine::game::geometry::Point;
use engine::game::{
    Attack, Cell, Game, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
use engine::render::GameRender;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameKind {
    DrRustario,
    Rustris,
}

impl GameKind {
    /// every game the launcher can run, in the order they are numbered. This is the key of
    /// every per-game collection - see [`PerGame`] - and the order the themes are built in,
    /// so a game's themes keep one place in the shared list.
    pub const ALL: [GameKind; 2] = [GameKind::DrRustario, GameKind::Rustris];

    /// the order the games are billed in: the pre-menu's list, and the turns a fixed versus
    /// playlist takes. Rustris opens, which is a decision about presentation rather than
    /// about how the games are numbered, so it is its own list.
    pub const RUNNING_ORDER: [GameKind; 2] = [GameKind::Rustris, GameKind::DrRustario];

    pub const COUNT: usize = Self::ALL.len();

    /// what this game is called on the pre-menu
    pub fn name(self) -> &'static str {
        match self {
            GameKind::DrRustario => "dr. rustario",
            GameKind::Rustris => "rustris",
        }
    }

    /// this game's slot in a [`PerGame`]
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|game| *game == self)
            .expect("every game is in GameKind::ALL")
    }
}

/// One value per game, keyed by [`GameKind`].
///
/// This is the shape everything that used to name the two games by hand takes instead - the
/// themes each game contributes, the slots a playlist deals it, the brains an ai player thinks
/// with - so that a third game is an *entry* in a collection rather than another field, and
/// the compiler cannot be satisfied by leaving it out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerGame<T>(Vec<T>);

impl<T> PerGame<T> {
    /// one value for each of [`GameKind::ALL`], in that order
    pub fn new(mut value: impl FnMut(GameKind) -> T) -> Self {
        Self(GameKind::ALL.into_iter().map(&mut value).collect())
    }

    /// one value for each of [`GameKind::ALL`], in that order, already built
    pub fn from_values(values: Vec<T>) -> Self {
        assert_eq!(
            values.len(),
            GameKind::COUNT,
            "a PerGame needs one value per game"
        );
        Self(values)
    }

    pub fn get(&self, game: GameKind) -> &T {
        &self.0[game.index()]
    }

    pub fn get_mut(&mut self, game: GameKind) -> &mut T {
        &mut self.0[game.index()]
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }
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
    /// which game this is, which is what a versus playlist deals and what an ai controller
    /// has to dispatch on
    pub fn kind(&self) -> GameKind {
        match self {
            AnyGame::DrRustario(_) => GameKind::DrRustario,
            AnyGame::Rustris(_) => GameKind::Rustris,
        }
    }
}

/// One game's ai, playing through [`AnyGame`].
///
/// A versus playlist deals every game, so an ai player is a brain *per game* - each of them
/// whatever that game would field on its own. Behind this trait that is a list with one entry
/// per game rather than a tuple that grows a field every time the compendium does: a brain
/// handed a board that is not its game simply does nothing, and the one whose game it is
/// plays it.
pub trait AiBrain {
    /// play one frame, if the board in front of it is the game this brain knows
    fn act(&mut self, game: &mut AnyGame, delta: Duration);

    /// forget whatever was queued: the playlist has swapped the board over
    fn reset(&mut self);
}

/// a Dr. Rustario brain, playing only Dr. Rustario boards
pub fn dr_rustario_brain(
    brain: dr_rustario::game::ai::DrAiKind,
    key_delay: Duration,
) -> Box<dyn AiBrain> {
    struct DrBrain(dr_rustario::game::ai::agent::DrAiAgent);
    impl AiBrain for DrBrain {
        fn act(&mut self, game: &mut AnyGame, delta: Duration) {
            if let AnyGame::DrRustario(game) = game {
                self.0.act(game, delta);
            }
        }
        fn reset(&mut self) {
            self.0.reset();
        }
    }
    Box::new(DrBrain(
        dr_rustario::game::ai::agent::DrAiAgent::of(brain).with_key_delay(key_delay),
    ))
}

/// a Rustris brain, playing only Rustris boards
pub fn rustris_brain(
    network: rustris::game::ai::models::TetrisNeuralNetwork,
    key_delay: Duration,
) -> Box<dyn AiBrain> {
    struct RustrisBrain(rustris::game::ai::agent::AiAgent);
    impl AiBrain for RustrisBrain {
        fn act(&mut self, game: &mut AnyGame, delta: Duration) {
            if let AnyGame::Rustris(game) = game {
                self.0.act(game, delta);
            }
        }
        fn reset(&mut self) {
            self.0.reset();
        }
    }
    Box::new(RustrisBrain(
        rustris::game::ai::agent::AiAgent::neural(network).with_key_delay(key_delay),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// the two lists are the same games in different orders: one game left out of either
    /// would go missing from the menus or from a per-game collection
    #[test]
    fn every_game_is_numbered_and_billed_exactly_once() {
        for (i, game) in GameKind::ALL.iter().enumerate() {
            assert_eq!(game.index(), i);
            assert_eq!(
                GameKind::RUNNING_ORDER
                    .iter()
                    .filter(|g| *g == game)
                    .count(),
                1,
                "{game:?} is not billed exactly once"
            );
        }
        assert_eq!(GameKind::RUNNING_ORDER.len(), GameKind::COUNT);
    }

    #[test]
    fn a_per_game_collection_keeps_one_value_per_game() {
        let names = PerGame::new(|game| game.name());
        for game in GameKind::ALL {
            assert_eq!(*names.get(game), game.name());
        }
        assert_eq!(names.values().count(), GameKind::COUNT);
        // ... and in the order they are numbered, which is what anything zipping ALL against
        // the values relies on
        assert_eq!(
            names.values().copied().collect::<Vec<&str>>(),
            GameKind::ALL.map(|game| game.name()).to_vec()
        );
    }
}
