use crate::config::config_path;
use crate::high_score::HighScoreKey;
use confy::ConfyError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_HIGH_SCORES: usize = 5;
const CONFIG_NAME: &str = "high_scores";

/// `ms` as a stopwatch reading, `m:ss.cc`
pub fn format_millis(ms: u32) -> String {
    let centis = ms / 10;
    format!(
        "{}:{:02}.{:02}",
        centis / 6000,
        (centis / 100) % 60,
        centis % 100
    )
}

/// What a table ranks its entries by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ranking {
    /// the most points, a marathon
    #[default]
    HighestScore,
    /// the quickest finish, in milliseconds; a sprint
    LowestTime,
}

impl Ranking {
    /// whether `new` ranks above `existing`
    pub fn beats(&self, new: u32, existing: u32) -> bool {
        match self {
            Ranking::HighestScore => new > existing,
            Ranking::LowestTime => new < existing,
        }
    }

    pub fn format(&self, value: u32) -> String {
        match self {
            Ranking::HighestScore => value.to_string(),
            Ranking::LowestTime => format_millis(value),
        }
    }

    /// the table's column header
    pub fn label(&self) -> &'static str {
        match self {
            Ranking::HighestScore => "Score",
            Ranking::LowestTime => "Time",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Ranking::HighestScore => "High Scores",
            Ranking::LowestTime => "Best Times",
        }
    }

    pub fn new_entry_title(&self) -> &'static str {
        match self {
            Ranking::HighestScore => "New High Score",
            Ranking::LowestTime => "New Best Time",
        }
    }
}

/// One entry: `score` is points or, in a [`Ranking::LowestTime`] table, milliseconds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighScore {
    pub name: String,
    pub score: u32,
}

impl HighScore {
    pub fn new(name: &str, score: u32) -> Self {
        Self {
            name: name.to_string(),
            score,
        }
    }

    pub fn from_string(name: String, score: u32) -> Self {
        Self { name, score }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighScoreTable {
    /// tables written before rankings existed are score tables
    #[serde(default)]
    ranking: Ranking,
    scores: Vec<HighScore>,
}

impl Default for HighScoreTable {
    fn default() -> Self {
        Self::new(Ranking::HighestScore)
    }
}

impl HighScoreTable {
    /// a fresh table of placeholder entries to beat: low scores, or long times for a sprint
    pub fn new(ranking: Ranking) -> Self {
        const MINUTE: u32 = 60_000;
        let scores = match ranking {
            Ranking::HighestScore => vec![
                HighScore::new("ALEX", 500),
                HighScore::new("MOLLY", 400),
                HighScore::new("ESME", 300),
                HighScore::new("MOLLI", 200),
                HighScore::new("MOGS", 100),
            ],
            Ranking::LowestTime => vec![
                HighScore::new("ALEX", 5 * MINUTE),
                HighScore::new("MOLLY", 10 * MINUTE),
                HighScore::new("ESME", 15 * MINUTE),
                HighScore::new("MOLLI", 20 * MINUTE),
                HighScore::new("MOGS", 25 * MINUTE),
            ],
        };
        Self { ranking, scores }
    }

    pub fn ranking(&self) -> Ranking {
        self.ranking
    }

    pub fn entries(&self) -> &[HighScore] {
        self.scores.as_slice()
    }

    pub fn is_high_score(&self, new_score: u32) -> bool {
        self.try_get_score_index(new_score).is_some()
    }

    pub fn add_high_score(&mut self, new_score: HighScore) {
        let index = self
            .try_get_score_index(new_score.score)
            .expect("not a high score");
        self.scores.insert(index, new_score);
        if self.scores.len() > MAX_HIGH_SCORES {
            self.scores.pop();
        }
    }

    pub fn try_get_score_index(&self, new_score: u32) -> Option<usize> {
        match self
            .scores
            .iter()
            .enumerate()
            .find(|(_, s)| self.ranking.beats(new_score, s.score))
            .map(|(i, _)| i)
        {
            None if self.scores.len() < MAX_HIGH_SCORES => Some(self.scores.len()),
            Some(i) => Some(i),
            _ => None,
        }
    }

    fn sorted(&mut self) {
        match self.ranking {
            Ranking::HighestScore => self.scores.sort_by(|x, y| y.score.cmp(&x.score)),
            Ranking::LowestTime => self.scores.sort_by(|x, y| x.score.cmp(&y.score)),
        }
    }

    /// a table straight off disk into ranked shape
    fn normalise(&mut self) {
        self.sorted();
        self.scores.truncate(MAX_HIGH_SCORES);
    }
}

/// Every high score table in one `high_scores.yml`, structured by game and then mode. A
/// table only lands in the file once a score is entered into it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighScoreStore {
    /// game -> mode -> table
    tables: BTreeMap<String, BTreeMap<String, HighScoreTable>>,
}

impl HighScoreStore {
    pub fn load() -> Result<Self, String> {
        let config_path = config_path(CONFIG_NAME)?;
        let mut store: Self = match confy::load_or_else(&config_path, Self::default) {
            Ok(store) => store,
            Err(ConfyError::BadYamlData(error)) => {
                println!(
                    "Bad high score file at {}, {}, starting empty",
                    config_path.to_str().unwrap_or_default(),
                    error
                );
                Self::default()
            }
            Err(error) => return Err(error.to_string()),
        };
        for table in store.tables.values_mut().flat_map(|modes| modes.values_mut()) {
            table.normalise();
        }
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        let config_path = config_path(CONFIG_NAME)?;
        confy::store_path(config_path, self).map_err(|e| e.to_string())
    }

