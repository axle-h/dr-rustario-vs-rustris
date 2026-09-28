//! The games the launcher can run, as one type the engine's generic match loop accepts.

use engine::animate::nuisance::NuisanceFall;
use engine::game::geometry::Point;
use engine::game::{
    Attack, Cell, CellId, Game, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
use engine::render::GameRender;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameKind {
    DrRustario,
    Rustris,
    Puyo,
    /// only with the `rustle-fighter` feature; without it the game is not in the binary
    #[cfg(feature = "rustle-fighter")]
    RustleFighter,
}

impl GameKind {
    /// every game in the order they are numbered: the key of every [`PerGame`] and the order
    /// the themes are built in
    pub const ALL: [GameKind; Self::COUNT] = [
        GameKind::DrRustario,
        GameKind::Rustris,
        GameKind::Puyo,
        #[cfg(feature = "rustle-fighter")]
        GameKind::RustleFighter,
    ];

    /// the order the games are billed in on the pre-menu
    pub const RUNNING_ORDER: [GameKind; Self::COUNT] = [
        GameKind::Rustris,
        GameKind::DrRustario,
        GameKind::Puyo,
        #[cfg(feature = "rustle-fighter")]
        GameKind::RustleFighter,
    ];

    /// The games a versus playlist can deal: the turn order and default selection, not what a
    /// match deals, which is `VersusMode`'s ticked subset (`modes::Dealt`).
    ///
    /// A game joins once it has an ai and its crossings are priced, so Super Rustle Fighter is
    /// not on it.
    pub const PLAYLIST_ORDER: &'static [GameKind] =
        &[GameKind::Rustris, GameKind::DrRustario, GameKind::Puyo];

    /// how many games this build carries: Super Rustle Fighter is behind a feature
    pub const COUNT: usize = if cfg!(feature = "rustle-fighter") {
        4
    } else {
        3
    };

    /// what this game is called on the pre-menu
    pub fn name(self) -> &'static str {
        match self {
            GameKind::DrRustario => "dr. rustario",
            GameKind::Rustris => "rustris",
            GameKind::Puyo => "puyo rusto",
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => "super rustle fighter",
        }
    }

    /// Whether this game has an ai; everything offering ai opponents, demos or difficulties
    /// asks this.
    pub fn fields_an_ai(self) -> bool {
        #[cfg(feature = "rustle-fighter")]
        if self == GameKind::RustleFighter {
            return false;
        }
        true
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerGame<T>(Vec<T>);

/// a default value for every game, not an empty collection, since every access indexes by
/// [`GameKind::index`]
impl<T: Default> Default for PerGame<T> {
    fn default() -> Self {
        Self::new(|_| T::default())
    }
}

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

/// Every match runs through this; a defaulted `Game` or `GameRender` method it does not
/// delegate is silently never asked of the game, and only the tests below catch it.
#[allow(clippy::large_enum_variant)]
pub enum AnyGame {
    DrRustario(dr_rustario::game::Game),
    Rustris(rustris::game::Game),
    Puyo(puyo_rusto::game::Game),
    #[cfg(feature = "rustle-fighter")]
    RustleFighter(rustle_fighter::game::Game),
}

macro_rules! delegate {
    ($self:ident, $game:ident => $body:expr) => {
        match $self {
            AnyGame::DrRustario($game) => $body,
            AnyGame::Rustris($game) => $body,
            AnyGame::Puyo($game) => $body,
            #[cfg(feature = "rustle-fighter")]
            AnyGame::RustleFighter($game) => $body,
        }
    };
}

impl AnyGame {
    pub fn kind(&self) -> GameKind {
        match self {
            AnyGame::DrRustario(_) => GameKind::DrRustario,
            AnyGame::Rustris(_) => GameKind::Rustris,
            AnyGame::Puyo(_) => GameKind::Puyo,
            #[cfg(feature = "rustle-fighter")]
            AnyGame::RustleFighter(_) => GameKind::RustleFighter,
        }
    }
}

/// One game's ai, playing through [`AnyGame`]; handed another game's board it does nothing.
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

/// a Puyo Rusto brain, playing only Puyo boards
pub fn puyo_brain(
    brain: puyo_rusto::game::ai::PuyoAiKind,
    key_delay: Duration,
) -> Box<dyn AiBrain> {
    struct PuyoBrain(puyo_rusto::game::ai::agent::PuyoAiAgent);
    impl AiBrain for PuyoBrain {
        fn act(&mut self, game: &mut AnyGame, delta: Duration) {
            if let AnyGame::Puyo(game) = game {
                self.0.act(game, delta);
            }
        }
        fn reset(&mut self) {
            self.0.reset();
        }
    }
    Box::new(PuyoBrain(
        puyo_rusto::game::ai::agent::PuyoAiAgent::of(brain).with_key_delay(key_delay),
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

    /// defaulted on [`Game`], so a wrapper that omits it answers 0.0 for every game
    fn fall_progress(&self) -> f64 {
        delegate!(self, g => Game::fall_progress(g))
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

    fn pending_attacks(&self) -> Vec<CellId> {
        delegate!(self, g => Game::pending_attacks(g))
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

    fn clear_popup(&self, event: &GameEvent) -> Option<String> {
        delegate!(self, g => GameRender::clear_popup(g, event))
    }

    fn spawn_cells(&self) -> Vec<Point> {
        delegate!(self, g => GameRender::spawn_cells(g))
    }

    fn stage_intro_cells(&self) -> Vec<PlacedCell> {
        delegate!(self, g => GameRender::stage_intro_cells(g))
    }

    fn attack_fall(&self) -> Option<NuisanceFall> {
        delegate!(self, g => GameRender::attack_fall(g))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn puyo() -> AnyGame {
        use puyo_rusto::game::rules::Difficulty;
        let difficulty = Difficulty::Normal;
        let seed = puyo_rusto::game::random::Seed::from_u64(42);
        let skin = puyo_rusto::game::cell::PuyoSkin::deal(seed, 1)[0];
        AnyGame::Puyo(puyo_rusto::game::Game::new(
            difficulty,
            0,
            puyo_rusto::game::random::from_seed(seed, 1, difficulty.colors())
                .into_iter()
                .next()
                .unwrap(),
            skin,
        ))
    }

    /// The wrapper forwards `fall_progress`, which a falling Puyo pair answers non-zero.
    #[test]
    fn a_wrapped_game_is_asked_how_far_it_has_fallen() {
        let mut game = puyo();
        game.set_soft_drop(true);

        let mut seen: Vec<f64> = vec![];
        for _ in 0..30 {
            game.update(Duration::from_millis(16));
            seen.push(game.fall_progress());
        }
        assert!(
            seen.iter().any(|p| *p > 0.0),
            "the wrapper answered 0.0 for a falling pair - the default, not the game: {seen:?}"
        );
    }

    #[test]
    fn a_wrapped_game_is_asked_what_is_in_its_tray() {
        let mut game = puyo();
        assert!(game.pending_attacks().is_empty());
        game.receive_attack(Attack::new(puyo_rusto::game::GAME_ID, 7));
        assert!(
            !game.pending_attacks().is_empty(),
            "the wrapper answered an empty tray - the default, not the game, so nothing is drawn"
        );
    }

    #[test]
    fn a_wrapped_game_is_asked_how_fast_an_attack_falls_in() {
        assert_eq!(
            GameRender::attack_fall(&puyo()),
            Some(puyo_rusto::game::rules::NUISANCE_FALL),
            "the wrapper answered None - the default, not the game, so nuisance appears \
             instead of falling"
        );
    }

    /// a big attack in the sender's own units, which every other game must feel
    fn a_real_attack(sender: GameKind, receiver: GameKind) -> u32 {
        let id = |game| match game {
            GameKind::DrRustario => engine::game::ids::DR_RUSTARIO,
            GameKind::Rustris => engine::game::ids::RUSTRIS,
            GameKind::Puyo => engine::game::ids::PUYO,
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => engine::game::ids::RUSTLE_FIGHTER,
        };
        match sender {
            GameKind::DrRustario => dr_rustario::game::foreign_attack(id(receiver), 3),
            GameKind::Rustris => rustris::game::foreign_attack(
                id(receiver),
                rustris::game::ClearAction {
                    lines: 4,
                    spin: None,
                    perfect_clear: false,
                },
            ),
            GameKind::Puyo => puyo_rusto::game::foreign_attack(id(receiver), 60),
            // not in `PLAYLIST_ORDER`, so it has no crossings
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => 0,
        }
    }

    /// Every pair in [`GameKind::PLAYLIST_ORDER`] prices a big attack above zero, since an
    /// unpriced crossing drops silently.
    #[test]
    fn every_crossing_between_two_games_is_priced() {
        for sender in GameKind::PLAYLIST_ORDER.iter().copied() {
            for receiver in GameKind::PLAYLIST_ORDER.iter().copied() {
                if sender == receiver {
                    continue;
                }
                assert!(
                    a_real_attack(sender, receiver) > 0,
                    "{sender:?} -> {receiver:?} is unpriced: its attacks are dropped at the \
                     border and nothing arrives"
                );
            }
        }
    }

    /// `ALL` and `RUNNING_ORDER` hold every game exactly once
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

    /// `PLAYLIST_ORDER` is a subset of `ALL` with no repeats
    #[test]
    fn every_game_a_playlist_deals_is_a_game() {
        for game in GameKind::PLAYLIST_ORDER {
            assert!(GameKind::ALL.contains(game), "{game:?}");
            assert_eq!(
                GameKind::PLAYLIST_ORDER
                    .iter()
                    .filter(|g| *g == game)
                    .count(),
                1,
                "{game:?} takes two turns"
            );
        }
    }

    #[test]
    fn a_per_game_collection_keeps_one_value_per_game() {
        let names = PerGame::new(|game| game.name());
        for game in GameKind::ALL {
            assert_eq!(*names.get(game), game.name());
        }
        assert_eq!(names.values().count(), GameKind::COUNT);
        // in `ALL` order, which zipping against the values relies on
        assert_eq!(
            names.values().copied().collect::<Vec<&str>>(),
            GameKind::ALL.map(|game| game.name()).to_vec()
        );
    }
}
