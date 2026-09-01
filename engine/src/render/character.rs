//! The art behind [`crate::animate::character`]: a theme's cast, and the one that was dealt.
//!
//! **Built lazily, one character at a time.** A cast is thirteen faces on Mean Bean Machine and
//! at most two of them are ever on screen, so building all of them at startup would be building
//! eleven textures nobody looks at - and which one is dealt is not known until a match starts.
//! The bytes are still `include_bytes!`, so nothing is read from disk and a handheld pays no
//! file IO; what is deferred is the *texture*, which is the expensive half. This is the only
//! lazily built theme art in the compendium, and it is why a cast can grow without costing a
//! slow device anything.
//!
//! The `RefCell` behind a `&self` draw is the pattern the block atlas, the popup font's tint and
//! the animation sheet's own alpha all use already.

use crate::animate::character::{
    CharacterMeta, CharacterState, EmitterMeta, EmitterSource, EmitterTrigger, LayerMeta, Routine,
    RoutineChoice, RoutineMeta, RoutineWay,
};
use crate::animate::frames::FrameAnimationType;
use crate::render::animation::AnimationSpriteSheet;
use crate::render::helper::TextureFactory;
use sdl2::rect::Rect;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

/// One character: its own png, and how each of its four rows plays.
///
/// The png carries the four rows in **play order** with the fade dropped, one row per state,
/// frames edge to edge - which is how the strips are cut, so nothing here has to reorder
/// anything. See `puyo-rusto/art/mugshots.py`.
#[derive(Clone, Debug)]
pub struct CharacterData {
    pub name: &'static str,
    pub file: &'static [u8],
    /// (frames, how it plays) per [`CharacterState`], in `CharacterState::ALL` order.
    /// Ignored while `routines` is set.
    pub states: [(usize, FrameAnimationType); 4],
    /// poses addressed by rect and played as routines, rather than one strip a state:
    /// `None` on a mugshot cast, which is every character here but Kirby
    pub routines: Option<RoutineArt>,
    /// what is drawn over the portrait, from strips further down the same png
    pub layers: &'static [LayerData],
    /// what is thrown off it, likewise
    pub emitters: &'static [EmitterData],
}

/// The art behind [`RoutineMeta`]: where each pose is in the character's png.
///
/// Rects rather than a frame size and a count, because these frames are **not one size** - the
/// pose Kirby lands flat in is half the height of the one he stands in and a third the width of
/// the one he bobs up tall in. Nothing can be addressed by counting frame widths from a strip's
/// start here, so the cutter prints the table and the sheet is a readable grid rather than a
/// strip; see `puyo-rusto/art/kirby.py`.
#[derive(Clone, Copy, Debug)]
pub struct RoutineArt {
    /// every pose in the png, as its own rect
    pub poses: &'static [(i32, i32, u32, u32)],
    /// everything the character may be dealt, and how much each moves
    pub choices: &'static [RoutineChoice],
    /// played between routines rather than as one of them: the blink
    pub filler: Option<RoutineWay>,
    /// what a buried character does, held
    pub defeat: Routine,
    /// whether a routine may be walked into when he is not standing in its window
    pub approach: bool,
    /// how long the last frame is held before another routine is dealt
    pub rest: Duration,
    /// where a routine's opening frame goes in the box
    pub home: (i32, i32),
    /// how far either way `home.0` may be walked
    pub wander: (i32, i32),
    /// how much a dealing varies a routine's pace, as a fraction either way
    pub speed_spread: f64,
}

impl RoutineArt {
    fn meta(&self) -> RoutineMeta {
        RoutineMeta {
            choices: self.choices,
            filler: self.filler,
            defeat: self.defeat,
            approach: self.approach,
            rest: self.rest,
            home: self.home,
            wander: self.wander,
            speed_spread: self.speed_spread,
        }
    }
}

/// A layer's art: where its strips are in the character's png, and how big one frame is.
///
/// Four strips, one per state, on `row_pitch` - the same shape the portrait rows have, so a
/// layer that is a *palette cycle* on two rows and absent on the other two is declared the
/// same way as one that is a different sprite per row.
#[derive(Clone, Copy, Debug)]
pub struct LayerData {
    /// top left of the first strip, in the character's png
    pub origin: (i32, i32),
    pub size: (u32, u32),
    pub row_pitch: u32,
    pub states: [(usize, FrameAnimationType); 4],
    pub anchors: &'static [(i32, i32)],
    pub wander: Option<Duration>,
}

impl LayerData {
    fn meta(&self) -> LayerMeta {
        LayerMeta {
            states: self.states,
            anchors: self.anchors,
            wander: self.wander,
        }
    }
}