    /// the table for `key`: the stored one, or a fresh table when there is none yet or the
    /// stored one is ranked the other way (its entries would not be comparable)
    pub fn table(&self, key: &HighScoreKey) -> HighScoreTable {
        self.tables
            .get(&key.game)
            .and_then(|modes| modes.get(&key.mode))
            .filter(|table| table.ranking == key.ranking)
            .cloned()
            .unwrap_or_else(|| HighScoreTable::new(key.ranking))
    }

    pub fn set(&mut self, key: &HighScoreKey, table: HighScoreTable) {
        self.tables
            .entry(key.game.clone())
            .or_default()
            .insert(key.mode.clone(), table);
    }

    /// every stored table with entries, grouped by game then mode
    pub fn all(&self) -> impl Iterator<Item = (&str, &str, &HighScoreTable)> + '_ {
        self.tables.iter().flat_map(|(game, modes)| {
            modes
                .iter()
                .filter(|(_, table)| !table.scores.is_empty())
                .map(move |(mode, table)| (game.as_str(), mode.as_str(), table))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(scores: Vec<HighScore>) -> HighScoreTable {
        let mut result = HighScoreTable {
            ranking: Ranking::HighestScore,
            scores,
        };
        result.sorted();
        result
    }

    fn times(scores: Vec<HighScore>) -> HighScoreTable {
        let mut result = HighScoreTable {
            ranking: Ranking::LowestTime,
            scores,
        };
        result.sorted();
        result
    }

    #[test]
    fn a_new_time_table_has_long_default_times_to_beat() {
        let mut table = HighScoreTable::new(Ranking::LowestTime);
        assert_eq!(table.entries().len(), 5);
        assert!(table.is_high_score(90_000));
        assert!(!table.is_high_score(26 * 60_000));
        table.add_high_score(HighScore::new("A", 90_000));
        assert_eq!(table.entries()[0], HighScore::new("A", 90_000));
        assert_eq!(table.entries().len(), 5);
    }

    #[test]
    fn a_time_table_ranks_the_quickest_first() {
        let mut table = times(vec![
            HighScore::new("A", 60_000),
            HighScore::new("B", 90_000),
            HighScore::new("C", 120_000),
            HighScore::new("D", 150_000),
            HighScore::new("E", 180_000),
        ]);
        assert!(!table.is_high_score(180_000));
        assert!(!table.is_high_score(200_000));
        assert!(table.is_high_score(75_000));
        table.add_high_score(HighScore::new("new", 75_000));
        assert_eq!(
            table.entries().iter().map(|s| s.score).collect::<Vec<u32>>(),
            vec![60_000, 75_000, 90_000, 120_000, 150_000]
        );
    }

    #[test]
    fn a_time_table_sorts_ascending_when_loaded() {
        let table = times(vec![HighScore::new("B", 90_000), HighScore::new("A", 60_000)]);
        assert_eq!(table.entries()[0].name, "A");
    }

    #[test]
    fn millis_format_as_a_stopwatch() {
        assert_eq!(format_millis(0), "0:00.00");
        assert_eq!(format_millis(1_234), "0:01.23");
        assert_eq!(format_millis(61_999), "1:01.99");
        assert_eq!(format_millis(600_000), "10:00.00");
    }

    #[test]
    fn store_keeps_tables_by_game_and_mode() {
        let mut store = HighScoreStore::default();
        let key = HighScoreKey::new("Rustris", "1 level sprint, level 3", Ranking::LowestTime);
        // an unsaved table is the defaults, and a new entry is saved along with them
        assert_eq!(store.table(&key), HighScoreTable::new(Ranking::LowestTime));
        let mut table = store.table(&key);
        table.add_high_score(HighScore::new("A", 60_000));
        store.set(&key, table);
        let stored = store.table(&key);
        assert_eq!(stored.entries()[0], HighScore::new("A", 60_000));
        assert_eq!(stored.entries()[1..], HighScoreTable::new(Ranking::LowestTime).entries()[..4]);
        let all = store.all().collect::<Vec<_>>();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "Rustris");
        assert_eq!(all[0].1, "1 level sprint, level 3");
    }

    #[test]
    fn empty_tables_are_not_listed() {
        let mut store = HighScoreStore::default();
        let key = HighScoreKey::new("Rustris", "theme sprint, level 0", Ranking::LowestTime);
        store.set(&key, times(vec![]));
        assert_eq!(store.all().count(), 0);
    }

    #[test]
    fn a_table_ranked_the_other_way_is_not_comparable() {
        let mut store = HighScoreStore::default();
        let scores = HighScoreKey::new("versus", "theme race, easy", Ranking::HighestScore);
        store.set(&scores, HighScoreTable::new(Ranking::HighestScore));
        let times = HighScoreKey::new("versus", "theme race, easy", Ranking::LowestTime);
        assert_eq!(store.table(&times), HighScoreTable::new(Ranking::LowestTime));
        assert_eq!(store.table(&scores), HighScoreTable::new(Ranking::HighestScore));
    }

    #[test]
    fn adds_score_to_empty_table() {
        let mut table = new(vec![]);
        assert!(table.is_high_score(0));
        table.add_high_score(HighScore::new("A", 0));
        assert_eq!(table.scores, vec![HighScore::new("A", 0)]);
    }

    #[test]
    fn adds_score_to_bottom() {
        let mut table = new(vec![HighScore::new("A", 1)]);
        assert!(table.is_high_score(0));
        table.add_high_score(HighScore::new("B", 0));
        assert_eq!(
            table.scores,
            vec![HighScore::new("A", 1), HighScore::new("B", 0)]
        );
    }

    #[test]
    fn adds_score_to_top() {
        let mut table = new(vec![HighScore::new("A", 0)]);
        assert!(table.is_high_score(1));
        table.add_high_score(HighScore::new("B", 1));
        assert_eq!(
            table.scores,
            vec![HighScore::new("B", 1), HighScore::new("A", 0)]
        );
    }

    #[test]
    fn not_a_high_score() {
        let table = new(vec![
            HighScore::new("A", 10),
            HighScore::new("B", 9),
            HighScore::new("C", 8),
            HighScore::new("D", 7),
            HighScore::new("E", 6),
        ]);
        assert!(!table.is_high_score(6));
    }

    #[test]
    fn inserts_new_high_score_in_middle() {
        let mut table = new(vec![
            HighScore::new("A", 10),
            HighScore::new("B", 9),
            HighScore::new("C", 8),
            HighScore::new("D", 7),
            HighScore::new("E", 6),
        ]);
        assert!(table.is_high_score(8));
        table.add_high_score(HighScore::new("new", 8));
        assert_eq!(
            table.scores,
            vec![
                HighScore::new("A", 10),
                HighScore::new("B", 9),
                HighScore::new("C", 8),
                HighScore::new("new", 8),
                HighScore::new("D", 7)
            ]
        );
    }

    #[test]
    fn inserts_new_high_score_at_top() {
        let mut table = new(vec![
            HighScore::new("A", 10),
            HighScore::new("B", 9),
            HighScore::new("C", 8),
            HighScore::new("D", 7),
            HighScore::new("E", 6),
        ]);
        assert!(table.is_high_score(11));
        table.add_high_score(HighScore::new("new", 11));
        assert_eq!(
            table.scores,
            vec![
                HighScore::new("new", 11),
                HighScore::new("A", 10),
                HighScore::new("B", 9),
                HighScore::new("C", 8),
                HighScore::new("D", 7)
            ]
        );
    }

    #[test]
    fn inserts_new_high_score_at_bottom() {
        let mut table = new(vec![
            HighScore::new("A", 10),
            HighScore::new("B", 9),
            HighScore::new("C", 8),
            HighScore::new("D", 7),
            HighScore::new("E", 6),
        ]);
        assert!(table.is_high_score(7));
        table.add_high_score(HighScore::new("new", 7));
        assert_eq!(
            table.scores,
            vec![
                HighScore::new("A", 10),
                HighScore::new("B", 9),
                HighScore::new("C", 8),
                HighScore::new("D", 7),
                HighScore::new("new", 7)
            ]
        );
    }
}
