//! Mean Bean Machine's thirteen characters, in the box the game keeps its mugshot in.
//!
//! Cut and measured from captures of the emulated game by `puyo-rusto/art/mugshots.py`, one png
//! per character so a face's texture is built only when it is dealt. Each row is cut action
//! first and rest last, since [`FrameAnimationType::LinearWithPause`] holds the last frame.

use engine::animate::character::{EmitterSource, EmitterTrigger};
use engine::animate::frames::FrameAnimationType;
use engine::render::character::{CharacterData, CharacterSetData, EmitterData, LayerData};
use std::f64::consts::FRAC_1_SQRT_2 as DIAG;
use std::time::Duration;

mod sprites {
    pub const FRANKLY: &[u8] = include_bytes!("mugshots/frankly.png");
    pub const ARMS: &[u8] = include_bytes!("mugshots/arms.png");
    pub const HUMPTY: &[u8] = include_bytes!("mugshots/humpty.png");
    pub const COCONUTS: &[u8] = include_bytes!("mugshots/coconuts.png");
    pub const DAVY: &[u8] = include_bytes!("mugshots/davy.png");
    pub const SKWEEL: &[u8] = include_bytes!("mugshots/skweel.png");
    pub const DYNAMIGHT: &[u8] = include_bytes!("mugshots/dynamight.png");
    pub const GROUNDER: &[u8] = include_bytes!("mugshots/grounder.png");
    pub const SPIKE: &[u8] = include_bytes!("mugshots/spike.png");
    pub const FFUZZY: &[u8] = include_bytes!("mugshots/ffuzzy.png");
    pub const DRAGON: &[u8] = include_bytes!("mugshots/dragon.png");
    pub const SCRATCH: &[u8] = include_bytes!("mugshots/scratch.png");
    pub const ROBOTNIK: &[u8] = include_bytes!("mugshots/robotnik.png");
}

/// The size of one frame, which is exactly the `MUGSHOT` hole in the panel.
pub const FRAME: (u32, u32) = (80, 56);
/// the spacing of a character png's rows
pub const ROW_PITCH: u32 = FRAME.1 + 1;

/// The sweat's dial: above this much of the board filled, at this many drops a second when it
/// is completely full. Kept above `DANGER_ENTER`, so the losing face comes on before the sweat.
const SWEAT_TRIGGER: EmitterTrigger = EmitterTrigger::Danger {
    above: 0.55,
    per_second: 10.0,
};

