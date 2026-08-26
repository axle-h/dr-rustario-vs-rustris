use crate::game::ai::input_sequence::{InputSequence, Translation};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
use std::str::FromStr;

/// Represents a single recorded game input with decision index information
#[derive(Debug, Clone)]
pub struct RecordedInput {
    /// The game input that was triggered, None if no decision was made
    pub keys: Option<InputSequence>,
    /// Whether this decision uses the "alt" (held) piece
    pub is_alt: bool,
}

// Custom serialization for RecordedInput
impl Serialize for RecordedInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.keys {
            Some(keys) => {
                // Format keys as comma-separated list
                let keys_str: Vec<String> = keys.iter().map(|key| key.to_string()).collect();
                let keys_part = keys_str.join(",");

                // For alt decisions: "alt:{key1},{key2}"
                // For non alt decisions just "{key1},{key2}"
                let serialized = if self.is_alt {
                    format!("alt:{}", keys_part)
                } else {
                    keys_part
                };
                serializer.serialize_str(&serialized)
            }
            None => {
                // For "null" decisions use "null"
                serializer.serialize_str("null")
            }
        }
    }
}

// Custom deserialization for RecordedInput
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
                    // Null decision
                    return Ok(RecordedInput {
                        keys: None,
                        is_alt: false,
                    });
                }

                let (is_alt, key_part) = if value.starts_with("alt:") {
                    (true, &value[4..]) // Skip past "alt:"
                } else {
                    (false, value)
                };

                // Empty key_part (either "" or "alt:") means empty keys vector
                if key_part.is_empty() {
                    return Ok(RecordedInput {
                        keys: Some(InputSequence::empty()),
                        is_alt,
                    });
                }

                // Parse keys
                let mut keys = Vec::new();
                for key_str in key_part.split(',') {
                    // Use strum's EnumString trait to parse the key
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

/// Manages the recording of a game session
pub struct GameRecording {
    /// The recorded inputs/decisions
    inputs: Vec<RecordedInput>,
}

impl GameRecording {
    /// Create a new GameRecorder in inactive state
    pub fn new() -> Self {
        Self { inputs: Vec::new() }
    }

    /// Record a complete decision made by the agent
    pub fn record_decision(&mut self, keys: InputSequence, is_alt: bool) {
        let input = RecordedInput {
            keys: Some(keys),
            is_alt,
        };
        self.inputs.push(input);
    }

    /// Record a null decision (when no action was possible)
    pub fn record_null_decision(&mut self) {
        let input = RecordedInput {
            keys: None,
            is_alt: false,
        };
        self.inputs.push(input);
    }

    /// Save the recording to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        // Serialize the entire inputs vector using serde_json
        serde_json::to_writer(writer, &self.inputs)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    /// Get the recorded inputs
    pub fn inputs(&self) -> &[RecordedInput] {
        &self.inputs
    }
}

/// For playing back a recorded game
pub struct GamePlayback {
    /// The recorded inputs
    inputs: Vec<RecordedInput>,
    /// Current position in the playback
    current_index: usize,
}

impl GamePlayback {
    /// Load a recording from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Deserialize the inputs vector from the file using serde_json
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

    /// Process the next decision
    /// Returns the next RecordedInput, or None if playback is finished
    pub fn next_decision(&mut self) -> Option<RecordedInput> {
        if self.is_finished() {
            return None;
        }

        // Get the current decision and clone it
        let result = self.inputs[self.current_index].clone();

        // Move to the next decision
        self.current_index += 1;

        Some(result)
    }

    /// Check if playback has finished
    pub fn is_finished(&self) -> bool {
        self.current_index >= self.inputs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // Helper function to create a temporary file path
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

        // Record some decisions
        recorder.record_decision(InputSequence::new(vec![Translation::Left]), false);
        recorder.record_decision(
            InputSequence::new(vec![Translation::RotateClockwise]),
            false,
        );

        // Inputs should now be in the inputs vector
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

        // Record a null decision
        recorder.record_null_decision();

        // Inputs should now contain a single RecordedInput with keys set to None
        assert_eq!(recorder.inputs.len(), 1);
        assert!(recorder.inputs[0].keys.is_none());
    }

    #[test]
    fn test_game_recorder_save_and_load() -> io::Result<()> {
        let mut recorder = GameRecording::new();

        // Record some inputs with timing
        recorder.record_decision(InputSequence::new(vec![Translation::Left]), false);
        recorder.record_decision(
            InputSequence::new(vec![Translation::RotateClockwise]),
            false,
        );
        recorder.record_decision(InputSequence::new(vec![Translation::HardDrop]), false);

        // Save to a temporary file
        let file_path = temp_file_path();
        recorder.save_to_file(&file_path)?;

        // Load the recording
        let player = GamePlayback::load_from_file(&file_path)?;

        // Check that the loaded inputs match what we recorded
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

        // Clean up
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

        // Process the first decision
        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::Left);
        assert!(!keys.is_alt);
        assert_eq!(player.current_index, 1);

        // Process the second decision
        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::RotateClockwise);
        assert!(!keys.is_alt);
        assert_eq!(player.current_index, 2);

        // Process the third decision
        let keys = player.next_decision().unwrap();
        assert_eq!(keys.keys.as_ref().unwrap().len(), 1);
        assert_eq!(keys.keys.as_ref().unwrap()[0], Translation::HardDrop);
        assert!(!keys.is_alt);
        assert_eq!(player.current_index, 3);

        // No more decisions should be available
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

        // Consume all inputs
        player.next_decision();
        player.next_decision();
        assert!(player.is_finished());

        // Reset the player
        player.reset();

        // Check that the state is reset
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

        // Initially not finished
        assert!(!player.is_finished());

        // After consuming all inputs
        player.next_decision();
        assert!(player.is_finished());

        // Empty player is finished
        let empty_player = GamePlayback {
            inputs: vec![],
            current_index: 0,
        };
        assert!(empty_player.is_finished());
    }

    #[test]
    fn test_grouped_keys() {
        let mut recorder = GameRecording::new();

        // Record multiple inputs before updating
        recorder.record_decision(InputSequence::new(vec![Translation::Left]), false);
        recorder.record_decision(InputSequence::new(vec![Translation::Right]), false);
        recorder.record_decision(InputSequence::new(vec![Translation::HardDrop]), false);

        // All three inputs should be in separate RecordedInput instances
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

        // Create a player with this input
        let mut player = GamePlayback {
            inputs: recorder.inputs().to_vec(),
            current_index: 0,
        };

        // Process the next decision and check that the correct key is returned
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
