//! A match: the players, their games, who is winning and how garbage moves between them.

use crate::game::{Attack, Game, StageState};
use crate::high_score::table::{HighScoreStore, HighScoreTable, Ranking};
use crate::high_score::{HighScoreKey, NewHighScore};
use num_format::{Locale, ToFormattedString};
use rand::prelude::ThreadRng;
use rand::{rng, RngExt};
use std::time::Duration;

/// How a match is won. Game-neutral: stages are whatever a game calls a stage (a cleared
/// bottle, ten lines...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchRules {
    /// endless; the highest score when everyone is out wins
    Marathon,
    /// first to complete this many stages
    StageSprint { stages: u32 },
    /// first to this score
    ScoreSprint { score: u32 },
    /// one stage per theme, in order
    ThemeSprint,
}

impl MatchRules {
    pub const ONE_STAGE_SPRINT: Self = Self::StageSprint { stages: 1 };
    pub const DEFAULT_SCORE_SPRINT: Self = Self::ScoreSprint { score: 10_000 };
    /// every mode a match can be played under, one or two players alike: a marathon and
    /// the three sprints
    pub const MODES: [Self; 4] = [
        Self::Marathon,
        Self::ONE_STAGE_SPRINT,
        Self::ThemeSprint,
        Self::DEFAULT_SCORE_SPRINT,
    ];

    /// the modes a match may be played under when the players run through `theme_count`
    /// themes: a theme sprint is one stage per theme, so it takes more than one theme
    pub fn modes(theme_count: usize) -> Vec<Self> {
        Self::MODES
            .into_iter()
            .filter(|rules| theme_count > 1 || rules != &Self::ThemeSprint)
            .collect()
    }

    pub fn name(&self, stage_noun: &str) -> String {
        match self {
            MatchRules::Marathon => "marathon".to_string(),
            MatchRules::StageSprint { stages } => format!("{} {} sprint", stages, stage_noun),
            MatchRules::ScoreSprint { score } => {
                format!("{} point sprint", score.to_formatted_string(&Locale::en))
            }
            MatchRules::ThemeSprint => "theme sprint".to_string(),
        }
    }

    pub fn allow_manual_theme_change(&self) -> bool {
        self != &Self::ThemeSprint
    }

    /// a race to a goal: timed, and its high scores are the quickest finishes
    pub fn is_sprint(&self) -> bool {
        !matches!(self, MatchRules::Marathon)
    }

    /// how these rules' high score table is ranked
    pub fn ranking(&self) -> Ranking {
        if self.is_sprint() {
            Ranking::LowestTime
        } else {
            Ranking::HighestScore
        }
    }

    pub fn default_by_players(players: u32) -> Self {
        if players == 1 {
            MatchRules::Marathon
        } else {
            MatchRules::ONE_STAGE_SPRINT
        }
    }
}

pub struct Player<G: Game> {
    player: u32,
    game: G,
    winner: bool,
    /// won by reaching the sprint goal rather than by outlasting everyone else
    completed_sprint: bool,
}

impl<G: Game> Player<G> {
    pub fn new(player: u32, game: G) -> Self {
        Self {
            player,
            game,
            winner: false,
            completed_sprint: false,
        }
    }