/// The cast, in the order the sheet draws them.
pub const CAST: &[CharacterData] = &[
    // frankly
    CharacterData {
        name: "Frankly",
        file: sprites::FRANKLY,
        states: [
            (2, FrameAnimationType::Linear { fps: 3 }),
            (1, FrameAnimationType::Static),
            (2, FrameAnimationType::Linear { fps: 5 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[
            // antenna sparks
            EmitterData {
                origin: (0, 237),
                size: (8, 16),
                frames: 4,
                fps: 14,
                triggers: [
                    None,
                    Some(EmitterTrigger::Every(Duration::from_millis(700))),
                    None,
                    None,
                ],
                sources: &[
                    EmitterSource {
                        at: (6.0, 15.0),
                        directions: &[(-DIAG, -DIAG), (DIAG, -DIAG), (-DIAG, DIAG)],
                    },
                    EmitterSource {
                        at: (77.0, 15.0),
                        directions: &[(DIAG, -DIAG), (-DIAG, -DIAG), (DIAG, DIAG)],
                    },
                ],
                speed: 1.8,
                life: Duration::from_millis(280),
                fade_last: 0.35,
            },
        ],
    },
    // arms
    CharacterData {
        name: "Arms",
        file: sprites::ARMS,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 8,
                    pause_for: Duration::from_millis(690),
                    resume_from_frame: 0,
                },
            ),
            (4, FrameAnimationType::Linear { fps: 7 }),
            (2, FrameAnimationType::Linear { fps: 6 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[
            // rim lights
            LayerData {
                origin: (0, 237),
                size: (80, 12),
                row_pitch: 13,
                states: [
                    (14, FrameAnimationType::Linear { fps: 8 }),
                    (8, FrameAnimationType::Linear { fps: 24 }),
                    (8, FrameAnimationType::Linear { fps: 25 }),
                    (0, FrameAnimationType::Static),
                ],
                anchors: &[(0, 44)],
                wander: None,
            },
        ],
        emitters: &[],
    },
    // humpty
    CharacterData {
        name: "Humpty",
        file: sprites::HUMPTY,
        states: [
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 3,
                    pause_for: Duration::from_millis(683),
                    resume_from_frame: 0,
                },
            ),
            (
                12,
                FrameAnimationType::LinearWithPause {
                    fps: 10,
                    pause_for: Duration::from_millis(1940),
                    resume_from_frame: 0,
                },
            ),
            (4, FrameAnimationType::Linear { fps: 6 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[
            // arc
            LayerData {
                origin: (0, 237),
                size: (24, 8),
                row_pitch: 9,
                states: [
                    (
                        3,
                        FrameAnimationType::LinearWithPause {
                            fps: 30,
                            pause_for: Duration::from_millis(450),
                            resume_from_frame: 0,
                        },
                    ),
                    (0, FrameAnimationType::Static),
                    (0, FrameAnimationType::Static),
                    (0, FrameAnimationType::Static),
                ],
                anchors: &[(17, 5), (25, 5), (33, 6), (41, 6)],
                wander: Some(Duration::from_millis(550)),
            },
            // wrung hands
            LayerData {
                origin: (0, 273),
                size: (24, 16),
                row_pitch: 17,
                states: [
                    (0, FrameAnimationType::Static),
                    (0, FrameAnimationType::Static),
                    (3, FrameAnimationType::Linear { fps: 7 }),
                    (0, FrameAnimationType::Static),
                ],
                anchors: &[(18, 24), (18, 27), (50, 32), (56, 34), (57, 27), (63, 31)],
                wander: Some(Duration::from_millis(140)),
            },
        ],
        emitters: &[
            // antenna bolts
            EmitterData {
                origin: (0, 341),
                size: (8, 16),
                frames: 3,
                fps: 30,
                triggers: [None, Some(EmitterTrigger::OnFrame(&[0, 6])), None, None],
                sources: &[
                    EmitterSource {
                        at: (28.5, 7.5),
                        directions: &[(-DIAG, -DIAG)],
                    },
                    EmitterSource {
                        at: (50.5, 7.5),
                        directions: &[(DIAG, -DIAG)],
                    },
                ],
                speed: 2.2,
                life: Duration::from_millis(100),
                fade_last: 0.30,
            },
        ],
    },
    // coconuts
    CharacterData {
        name: "Coconuts",
        file: sprites::COCONUTS,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 6,
                    pause_for: Duration::from_millis(1013),
                    resume_from_frame: 0,
                },
            ),
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 2,
                    pause_for: Duration::from_millis(360),
                    resume_from_frame: 0,
                },
            ),
            (4, FrameAnimationType::Linear { fps: 12 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[
            // coin
            LayerData {
                origin: (0, 237),
                size: (17, 12),
                row_pitch: 13,
                states: [
                    (0, FrameAnimationType::Static),
                    (14, FrameAnimationType::Linear { fps: 23 }),
                    (6, FrameAnimationType::Linear { fps: 15 }),
                    (0, FrameAnimationType::Static),
                ],
                anchors: &[(44, 0)],
                wander: None,
            },
        ],
        emitters: &[],
    },
    // davy
    CharacterData {
        name: "Davy Sprocket",
        file: sprites::DAVY,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 6,
                    pause_for: Duration::from_millis(1813),
                    resume_from_frame: 0,
                },
            ),
            (2, FrameAnimationType::Linear { fps: 7 }),
            (2, FrameAnimationType::Linear { fps: 6 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // skweel
    CharacterData {
        name: "Skweel",
        file: sprites::SKWEEL,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 10,
                    pause_for: Duration::from_millis(870),
                    resume_from_frame: 0,
                },
            ),
            (4, FrameAnimationType::Linear { fps: 6 }),
            (6, FrameAnimationType::Linear { fps: 9 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // dynamight
    CharacterData {
        name: "Dynamight",
        file: sprites::DYNAMIGHT,
        states: [
            (2, FrameAnimationType::Linear { fps: 3 }),
            (5, FrameAnimationType::Linear { fps: 11 }),
            (2, FrameAnimationType::Linear { fps: 8 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // grounder
    CharacterData {
        name: "Grounder",
        file: sprites::GROUNDER,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 10,
                    pause_for: Duration::from_millis(1930),
                    resume_from_frame: 0,
                },
            ),
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 5,
                    pause_for: Duration::from_millis(760),
                    resume_from_frame: 0,
                },
            ),
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 4,
                    pause_for: Duration::from_millis(1120),
                    resume_from_frame: 0,
                },
            ),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // spike
    CharacterData {
        name: "Spike",
        file: sprites::SPIKE,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 8,
                    pause_for: Duration::from_millis(1900),
                    resume_from_frame: 0,
                },
            ),
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 5,
                    pause_for: Duration::from_millis(1540),
                    resume_from_frame: 0,
                },
            ),
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 6,
                    pause_for: Duration::from_millis(1323),
                    resume_from_frame: 0,
                },
            ),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // ffuzzy
    CharacterData {
        name: "Sir Ffuzzy-Logik",
        file: sprites::FFUZZY,
        states: [
            (4, FrameAnimationType::Linear { fps: 7 }),
            (4, FrameAnimationType::Linear { fps: 6 }),
            (4, FrameAnimationType::Linear { fps: 10 }),
            (4, FrameAnimationType::Linear { fps: 7 }),
        ],
        routines: None,
        layers: &[
            // eye yellow
            LayerData {
                origin: (0, 237),
                size: (25, 15),
                row_pitch: 16,
                states: [
                    (8, FrameAnimationType::Linear { fps: 7 }),
                    (8, FrameAnimationType::Linear { fps: 15 }),
                    (0, FrameAnimationType::Static),
                    (8, FrameAnimationType::Linear { fps: 7 }),
                ],
                anchors: &[(25, 16)],
                wander: None,
            },
            // eyes
            LayerData {
                origin: (0, 301),
                size: (32, 24),
                row_pitch: 25,
                states: [
                    (0, FrameAnimationType::Static),
                    (0, FrameAnimationType::Static),
                    (
                        4,
                        FrameAnimationType::LinearWithPause {
                            fps: 15,
                            pause_for: Duration::from_millis(933),
                            resume_from_frame: 0,
                        },
                    ),
                    (0, FrameAnimationType::Static),
                ],
                anchors: &[(24, 8)],
                wander: None,
            },
        ],
        emitters: &[],
    },
    // dragon
    CharacterData {
        name: "Dragon Breath",
        file: sprites::DRAGON,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 8,
                    pause_for: Duration::from_millis(1430),
                    resume_from_frame: 0,
                },
            ),
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 3,
                    pause_for: Duration::from_millis(873),
                    resume_from_frame: 0,
                },
            ),
            (
                7,
                FrameAnimationType::LinearWithPause {
                    fps: 11,
                    pause_for: Duration::from_millis(964),
                    resume_from_frame: 0,
                },
            ),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // scratch
    CharacterData {
        name: "Scratch",
        file: sprites::SCRATCH,
        states: [
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 2,
                    pause_for: Duration::from_millis(290),
                    resume_from_frame: 0,
                },
            ),
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 5,
                    pause_for: Duration::from_millis(710),
                    resume_from_frame: 0,
                },
            ),
            (
                2,
                FrameAnimationType::LinearWithPause {
                    fps: 5,
                    pause_for: Duration::from_millis(700),
                    resume_from_frame: 0,
                },
            ),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
    // robotnik
    CharacterData {
        name: "Dr. Robotnik",
        file: sprites::ROBOTNIK,
        states: [
            (
                4,
                FrameAnimationType::LinearWithPause {
                    fps: 11,
                    pause_for: Duration::from_millis(1286),
                    resume_from_frame: 0,
                },
            ),
            (2, FrameAnimationType::Linear { fps: 7 }),
            (2, FrameAnimationType::Linear { fps: 10 }),
            (1, FrameAnimationType::Static),
        ],
        routines: None,
        layers: &[],
        emitters: &[],
    },
];

