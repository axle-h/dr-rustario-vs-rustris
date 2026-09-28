use crate::game::ai::input_sequence::{InputSequence, Translation};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
use std::str::FromStr;

/// One recorded decision.
#[derive(Debug, Clone)]
pub struct RecordedInput {
    /// the inputs, `None` if no decision was made
    pub keys: Option<InputSequence>,
    /// whether this decision plays the held piece
    pub is_alt: bool,
}

// Serialised as a string: `key1,key2`, `alt:key1,key2` for the held piece, or `null`.
impl Serialize for RecordedInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.keys {
            Some(keys) => {
                let keys_str: Vec<String> = keys.iter().map(|key| key.to_string()).collect();
                let keys_part = keys_str.join(",");

                let serialized = if self.is_alt {
                    format!("alt:{}", keys_part)
                } else {
                    keys_part
                };
                serializer.serialize_str(&serialized)
            }
            None => serializer.serialize_str("null"),
        }
    }
}

impl<'de> Deserialize<'de> for RecordedInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RecordedInputVisitor;

        impl<'de> serde::de::Visitor<'de> for RecordedInputVisitor {
            type Value = RecordedInput;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in format 'key1,key2,...', 'alt:key1,key2,...', '' (empty), 'alt:', or 'null'")
            }

            fn visit_str<E>(self, value: &str) -> Result<RecordedInput, E>
            where
                E: serde::de::Error,
            {
                if value == "null" {
                    return Ok(RecordedInput {
                        keys: None,
                        is_alt: false,
                    });
                }

                let (is_alt, key_part) = match value.strip_prefix("alt:") {
                    Some(key_part) => (true, key_part),
                    None => (false, value),
                };

                // `""` and `"alt:"` are an empty sequence
                if key_part.is_empty() {
                    return Ok(RecordedInput {
                        keys: Some(InputSequence::empty()),
                        is_alt,
                    });
                }

                let mut keys = Vec::new();
                for key_str in key_part.split(',') {
                    match Translation::from_str(key_str) {
                        Ok(key) => keys.push(key),
                        Err(_) => return Err(E::custom(format!("invalid key: {}", key_str))),
                    }
                }

                Ok(RecordedInput {
                    keys: Some(InputSequence::new(keys)),
                    is_alt,
                })
            }
        }

        deserializer.deserialize_str(RecordedInputVisitor)
    }
}

/// A recorded game session.
pub struct GameRecording {
    inputs: Vec<RecordedInput>,
}

impl Default for GameRecording {
    fn default() -> Self {
        Self::new()
    }
}

impl GameRecording {
    pub fn new() -> Self {
        Self { inputs: Vec::new() }
    }

    pub fn record_decision(&mut self, keys: InputSequence, is_alt: bool) {
        let input = RecordedInput {
            keys: Some(keys),
            is_alt,
        };
        self.inputs.push(input);
    }

    /// Record that no action was possible.
    pub fn record_null_decision(&mut self) {
        let input = RecordedInput {
            keys: None,
            is_alt: false,
        };
        self.inputs.push(input);
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        serde_json::to_writer(writer, &self.inputs).map_err(|e| io::Error::other(e.to_string()))?;

        Ok(())
    }

    pub fn inputs(&self) -> &[RecordedInput] {
        &self.inputs
    }
}

/// Plays back a recorded game.
pub struct GamePlayback {
    inputs: Vec<RecordedInput>,
    current_index: usize,
}

impl GamePlayback {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let inputs: Vec<RecordedInput> = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        Ok(Self {
            inputs,
            current_index: 0,
        })
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// The next decision, or `None` once playback is finished.
    pub fn next_decision(&mut self) -> Option<RecordedInput> {
        if self.is_finished() {
            return None;
        }

        let result = self.inputs[self.current_index].clone();

        self.current_index += 1;

        Some(result)
    }

    pub fn is_finished(&self) -> bool {
        self.current_index >= self.inputs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_file_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "rustris_test_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        path
    }

    #[test]
    fn test_game_recorder_new() {
        let recorder = GameRecording::new();
        assert!(recorder.inputs.is_empty());
    }

    #[test]
    fn test_game_recorder_record_decision() {
        let mut recorder = GameRecording::new();

        recorder.record_decision(InputSequence::new(vec![Translation::Left]), false);
        recorder.record_decision(
            InputSequence::new(vec![Translation::RotateClockwise]),
            false,
        );

        assert_eq!(recorder.inputs.len(), 2);
        assert_eq!(recorder.inputs[0].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            recorder.inputs[0].keys.as_ref().unwrap()[0],
            Translation::Left
        );
        assert_eq!(recorder.inputs[1].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            recorder.inputs[1].keys.as_ref().unwrap()[0],
            Translation::RotateClockwise
        );
    }