/// An emitter's art: one strip of the particle, wherever it is in the png.
#[derive(Clone, Copy, Debug)]
pub struct EmitterData {
    pub origin: (i32, i32),
    pub size: (u32, u32),
    pub frames: usize,
    pub fps: u32,
    pub triggers: [Option<EmitterTrigger>; 4],
    pub sources: &'static [EmitterSource],
    /// box pixels per axis per 60 Hz frame
    pub speed: f64,
    pub life: Duration,
    pub fade_last: f64,
}

impl EmitterData {
    fn meta(&self) -> EmitterMeta {
        EmitterMeta {
            triggers: self.triggers,
            sources: self.sources,
            speed: self.speed,
            life: self.life,
            fade_last: self.fade_last,
            frames: self.frames,
            fps: self.fps,
        }
    }
}

impl CharacterData {
    /// The meta a `CharacterSet` would hand out, without a set - for a theme's own tests.
    pub fn meta_for_test(&self) -> CharacterMeta {
        self.meta(None)
    }

    /// The layers and emitters this character has, with the cast-wide one appended.
    ///
    /// The sweat is **last** in both lists so that a character's own emitters keep the indices
    /// their table gives them, and so a cast that declares none still has it.
    fn meta(&self, shared: Option<&EmitterData>) -> CharacterMeta {
        CharacterMeta {
            states: self.states,
            routines: self.routines.map(|r| r.meta()),
            layers: self.layers.iter().map(|l| l.meta()).collect(),
            emitters: self
                .emitters
                .iter()
                .chain(shared)
                .map(|e| e.meta())
                .collect(),
        }
    }
}

/// A theme's whole cast, and the shape of the sheets it cut them onto.
#[derive(Clone, Debug)]
pub struct CharacterSetData {
    pub characters: &'static [CharacterData],
    /// one frame, which for a retro theme is the box the game drew
    pub frame_size: (u32, u32),
    /// rows of the png are spaced by this, so a person can read the sheet
    pub row_pitch: u32,
    /// an emitter **every** character has, whose art the cutter writes into every png at the
    /// same place - the sweat, which is not anybody's own and is measured, not drawn: the same
    /// character sweats in one clip and not another, and no rip carries a drop at all
    pub sweat: Option<EmitterData>,
}

/// Where a character is drawn, in the theme's own source pixels.
#[derive(Clone, Copy, Debug)]
pub struct CharacterLayout {
    pub rect: Rect,
}

/// One character's frames, as one texture with a base index per state.
///
/// One texture rather than four: the four strips are rows of the same png, so loading it once
/// and keeping a rect per frame costs a quarter of what four sheets would.
pub struct CharacterSprites<'a> {
    sheet: AnimationSpriteSheet<'a>,
    base: [usize; 4],
    /// where each layer's four strips start in `sheet`
    layers: Vec<[usize; 4]>,
    /// where each emitter's strip starts, the cast-wide one last
    emitters: Vec<usize>,
}

impl<'a> CharacterSprites<'a> {
    pub fn draw(
        &self,
        canvas: &mut WindowCanvas,
        dest: Rect,
        state: CharacterState,
        frame: usize,
        mirrored: bool,
    ) -> Result<(), String> {
        let index = self.base[state.index()] + frame;
        self.sheet
            .draw_frame_scaled_flipped(canvas, dest, index, mirrored)
    }

    /// One pose, at whatever rect the caller worked out from its own size and its place in
    /// the box - which is the routine path, and the one draw here that does not fill the box.
    pub fn draw_pose(
        &self,
        canvas: &mut WindowCanvas,
        dest: Rect,
        pose: usize,
        mirrored: bool,
    ) -> Result<(), String> {
        self.sheet
            .draw_frame_scaled_flipped(canvas, dest, pose, mirrored)
    }

    pub fn draw_layer(
        &self,
        canvas: &mut WindowCanvas,
        dest: Rect,
        layer: usize,
        state: CharacterState,
        frame: usize,
        mirrored: bool,
    ) -> Result<(), String> {
        let Some(base) = self.layers.get(layer) else {
            return Ok(());
        };
        self.sheet
            .draw_frame_scaled_flipped(canvas, dest, base[state.index()] + frame, mirrored)
    }

    pub fn draw_particle(
        &self,
        canvas: &mut WindowCanvas,
        dest: Rect,
        emitter: usize,
        frame: usize,
        mirrored: bool,
        alpha: u8,
    ) -> Result<(), String> {
        let Some(base) = self.emitters.get(emitter) else {
            return Ok(());
        };
        self.sheet
            .draw_frame_scaled_faded(canvas, dest, base + frame, mirrored, alpha)
    }
}

/// A theme's cast, built on demand.
pub struct CharacterSet<'a> {
    data: CharacterSetData,
    texture_creator: &'a TextureCreator<WindowContext>,
    built: RefCell<HashMap<usize, CharacterSprites<'a>>>,
}