pub fn characters() -> CharacterSetData {
    CharacterSetData {
        characters: CAST,
        frame_size: FRAME,
        row_pitch: ROW_PITCH,
        sweat: Some(EmitterData {
            origin: (0, 228),
            size: (8, 8),
            frames: 1,
            fps: 0,
            triggers: [None, None, Some(SWEAT_TRIGGER), None],
            sources: &[
                EmitterSource {
                    at: (10.0, 17.0),
                    directions: &[(-DIAG, -DIAG), (-0.4500, -0.8900)],
                },
                EmitterSource {
                    at: (70.0, 17.0),
                    directions: &[(DIAG, -DIAG), (0.4500, -0.8900)],
                },
            ],
            speed: 1.2,
            life: Duration::from_millis(600),
            fade_last: 0.40,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::animate::character::CharacterState;

    fn png_size(bytes: &[u8]) -> (u32, u32) {
        // a png's IHDR carries width and height as big-endian u32s at a fixed offset
        assert_eq!(&bytes[1..4], b"PNG", "not a png");
        let at =
            |o: usize| u32::from_be_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
        (at(16), at(20))
    }

    /// Every portrait row, layer and emitter each character declares lies inside its png.
    #[test]
    fn every_character_declares_the_art_it_actually_has() {
        let sweat = characters().sweat.expect("the cast lost its sweat");
        for character in CAST {
            let (width, height) = png_size(character.file);
            let name = character.name;
            let fits = |what: &str, right: i32, bottom: i32| {
                assert!(
                    right <= width as i32 && bottom <= height as i32,
                    "{name}'s {what} runs to ({right}, {bottom}) in a png that is {width}x{height}"
                );
            };
            for (state, (frames, _)) in character.states.iter().enumerate() {
                assert!(*frames > 0, "{name} has an empty row");
                fits(
                    "portrait",
                    (*frames as u32 * FRAME.0) as i32,
                    (state as u32 * ROW_PITCH + FRAME.1) as i32,
                );
            }
            let widest = character
                .states
                .iter()
                .map(|(frames, _)| *frames as u32 * FRAME.0)
                .max()
                .unwrap();
            assert!(
                width >= widest,
                "{name} is {width} wide, but its longest portrait row needs {widest}"
            );
            for layer in character.layers {
                for (state, (frames, _)) in layer.states.iter().enumerate() {
                    fits(
                        "layer",
                        layer.origin.0 + (*frames as u32 * layer.size.0) as i32,
                        layer.origin.1 + (state as u32 * layer.row_pitch + layer.size.1) as i32,
                    );
                }
                assert!(!layer.anchors.is_empty(), "{name} has an unanchored layer");
            }
            for emitter in character.emitters.iter().chain([&sweat]) {
                fits(
                    "emitter",
                    emitter.origin.0 + (emitter.frames as u32 * emitter.size.0) as i32,
                    emitter.origin.1 + emitter.size.1 as i32,
                );
                assert!(
                    emitter.frames > 0 && !emitter.sources.is_empty(),
                    "{name} has an emitter that can throw nothing"
                );
            }
        }
    }

    /// Every character's png has room for the shared sweat under its four portrait rows.
    #[test]
    fn every_character_carries_the_shared_sweat_at_the_same_place() {
        let sweat = characters().sweat.expect("the cast lost its sweat");
        assert_eq!(
            sweat.origin,
            (0, (4 * ROW_PITCH) as i32),
            "the sweat is not on the row under the four portrait rows"
        );
        for character in CAST {
            let (_, height) = png_size(character.file);
            assert!(
                height as i32 >= sweat.origin.1 + sweat.size.1 as i32,
                "{} was cut without room for the sweat",
                character.name
            );
        }
    }

    /// The sweat triggers in the losing state and no other.
    #[test]
    fn the_sweat_runs_on_the_losing_row_and_nowhere_else() {
        let sweat = characters().sweat.expect("the cast lost its sweat");
        for state in CharacterState::ALL {
            let trigger = sweat.triggers[state.index()];
            if state == CharacterState::Losing {
                assert!(trigger.is_some(), "nobody sweats when they are losing");
            } else {
                assert!(trigger.is_none(), "the sweat runs on {state:?}");
            }
        }
    }

    #[test]
    fn the_cast_is_the_whole_sheet_and_every_face_is_named() {
        assert_eq!(
            CAST.len(),
            13,
            "Mean Bean Machine draws thirteen characters"
        );
        let mut names: Vec<&str> = CAST.iter().map(|c| c.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "two characters share a name");
    }

    /// Sir Ffuzzy-Logik's defeat row animates, as in the game; every other defeat is held.
    #[test]
    fn only_sir_ffuzzy_logik_animates_after_he_is_buried() {
        for character in CAST {
            let (frames, animation) = character.states[3];
            if character.name == "Sir Ffuzzy-Logik" {
                assert!(
                    frames > 1 && animation.fps().is_some(),
                    "his defeat stopped moving"
                );
            } else {
                assert_eq!(
                    animation,
                    FrameAnimationType::Static,
                    "{}'s defeat animates, which only Sir Ffuzzy-Logik's does",
                    character.name
                );
            }
        }
    }
}