    #[test]
    fn test_game_recorder_record_null_decision() {
        let mut recorder = GameRecording::new();

        recorder.record_null_decision();

        assert_eq!(recorder.inputs.len(), 1);
        assert!(recorder.inputs[0].keys.is_none());
    }

    #[test]
    fn test_game_recorder_save_and_load() -> io::Result<()> {
        let mut recorder = GameRecording::new();

        recorder.record_decision(InputSequence::new(vec![Translation::Left]), false);
        recorder.record_decision(
            InputSequence::new(vec![Translation::RotateClockwise]),
            false,
        );
        recorder.record_decision(InputSequence::new(vec![Translation::HardDrop]), false);

        let file_path = temp_file_path();
        recorder.save_to_file(&file_path)?;

        let player = GamePlayback::load_from_file(&file_path)?;

        assert_eq!(player.inputs.len(), 3);
        assert_eq!(player.inputs[0].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            player.inputs[0].keys.as_ref().unwrap()[0],
            Translation::Left
        );
        assert_eq!(player.inputs[1].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            player.inputs[1].keys.as_ref().unwrap()[0],
            Translation::RotateClockwise
        );
        assert_eq!(player.inputs[2].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            player.inputs[2].keys.as_ref().unwrap()[0],
            Translation::HardDrop
        );

        fs::remove_file(&file_path)?;

        Ok(())
    }

    #[test]
    fn test_game_player_update() {
        let mut player = GamePlayback {
            inputs: vec![
                RecordedInput {
                    keys: Some(InputSequence::new(vec![Translation::Left])),
                    is_alt: false,
                },
                RecordedInput {
                    keys: Some(InputSequence::new(vec![Translation::RotateClockwise])),
                    is_alt: false,
                },
                RecordedInput {
                    keys: Some(InputSequence::new(vec![Translation::HardDrop])),
                    is_alt: false,
                },
            ],
            current_index: 0,
        };

        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::Left);
        assert!(!keys.is_alt);
        assert_eq!(player.current_index, 1);

        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::RotateClockwise);
        assert!(!keys.is_alt);
        assert_eq!(player.current_index, 2);

        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::HardDrop);
        assert!(!keys.is_alt);
        assert_eq!(player.current_index, 3);

        let result = player.next_decision();
        assert!(result.is_none());
        assert!(player.is_finished());
    }

    #[test]
    fn test_game_player_reset() {
        let mut player = GamePlayback {
            inputs: vec![
                RecordedInput {
                    keys: Some(InputSequence::new(vec![Translation::Left])),
                    is_alt: false,
                },
                RecordedInput {
                    keys: Some(InputSequence::new(vec![Translation::RotateClockwise])),
                    is_alt: false,
                },
            ],
            current_index: 0,
        };

        player.next_decision();
        player.next_decision();
        assert!(player.is_finished());

        player.reset();

        assert_eq!(player.current_index, 0);
        assert!(!player.is_finished());
    }

    #[test]
    fn test_game_player_is_finished() {
        let mut player = GamePlayback {
            inputs: vec![RecordedInput {
                keys: Some(InputSequence::new(vec![Translation::Left])),
                is_alt: false,
            }],
            current_index: 0,
        };

        assert!(!player.is_finished());

        player.next_decision();
        assert!(player.is_finished());

        let empty_player = GamePlayback {
            inputs: vec![],
            current_index: 0,
        };
        assert!(empty_player.is_finished());
    }

    #[test]
    fn test_grouped_keys() {
        let mut recorder = GameRecording::new();

        recorder.record_decision(InputSequence::new(vec![Translation::Left]), false);
        recorder.record_decision(InputSequence::new(vec![Translation::Right]), false);
        recorder.record_decision(InputSequence::new(vec![Translation::HardDrop]), false);

        assert_eq!(recorder.inputs.len(), 3);
        assert_eq!(recorder.inputs[0].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            recorder.inputs[0].keys.as_ref().unwrap()[0],
            Translation::Left
        );
        assert_eq!(recorder.inputs[1].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            recorder.inputs[1].keys.as_ref().unwrap()[0],
            Translation::Right
        );
        assert_eq!(recorder.inputs[2].keys.as_ref().unwrap().len(), 1);
        assert_eq!(
            recorder.inputs[2].keys.as_ref().unwrap()[0],
            Translation::HardDrop
        );

        let mut player = GamePlayback {
            inputs: recorder.inputs().to_vec(),
            current_index: 0,
        };

        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::Left);

        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::Right);

        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::HardDrop);
    }
}