    pub fn player(&self) -> u32 {
        self.player
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn game_mut(&mut self) -> &mut G {
        &mut self.game
    }

    /// swap in the next stage's game, keeping score and stage count; speed carries over only
    /// to the same game, as different games have different speed scales
    pub fn replace_game(&mut self, mut game: G) -> G {
        game.set_score(self.game.score());
        if game.game_id() == self.game.game_id() {
            game.set_speed_index(self.game.speed_index());
        }
        game.set_completed_stages(self.game.completed_stages());
        std::mem::replace(&mut self.game, game)
    }

    pub fn is_winner(&self) -> bool {
        self.winner
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchState {
    Normal,
    Paused,
    GameOver { high_score: Option<NewHighScore> },
}

impl MatchState {
    pub fn is_paused(&self) -> bool {
        self == &MatchState::Paused
    }

    pub fn is_game_over(&self) -> bool {
        matches!(self, MatchState::GameOver { .. })
    }

    pub fn is_normal(&self) -> bool {
        self == &MatchState::Normal
    }
}

pub struct Match<G: Game> {
    pub players: Vec<Player<G>>,
    high_scores: HighScoreTable,
    state: MatchState,
    rules: MatchRules,
    /// stages in a theme sprint: the fewest themes any player has
    theme_count: u32,
    /// players a computer plays for; they do not enter the high score table
    ai_players: Vec<u32>,
    /// the race clock of a sprint: it runs while anyone is playing, see [`Match::add_play_time`]
    play_time: Duration,
    /// who attacked whom this frame. The session picks the victim, and only the attacker is
    /// visible in the events, so the route is queued here for the renderer to drain.
    attack_routes: Vec<AttackRoute>,
    rng: ThreadRng,
}

/// An attack that landed, both ends of it named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackRoute {
    pub from: u32,
    pub to: u32,
    pub strength: u32,
}

impl<G: Game> Match<G> {
    /// `theme_counts` is how many themes each player cycles through; `high_score_key` picks
    /// the high score table this match competes for
    pub fn new(
        games: Vec<G>,
        rules: MatchRules,
        theme_counts: &[u32],
        high_score_key: &HighScoreKey,
    ) -> Self {
        assert!(!games.is_empty());
        Self {
            players: games
                .into_iter()
                .enumerate()
                .map(|(pid, game)| Player::new(pid as u32, game))
                .collect(),
            high_scores: HighScoreStore::load().unwrap().table(high_score_key),
            state: MatchState::Normal,
            rules,
            theme_count: theme_counts.iter().copied().min().unwrap_or(1).max(1),
            ai_players: vec![],
            play_time: Duration::ZERO,
            attack_routes: vec![],
            rng: rng(),
        }
    }

    pub fn with_ai_players(mut self, ai_players: Vec<u32>) -> Self {
        self.ai_players = ai_players;
        self
    }

    pub fn is_ai_player(&self, player: u32) -> bool {
        self.ai_players.contains(&player)
    }

    pub fn rules(&self) -> MatchRules {
        self.rules
    }

    pub fn player_count(&self) -> u32 {
        self.players.len() as u32
    }

    pub fn is_single_player(&self) -> bool {
        self.players.len() == 1
    }

    pub fn unset_flags(&mut self) {
        for player in self.players.iter_mut() {
            player.game.set_soft_drop(false);
        }
    }

    /// returns true if the pause state changed
    pub fn toggle_paused(&mut self) -> Option<bool> {
        match self.state {
            MatchState::Normal => {
                self.state = MatchState::Paused;
                Some(true)
            }
            MatchState::Paused => {
                self.state = MatchState::Normal;
                Some(false)
            }
            _ => None,
        }
    }

    pub fn state(&self) -> MatchState {
        self.state
    }

    /// run the race clock: the match loop calls this for every frame at least one player is
    /// playing, so it stops while paused and when everyone is held up at once (stage cards,
    /// theme fades, the end of the match)
    pub fn add_play_time(&mut self, delta: Duration) {
        self.play_time += delta;
    }

    pub fn play_time(&self) -> Duration {
        self.play_time
    }

    /// the clock as a high score table entry
    fn play_time_millis(&self) -> u32 {
        self.play_time.as_millis().min(u32::MAX as u128) as u32
    }

    fn sprint_stages(&self) -> Option<u32> {
        match self.rules {
            MatchRules::StageSprint { stages } => Some(stages),
            MatchRules::ThemeSprint => Some(self.theme_count),
            _ => None,
        }
    }

    /// whether completing the stage this player is on ends the match
    pub fn next_stage_ends_match(&self, player: u32) -> bool {
        match self.sprint_stages() {
            Some(stages) => self.player(player).game().completed_stages() + 1 >= stages,
            None => false,
        }
    }

    pub fn set_winner(&mut self, player: u32) {
        self.player_mut(player).winner = true;
    }

    /// this player reached the sprint goal: they win, and their time counts
    pub fn complete_sprint(&mut self, player: u32) {
        let player = self.player_mut(player);
        player.winner = true;
        player.completed_sprint = true;
    }

    /// whether a player has reached the goal of a sprint (a stage sprint is flagged as the
    /// last stage completes; a score sprint is read off the score)
    fn reached_sprint_goal(&self, player: &Player<G>) -> bool {
        match self.rules {
            MatchRules::ScoreSprint { score } => player.game.score() >= score,
            MatchRules::StageSprint { .. } | MatchRules::ThemeSprint => player.completed_sprint,
            MatchRules::Marathon => false,
        }
    }

    pub fn check_for_winning_player(&self) -> Option<u32> {
        if self.state.is_game_over() {
            return None;
        }

        if let Some(winner) = self.players.iter().find(|p| p.winner) {
            return Some(winner.player);
        }

        match self.rules {
            MatchRules::ScoreSprint {
                score: sprint_score,
            } => {
                let best = self.highest_score();
                if best.game.score() >= sprint_score {
                    Some(best.player)
                } else {
                    None
                }
            }
            MatchRules::StageSprint { .. } | MatchRules::ThemeSprint => {
                let stages = self.sprint_stages().unwrap_or(u32::MAX);
                let best = self.most_stages();
                if best.game.completed_stages() >= stages {
                    Some(best.player)
                } else {
                    None
                }
            }
            MatchRules::Marathon => None,
        }
    }

    /// the player whose theme music should be played: a declared winner, otherwise whoever
    /// has completed the most stages (score breaks ties). `None` when exactly tied.
    pub fn leading_player(&self) -> Option<u32> {
        if let Some(winner) = self.players.iter().find(|p| p.winner) {
            return Some(winner.player);
        }
        let mut ranked = self
            .players
            .iter()
            .map(|p| (p.game.completed_stages(), p.game.score(), p.player))
            .collect::<Vec<_>>();
        ranked.sort_by_key(|(stages, score, _)| std::cmp::Reverse((*stages, *score)));
        match ranked.as_slice() {
            [] => None,
            [best] => Some(best.2),
            [best, second, ..] if (best.0, best.1) == (second.0, second.1) => None,
            [best, ..] => Some(best.2),
        }
    }

    pub fn maybe_set_game_over(&mut self) -> bool {
        if self.state.is_game_over() {
            return false;
        }

        // only human players can enter the high score table
        let humans = self
            .players
            .iter()
            .filter(|p| !self.is_ai_player(p.player));
        let high_score = if self.rules.is_sprint() {
            // a sprint's table is the quickest finishes, so only a player who finished may
            // enter it: not one who merely outlasted an opponent, nor one who topped out.
            // The race clock stopped as the match ended, so it is the winner's time.
            let millis = self.play_time_millis();
            humans
                .filter(|p| self.reached_sprint_goal(p))
                .max_by_key(|p| p.game.score())
                .filter(|_| self.high_scores.is_high_score(millis))
                .map(|best| NewHighScore::new(best.player, millis))
        } else {
            humans
                .max_by_key(|p| p.game.score())
                .filter(|best| self.high_scores.is_high_score(best.game.score()))
                .map(|best| NewHighScore::new(best.player, best.game.score()))
        };

        self.state = MatchState::GameOver { high_score };
        true
    }

    pub fn mut_game<F>(&mut self, player: u32, mut f: F)
    where
        F: FnMut(&mut G),
    {
        if self.state.is_normal() {
            let player = self.players.get_mut(player as usize).unwrap();
            f(&mut player.game)
        }
    }

    pub fn player(&self, player: u32) -> &Player<G> {
        self.players.get(player as usize).unwrap()
    }

    pub fn player_mut(&mut self, player: u32) -> &mut Player<G> {
        self.players.get_mut(player as usize).unwrap()
    }

    /// route an attack to a random other player
    pub fn send_attack(&mut self, from_player: u32, attack: Attack) {
        if self.players.len() < 2 {
            return;
        }

        let other_players = (0..self.players.len())
            .filter(|&p| p != from_player as usize)
            .collect::<Vec<usize>>();

        let pid = if other_players.len() == 1 {
            other_players[0]
        } else {
            other_players[self.rng.random_range(0..other_players.len())]
        };
        self.attack_routes.push(AttackRoute {
            from: from_player,
            to: pid as u32,
            strength: attack.strength,
        });
        self.players
            .get_mut(pid)
            .unwrap()
            .game
            .receive_attack(attack);
    }

    /// the attacks routed since the last drain, oldest first
    pub fn drain_attack_routes(&mut self) -> Vec<AttackRoute> {
        std::mem::take(&mut self.attack_routes)
    }

    pub fn stage_state(&self, player: u32) -> StageState {
        self.player(player).game().stage_state()
    }

    fn highest_score(&self) -> &Player<G> {
        self.players
            .iter()
            .max_by_key(|p| p.game.score())
            .unwrap()
    }

    fn most_stages(&self) -> &Player<G> {
        self.players
            .iter()
            .max_by_key(|p| (p.game.completed_stages(), p.game.score()))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::geometry::Point;
    use crate::game::{Cell, GameEvent, GameId, MetricKind, PieceId, StageTransition};
    use std::time::Duration;

    /// a game that only keeps the numbers a playlist carries between stages
    struct Counter {
        id: u16,
        score: u32,
        speed: u32,
        stages: u32,
    }

    impl Game for Counter {
        fn game_id(&self) -> GameId {
            GameId(self.id)
        }
        fn update(&mut self, _: Duration) {}
        fn left(&mut self) {}
        fn right(&mut self) {}
        fn rotate(&mut self, _: bool) {}
        fn set_soft_drop(&mut self, _: bool) {}
        fn hard_drop(&mut self) {}
        fn hold(&mut self) {}
        fn drain_events(&mut self) -> Vec<GameEvent> {
            vec![]
        }
        fn board_width(&self) -> u32 {
            1
        }
        fn board_height(&self) -> u32 {
            1
        }
        fn visible_height(&self) -> u32 {
            1
        }
        fn cell(&self, _: Point) -> Cell {
            Cell::Empty
        }
        fn queue(&self) -> Vec<PieceId> {
            vec![]
        }
        fn held(&self) -> Option<PieceId> {
            None
        }
        fn metric(&self, _: MetricKind) -> Option<u32> {
            None
        }
        fn score(&self) -> u32 {
            self.score
        }
        fn set_score(&mut self, score: u32) {
            self.score = score;
        }
        fn speed_index(&self) -> u32 {
            self.speed
        }
        fn set_speed_index(&mut self, index: u32) {
            self.speed = index;
        }
        fn stage_state(&self) -> StageState {
            StageState::Playing
        }
        fn stage_transition(&self) -> StageTransition {
            StageTransition::Seamless
        }
        fn completed_stages(&self) -> u32 {
            self.stages
        }
        fn set_completed_stages(&mut self, stages: u32) {
            self.stages = stages;
        }
        fn next_stage(&mut self) -> Result<(), String> {
            self.stages += 1;
            Ok(())
        }
        fn receive_attack(&mut self, _: Attack) {}
    }

    fn counter(score: u32, speed: u32, stages: u32) -> Counter {
        Counter {
            id: 0,
            score,
            speed,
            stages,
        }
    }

    #[test]
    fn replacing_a_game_carries_score_speed_and_stages() {
        let mut player = Player::new(0, counter(1234, 3, 2));
        player.replace_game(counter(0, 0, 0));
        assert_eq!(player.game().score(), 1234);
        assert_eq!(player.game().speed_index(), 3);
        assert_eq!(player.game().completed_stages(), 2);
    }

    #[test]
    fn speed_does_not_carry_to_a_different_game() {
        let mut player = Player::new(0, counter(10, 3, 1));
        let mut other = counter(0, 7, 0);
        other.id = 1;
        player.replace_game(other);
        assert_eq!(player.game().score(), 10);
        assert_eq!(player.game().speed_index(), 7);
        assert_eq!(player.game().completed_stages(), 1);
    }

    #[test]
    fn leader_is_most_stages_then_score() {
        let mut fixture = Match {
            players: vec![
                Player::new(0, counter(500, 0, 1)),
                Player::new(1, counter(100, 0, 2)),
            ],
            high_scores: HighScoreTable::default(),
            state: MatchState::Normal,
            rules: MatchRules::Marathon,
            theme_count: 1,
            ai_players: vec![],
            play_time: Duration::ZERO,
            attack_routes: vec![],
            rng: rng(),
        };
        assert_eq!(fixture.leading_player(), Some(1));
        fixture.players[0].game.stages = 2;
        assert_eq!(fixture.leading_player(), Some(0));
        fixture.players[0].game.score = 100;
        assert_eq!(fixture.leading_player(), None);
    }

    fn sprint(players: Vec<Player<Counter>>, rules: MatchRules) -> Match<Counter> {
        Match {
            players,
            high_scores: HighScoreTable::new(rules.ranking()),
            state: MatchState::Normal,
            rules,
            theme_count: 1,
            ai_players: vec![],
            play_time: Duration::ZERO,
            attack_routes: vec![],
            rng: rng(),
        }
    }

    #[test]
    fn stage_sprint_ends_with_the_last_stage() {
        let fixture = sprint(
            vec![Player::new(0, counter(0, 0, 1))],
            MatchRules::StageSprint { stages: 2 },
        );
        assert!(fixture.next_stage_ends_match(0));
        assert_eq!(fixture.check_for_winning_player(), None);
    }

    #[test]
    fn a_finished_sprint_enters_its_time() {
        let mut fixture = sprint(
            vec![
                Player::new(0, counter(0, 0, 0)),
                Player::new(1, counter(0, 0, 0)),
            ],
            MatchRules::StageSprint { stages: 1 },
        );
        fixture.add_play_time(Duration::from_millis(90_000));
        fixture.complete_sprint(1);
        assert!(fixture.maybe_set_game_over());
        assert_eq!(
            fixture.state(),
            MatchState::GameOver {
                high_score: Some(NewHighScore::new(1, 90_000))
            }
        );
    }

    #[test]
    fn outlasting_an_opponent_is_not_a_sprint_time() {
        let mut fixture = sprint(
            vec![
                Player::new(0, counter(0, 0, 0)),
                Player::new(1, counter(0, 0, 0)),
            ],
            MatchRules::StageSprint { stages: 1 },
        );
        fixture.add_play_time(Duration::from_millis(1500));
        fixture.set_winner(0);
        assert!(fixture.maybe_set_game_over());
        assert_eq!(fixture.state(), MatchState::GameOver { high_score: None });
    }

    #[test]
    fn a_score_sprint_is_finished_by_its_score() {
        let mut fixture = sprint(
            vec![Player::new(0, counter(12_000, 0, 0))],
            MatchRules::ScoreSprint { score: 10_000 },
        );
        fixture.add_play_time(Duration::from_millis(45_000));
        assert_eq!(fixture.check_for_winning_player(), Some(0));
        assert!(fixture.maybe_set_game_over());
        assert_eq!(
            fixture.state(),
            MatchState::GameOver {
                high_score: Some(NewHighScore::new(0, 45_000))
            }
        );
    }

    #[test]
    fn ai_players_do_not_enter_times() {
        let mut fixture = sprint(
            vec![Player::new(0, counter(0, 0, 0))],
            MatchRules::StageSprint { stages: 1 },
        )
        .with_ai_players(vec![0]);
        fixture.complete_sprint(0);
        assert!(fixture.maybe_set_game_over());
        assert_eq!(fixture.state(), MatchState::GameOver { high_score: None });
    }

    #[test]
    fn a_marathon_enters_its_score() {
        let mut fixture = sprint(
            vec![Player::new(0, counter(1234, 0, 0))],
            MatchRules::Marathon,
        );
        assert!(fixture.maybe_set_game_over());
        assert_eq!(
            fixture.state(),
            MatchState::GameOver {
                high_score: Some(NewHighScore::new(0, 1234))
            }
        );
    }

    #[test]
    fn only_marathons_rank_by_score() {
        assert_eq!(MatchRules::Marathon.ranking(), Ranking::HighestScore);
        assert_eq!(MatchRules::ONE_STAGE_SPRINT.ranking(), Ranking::LowestTime);
        assert_eq!(MatchRules::DEFAULT_SCORE_SPRINT.ranking(), Ranking::LowestTime);
        assert_eq!(MatchRules::ThemeSprint.ranking(), Ranking::LowestTime);
    }
}