impl<'a> CharacterSet<'a> {
    pub fn new(data: CharacterSetData, texture_creator: &'a TextureCreator<WindowContext>) -> Self {
        Self {
            data,
            texture_creator,
            built: RefCell::new(HashMap::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.data.characters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.characters.is_empty()
    }

    pub fn meta(&self, character: usize) -> Option<CharacterMeta> {
        self.data
            .characters
            .get(character)
            .map(|c| c.meta(self.data.sweat.as_ref()))
    }

    /// One frame of an emitter's particle, in the character's own png, so a caller can size
    /// it - a spark is 8x16 where a sweat drop is 4x5.
    pub fn particle_size(&self, character: usize, emitter: usize) -> Option<(u32, u32)> {
        let data = self.data.characters.get(character)?;
        data.emitters
            .iter()
            .chain(self.data.sweat.as_ref())
            .nth(emitter)
            .map(|e| e.size)
    }

    /// One pose's own size, so a caller can draw it at that rather than filling the box.
    pub fn pose_size(&self, character: usize, pose: usize) -> Option<(u32, u32)> {
        let routines = self.data.characters.get(character)?.routines?;
        routines.poses.get(pose).map(|(_, _, w, h)| (*w, *h))
    }

    /// One frame of a layer, likewise.
    pub fn layer_size(&self, character: usize, layer: usize) -> Option<(u32, u32)> {
        self.data
            .characters
            .get(character)?
            .layers
            .get(layer)
            .map(|l| l.size)
    }

    pub fn name(&self, character: usize) -> Option<&'static str> {
        self.data.characters.get(character).map(|c| c.name)
    }

    /// Build one character's texture if it has not been built. Called when a character is
    /// dealt, so that the first frame of a match is not the one that pays for it.
    pub fn ensure_built(&self, character: usize) -> Result<(), String> {
        if character >= self.data.characters.len() || self.built.borrow().contains_key(&character) {
            return Ok(());
        }
        let data = &self.data.characters[character];
        let texture = self.texture_creator.load_texture_bytes(data.file)?;
        let (width, height) = self.data.frame_size;
        let mut frames = vec![];
        let mut base = [0usize; 4];
        if let Some(routines) = data.routines {
            // a routine names a pose outright, so every state indexes the one table and the
            // four bases are all zero
            for (x, y, w, h) in routines.poses {
                frames.push(Rect::new(*x, *y, *w, *h));
            }
        } else {
            for state in CharacterState::ALL {
                base[state.index()] = frames.len();
                let count = data.states[state.index()].0;
                let y = (state.index() as u32 * self.data.row_pitch) as i32;
                for frame in 0..count {
                    frames.push(Rect::new((frame as u32 * width) as i32, y, width, height));
                }
            }
        }
        let mut layers = vec![];
        for layer in data.layers {
            let mut bases = [0usize; 4];
            for state in CharacterState::ALL {
                bases[state.index()] = frames.len();
                let (count, _) = layer.states[state.index()];
                let y = layer.origin.1 + (state.index() as u32 * layer.row_pitch) as i32;
                for frame in 0..count {
                    frames.push(Rect::new(
                        layer.origin.0 + (frame as u32 * layer.size.0) as i32,
                        y,
                        layer.size.0,
                        layer.size.1,
                    ));
                }
            }
            layers.push(bases);
        }
        let mut emitters = vec![];
        for emitter in data.emitters.iter().chain(self.data.sweat.as_ref()) {
            emitters.push(frames.len());
            for frame in 0..emitter.frames {
                frames.push(Rect::new(
                    emitter.origin.0 + (frame as u32 * emitter.size.0) as i32,
                    emitter.origin.1,
                    emitter.size.0,
                    emitter.size.1,
                ));
            }
        }
        self.built.borrow_mut().insert(
            character,
            CharacterSprites {
                sheet: AnimationSpriteSheet::new(texture, frames),
                base,
                layers,
                emitters,
            },
        );
        Ok(())
    }

    /// Run `draw` against one character's built sprites, building them if they are new.
    pub fn with<T>(
        &self,
        character: usize,
        draw: impl FnOnce(&CharacterSprites<'a>) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        self.ensure_built(character)?;
        let built = self.built.borrow();
        match built.get(&character) {
            Some(sprites) => draw(sprites).map(Some),
            None => Ok(None),
        }
    }

    pub fn draw(
        &self,
        canvas: &mut WindowCanvas,
        dest: Rect,
        character: usize,
        state: CharacterState,
        frame: usize,
        mirrored: bool,
    ) -> Result<(), String> {
        self.ensure_built(character)?;
        let built = self.built.borrow();
        let Some(sprites) = built.get(&character) else {
            return Ok(());
        };
        sprites.draw(canvas, dest, state, frame, mirrored)
    }
}
