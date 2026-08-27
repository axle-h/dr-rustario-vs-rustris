use crate::game::block::{block_partner_offset, Block};
use crate::game::event::ColoredBlock;
use crate::game::geometry::BottlePoint;
use crate::game::pill::{Garbage, Pill, PillShape, VirusColor, Vitamin, Vitamins};
use crate::game::random::BottleSeed;
use rand::seq::SliceRandom;
#[cfg(test)]
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::ops::Range;

pub const BOTTLE_WIDTH: u32 = 8;
pub const BOTTLE_HEIGHT: u32 = 16;
pub const BOTTLE_FLOOR: u32 = BOTTLE_HEIGHT - 1;
pub const TOTAL_BLOCKS: u32 = BOTTLE_WIDTH * BOTTLE_HEIGHT;

fn index_at(x: u32, y: u32) -> usize {
    (y * BOTTLE_WIDTH + x) as usize
}

fn index_to_point(index: usize) -> BottlePoint {
    BottlePoint::new(
        (index % BOTTLE_WIDTH as usize) as i32,
        (index / BOTTLE_WIDTH as usize) as i32,
    )
}

fn index(point: BottlePoint) -> usize {
    index_at(point.x() as u32, point.y() as u32)
}

fn row_range(y: u32) -> Range<usize> {
    index_at(0, y)..index_at(0, y + 1)
}

pub type SendGarbage = Vec<VirusColor>;

struct PatternMatchContext {
    is_vertical: bool,
    result: HashSet<BottlePoint>,
    last_color: Option<VirusColor>,
    count: u32,
    patterns: Vec<VirusColor>,
}

impl PatternMatchContext {
    fn new(is_vertical: bool) -> Self {
        Self {
            is_vertical,
            count: 0,
            last_color: None,
            result: HashSet::new(),
            patterns: vec![],
        }
    }

    fn reset(&mut self, is_vertical: bool) {
        self.is_vertical = is_vertical;
        self.count = 0;
        self.last_color = None;
    }

    fn block(&mut self, x: u32, y: u32, block: Block) {
        match block.destructible_color() {
            Some(color) if self.last_color == Some(color) => {
                self.count += 1;
            }
            Some(color) => {
                self.maybe_pattern(x, y, -1);
                self.count = 1;
                self.last_color = Some(color);
            }
            None => {
                self.maybe_pattern(x, y, -1);
                self.count = 0;
                self.last_color = None;
            }
        }
    }

    fn maybe_pattern(&mut self, x: u32, y: u32, offset: i32) {
        if self.count > 3 {
            self.patterns.push(self.last_color.unwrap());
            let x = x as i32;
            let y = y as i32;
            for i in 0..self.count as i32 {
                if self.is_vertical {
                    self.result.insert(BottlePoint::new(x, y - i + offset));
                } else {
                    self.result.insert(BottlePoint::new(x - i + offset, y));
                }
            }
            self.count = 0;
            self.last_color = None;
        }
    }
}

#[derive(Clone)]
pub struct Bottle {
    blocks: [Block; TOTAL_BLOCKS as usize],
    pill: Option<Pill>,
    rng: ChaChaRng,
}

impl Bottle {
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            blocks: [Block::Empty; TOTAL_BLOCKS as usize],
            pill: None,
            rng: ChaChaRng::seed_from_u64(0),
        }
    }

    pub fn from_seed(seed: BottleSeed, rng: ChaChaRng) -> Self {
        Self {
            blocks: seed.into_blocks(),
            pill: None,
            rng,
        }
    }

    /// the pill in play, if one has spawned and not yet locked
    pub fn pill(&self) -> Option<&Pill> {
        self.pill.as_ref()
    }

    pub fn row(&self, y: u32) -> &[Block] {
        &self.blocks[row_range(y)]
    }

    pub fn block(&self, point: BottlePoint) -> Block {
        self.blocks[index(point)]
    }

    pub fn block_at(&self, x: u32, y: u32) -> Block {
        self.blocks[index_at(x, y)]
    }

    fn set_block(&mut self, point: BottlePoint, state: Block) {
        self.blocks[index(point)] = state;
    }

    /// place a settled block directly, for tests and the ai's own fixtures
    pub fn place(&mut self, x: u32, y: u32, state: Block) {
        self.set_block_at(x, y, state);
    }

    fn set_block_at(&mut self, x: u32, y: u32, state: Block) {
        self.blocks[index_at(x, y)] = state;
    }

    fn set_vitamin(&mut self, pill: &Pill, vitamin: &Vitamin) {
        self.set_block(
            vitamin.position(),
            Block::Vitamin(vitamin.color(), pill.rotation(), vitamin.ordinal()),
        )
    }

    fn set_garbage(&mut self, point: BottlePoint) {
        let index = index(point);
        self.blocks[index] = Block::Garbage(
            self.blocks[index]
                .destructible_color()
                .expect("not a destructible block"),
        );
    }

    pub fn virus_count(&self) -> u32 {
        self.blocks.iter().filter(|b| b.is_virus()).count() as u32
    }

    pub fn viruses(&self) -> Vec<ColoredBlock> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.is_virus())
            .map(|(index, b)| ColoredBlock::from_block(index_to_point(index), *b))
            .collect()
    }

    pub fn try_spawn(&mut self, shape: PillShape) -> Option<Vitamins> {
        if self.pill.is_some() {
            panic!("pill already spawned")
        }

        let pill = Pill::new(shape);
        let mut success = true;
        for vitamin in pill.vitamins() {
            if self.block(vitamin.position()).is_destructible() {
                success = false;
            } else {
                self.set_vitamin(&pill, &vitamin);
            }
        }

        // regardless of success we have set blocks for this pill
        self.pill = Some(pill);

        if success {
            self.render_ghost();
            Some(pill.vitamins())
        } else {
            None
        }
    }

    pub fn left(&mut self) -> bool {
        self.move_pill(-1)
    }

    pub fn right(&mut self) -> bool {
        self.move_pill(1)
    }

    fn move_pill(&mut self, dx: i32) -> bool {
        debug_assert!(dx == -1 || dx == 1);
        if self.pill.is_none() {
            return false;
        }

        let vitamins = self.pill.unwrap().vitamins();
        for vitamin in vitamins {
            if dx > 0 && vitamin.position().x() == BOTTLE_WIDTH as i32 - 1 {
                // collided with the right wall
                return false;
            } else if dx < 0 && vitamin.position().x() == 0 {
                // collided with the left wall
                return false;
            }
            let check_point = vitamin.position().translate(dx, 0);
            if self.block(check_point).is_destructible() {
                // collided with a virus
                return false;
            }
        }

        self.mutate_pill(|t| t.translate(dx, 0));
        true
    }

    pub fn rotate(&mut self, clockwise: bool) -> bool {
        if let Some((dx, dy)) = self.try_rotate(clockwise) {
            self.mutate_pill(|pill| pill.rotate(clockwise, dx, dy));
            true
        } else {
            false
        }
    }

    /// Steps down the current pill
    /// Returns true if successful
    pub fn step_down_pill(&mut self) -> bool {
        let vitamins = self.pill.expect("no pill").vitamins();
        if self.vitamins_collide(vitamins) {
            false
        } else {
            self.mutate_pill(|t| t.translate(0, 1));
            true
        }
    }

    pub fn hard_drop(&mut self) -> Option<(u32, Vitamins)> {
        let (dropped, _) = self.dropped_vitamins()?;
        let vitamins = self.pill?.vitamins();
        if dropped > 0 {
            self.mutate_pill(|t| t.translate(0, dropped as i32));
        }
        Some((dropped, vitamins))
    }

    pub fn lock(&mut self) -> Option<Vitamins> {
        let pill = self.pill?;
        let vitamins = pill.vitamins();
        for vitamin in vitamins {
            self.set_block(
                vitamin.position(),
                Block::Stack(vitamin.color(), pill.rotation(), vitamin.ordinal()),
            );
        }
        self.pill = None;
        Some(vitamins)
    }

    pub fn pattern(&self) -> (Vec<ColoredBlock>, Vec<VirusColor>) {
        let mut context = PatternMatchContext::new(false);

        // by rows
        for y in 0..BOTTLE_HEIGHT {
            context.reset(false);
            for x in 0..BOTTLE_WIDTH {
                context.block(x, y, self.block_at(x, y));
            }
            context.maybe_pattern(BOTTLE_WIDTH - 1, y, 0);
        }

        // by cols
        for x in 0..BOTTLE_WIDTH {
            context.reset(true);
            for y in 0..BOTTLE_HEIGHT {
                context.block(x, y, self.block_at(x, y));
            }
            context.maybe_pattern(x, BOTTLE_FLOOR, 0);
        }

        let blocks = context
            .result
            .into_iter()
            .map(|p| ColoredBlock::from_block(p, self.block(p)))
            .collect();

        (blocks, context.patterns)
    }

    pub fn destroy(&mut self, blocks: Vec<ColoredBlock>) {
        for block in blocks {
            let point = block.position;
            let block = self.block(point);
            debug_assert!(block.is_destructible());

            // 1. destroy the block
            self.set_block(point, Block::Empty);

            // 2. check if this created any garbage
            if let Some(partner_offset) = block.find_stack_partner_offset() {
                self.set_garbage(point + partner_offset);
            }
        }
    }

    pub fn step_down_garbage(&mut self) -> bool {
        let mut to_fall = HashSet::new();
        for x in 0..BOTTLE_WIDTH {
            for y in 0..BOTTLE_FLOOR {
                let point = BottlePoint::new(x as i32, y as i32);
                if to_fall.contains(&point) {
                    continue;
                }
                if let Some(group) = self.get_falling_group(point) {
                    for falling_block in group {
                        to_fall.insert(falling_block);
                    }
                }
            }
        }

        if to_fall.is_empty() {
            return false;
        }

        for x in 0..BOTTLE_WIDTH {
            for y in (0..BOTTLE_FLOOR).rev() {
                let point = BottlePoint::new(x as i32, y as i32);
                if to_fall.contains(&point) {
                    let block = self.block(point);
                    self.set_block(point.translate(0, 1), block);
                    self.set_block(point, Block::Empty);
                }
            }
        }

        true
    }

    fn get_falling_group(&self, point: BottlePoint) -> Option<HashSet<BottlePoint>> {
        let mut result = HashSet::new();
        let mut queue = vec![point];
        while let Some(point) = queue.pop() {
            if result.contains(&point) {
                continue;
            }
            let is_floor = point.y() == BOTTLE_FLOOR as i32;
            let block = self.block(point);
            match block {
                Block::Empty => {
                    continue;
                }
                Block::Stack(_, rotation, ordinal) if !is_floor => {
                    queue.push(point.translate(0, 1));
                    queue.push(point + block_partner_offset(rotation, ordinal));
                    result.insert(point);
                }
                Block::Garbage(_) if !is_floor => {
                    result.insert(point);
                    queue.push(point.translate(0, 1));
                }
                _ => return None,
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    pub fn hold(&mut self) -> Option<PillShape> {
        let pill = self.pill?;

        // remove from board
        for vitamin in pill.vitamins() {
            self.set_block(vitamin.position(), Block::Empty);
        }
        self.pill = None;
        self.render_ghost();

        Some(pill.shape())
    }

    pub fn send_garbage(&mut self, garbage: SendGarbage) -> Vec<Garbage> {
        let mut available_x = self
            .row(0)
            .into_iter()
            .enumerate()
            .filter(|(_, b)| b.is_empty())
            .map(|(x, _)| x as u32)
            .collect::<Vec<u32>>();
        available_x.shuffle(&mut self.rng);

        let mut sent = vec![];
        for color in garbage.into_iter() {
            if let Some(x) = available_x.pop() {
                self.set_block_at(x, 0, Block::Garbage(color));
                sent.push(Garbage::new(color, BottlePoint::new(x as i32, 0)));
            } else {
                break;
            }
        }
        sent
    }

    pub fn register_lock_placement(&mut self) -> u32 {
        self.pill
            .as_mut()
            .expect("no pill to set lock placements")
            .register_lock_placement()
    }

    pub fn lock_placements(&self) -> u32 {
        self.pill
            .expect("no pill to get lock placements")
            .lock_placements()
    }

    pub fn is_collision(&self) -> bool {
        self.vitamins_collide(self.pill.expect("no pill to test for collision").vitamins())
    }

    fn try_rotate(&self, clockwise: bool) -> Option<(i32, i32)> {
        let pill = &self.pill?;

        let next_points = pill.next_rotation(clockwise);
        for (dx, dy) in pill.available_wall_kicks() {
            let mut success = true;
            for p in next_points.iter() {
                let p = p.translate(*dx, *dy);
                if p.x() < 0
                    || p.x() >= BOTTLE_WIDTH as i32
                    || p.y() < 0
                    || p.y() >= BOTTLE_HEIGHT as i32
                {
                    success = false;
                    break;
                }
                if self.block(p).is_destructible() {
                    success = false;
                    break;
                }
            }
            if success {
                return Some((*dx, *dy));
            }
        }

        None
    }

    fn mutate_pill<F: FnMut(&mut Pill)>(&mut self, mut f: F) {
        // remove from board
        let mut pill = self.pill.unwrap();
        for vitamin in pill.vitamins() {
            self.set_block(vitamin.position(), Block::Empty);
        }

        // update pill
        f(&mut pill);

        // add back to board
        for vitamin in pill.vitamins() {
            self.set_vitamin(&pill, &vitamin);
        }

        self.pill = Some(pill);
        self.render_ghost();
    }

    /// gets a copy of the current vitamins having dropped until a collision
    fn dropped_vitamins(&self) -> Option<(u32, Vitamins)> {
        let mut vitamins = self.pill?.vitamins();
        let mut dropped_rows = 0;
        while !self.vitamins_collide(vitamins) {
            dropped_rows += 1;
            for vitamin in vitamins.iter_mut() {
                vitamin.translate(0, 1);
            }
        }
        Some((dropped_rows, vitamins))
    }

    fn render_ghost(&mut self) {
        // remove all existing ghost blocks
        for i in 0..(TOTAL_BLOCKS as usize) {
            if matches!(self.blocks[i], Block::Ghost(_, _, _)) {
                self.blocks[i] = Block::Empty;
            }
        }

        if self.pill.is_none() {
            // no pill, no ghost.
            return;
        }

        let rotation = self.pill.unwrap().rotation();
        let (_, vitamins) = self.dropped_vitamins().unwrap();
        for vitamin in vitamins {
            if self.block(vitamin.position()) == Block::Empty {
                self.set_block(
                    vitamin.position(),
                    Block::Ghost(vitamin.color(), rotation, vitamin.ordinal()),
                )
            }
        }
    }

    fn vitamins_collide(&self, vitamins: Vitamins) -> bool {
        for vitamin in vitamins {
            if vitamin.position().y() == BOTTLE_FLOOR as i32 {
                // collided with the floor
                return true;
            }
            let block_down = self.block(vitamin.position().translate(0, 1));
            if block_down.is_destructible() {
                // collided with a virus or another vitamin
                return true;
            }
        }

        false
    }
}

impl Debug for Bottle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "   {}", "-".repeat(BOTTLE_WIDTH as usize))?;

        for y in 0..BOTTLE_HEIGHT {
            write!(f, "{:02}|", y)?;

            for x in 0..BOTTLE_WIDTH {
                match self.block_at(x, y) {
                    Block::Vitamin(color, _, _)
                    | Block::Stack(color, _, _)
                    | Block::Garbage(color) => write!(f, "{}", color.to_char())?,
                    Block::Virus(color) => write!(f, "{}", color.to_char().to_ascii_uppercase())?,
                    Block::Ghost(_, _, _) => write!(f, "G")?,
                    _ => write!(f, " ")?,
                }
            }

            writeln!(f, "|")?;
        }
        write!(f, "   {}", "-".repeat(BOTTLE_WIDTH as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::geometry::Rotation;
    use crate::game::pill::VitaminOrdinal;
    use std::collections::hash_map::RandomState;

    #[test]
    fn spawns_pill() {
        let mut bottle = Bottle::new();
        let vitamins = bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(vitamins.is_some());
        bottle.has_vitamin_at(3, 0, VirusColor::Red, Rotation::North, VitaminOrdinal::Left);
        bottle.has_vitamin_at(
            4,
            0,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn renders_ghost_to_floor() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.has_ghost_at(
            3,
            15,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_ghost_at(
            4,
            15,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn renders_ghost_to_virus() {
        let mut bottle = Bottle::new();
        bottle.having_virus(3, 15, VirusColor::Yellow);
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.has_ghost_at(
            3,
            14,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_ghost_at(
            4,
            14,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn ignores_move_left_when_no_pill() {
        let mut bottle = Bottle::new();
        assert!(!bottle.left());
    }

    #[test]
    fn ignores_move_right_when_no_pill() {
        let mut bottle = Bottle::new();
        assert!(!bottle.right());
    }

    #[test]
    fn ignores_rotate_when_no_pill() {
        let mut bottle = Bottle::new();
        assert!(!bottle.rotate(true));
    }

    #[test]
    fn can_move_left() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(bottle.left(), "{:?}", bottle);
        bottle.has_ghost_at(
            2,
            15,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_ghost_at(
            3,
            15,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn cannot_move_left_through_virus() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.having_virus(2, 0, VirusColor::Yellow);
        assert!(!bottle.left(), "{:?}", bottle);
    }

    #[test]
    fn cannot_move_left_through_wall() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        for _ in 0..3 {
            assert!(bottle.left(), "{:?}", bottle);
        }
        assert!(!bottle.left(), "{:?}", bottle);
    }

    #[test]
    fn can_move_right() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(bottle.right(), "{:?}", bottle);
        bottle.has_ghost_at(
            4,
            15,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_ghost_at(
            5,
            15,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn cannot_move_right_through_virus() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.having_virus(5, 0, VirusColor::Yellow);
        assert!(!bottle.right(), "{:?}", bottle);
    }

    #[test]
    fn cannot_move_right_through_wall() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        for _ in 0..3 {
            assert!(bottle.right(), "{:?}", bottle);
        }
        assert!(!bottle.right(), "{:?}", bottle);
    }

    #[test]
    fn can_rotate_clockwise() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(bottle.rotate(true));
        bottle.has_vitamin_at(3, 0, VirusColor::Red, Rotation::East, VitaminOrdinal::Left);
        bottle.has_vitamin_at(
            3,
            1,
            VirusColor::Blue,
            Rotation::East,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn can_rotate_anticlockwise() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(bottle.rotate(false));
        bottle.has_vitamin_at(3, 1, VirusColor::Red, Rotation::West, VitaminOrdinal::Left);
        bottle.has_vitamin_at(
            3,
            0,
            VirusColor::Blue,
            Rotation::West,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn clockwise_rotation_kicks_off_right_wall() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(bottle.rotate(true));
        for _ in 0..4 {
            assert!(bottle.right(), "{:?}", bottle);
        }
        assert!(bottle.rotate(true));
        bottle.has_vitamin_at(7, 1, VirusColor::Red, Rotation::South, VitaminOrdinal::Left);
        bottle.has_vitamin_at(
            6,
            1,
            VirusColor::Blue,
            Rotation::South,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn steps_down_to_floor() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        for _ in 1..BOTTLE_HEIGHT {
            assert!(bottle.step_down_pill());
        }
        assert!(!bottle.step_down_pill());
        bottle.has_vitamin_at(
            3,
            15,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_vitamin_at(
            4,
            15,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn steps_down_to_virus() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.having_virus(4, 15, VirusColor::Yellow);
        for _ in 2..BOTTLE_HEIGHT {
            assert!(bottle.step_down_pill());
        }
        assert!(!bottle.step_down_pill());
        bottle.has_vitamin_at(
            3,
            14,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_vitamin_at(
            4,
            14,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn hard_drops_to_floor() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        assert!(bottle.hard_drop().is_some());
        bottle.has_vitamin_at(
            3,
            15,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_vitamin_at(
            4,
            15,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn hard_drops_to_virus() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.having_virus(4, 15, VirusColor::Yellow);
        assert!(bottle.hard_drop().is_some());
        bottle.has_vitamin_at(
            3,
            14,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_vitamin_at(
            4,
            14,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn locks() {
        let mut bottle = Bottle::new();
        bottle.try_spawn(PillShape::new(VirusColor::Red, VirusColor::Blue));
        bottle.hard_drop();
        assert!(bottle.lock().is_some());
        bottle.has_stack_at(
            3,
            15,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_stack_at(
            4,
            15,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn empty_patterns() {
        let bottle = Bottle::new();
        let (pattern, patterns) = bottle.pattern();
        assert_eq!(pattern, vec![]);
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn no_patterns() {
        let mut bottle = Bottle::new();
        bottle.having_virus(1, 10, VirusColor::Yellow);
        bottle.having_virus(2, 10, VirusColor::Yellow);
        bottle.having_virus(3, 10, VirusColor::Yellow);

        bottle.having_stack(
            2,
            11,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.having_stack(
            3,
            11,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        bottle.having_garbage(4, 11, VirusColor::Blue);

        bottle.having_garbage(3, 12, VirusColor::Red);
        bottle.having_garbage(4, 12, VirusColor::Red);
        bottle.having_garbage(5, 12, VirusColor::Red);

        let (pattern, patterns) = bottle.pattern();
        assert_eq!(pattern, vec![]);
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn pattern_horizontal_4() {
        let mut bottle = Bottle::new();
        bottle.having_virus(1, 10, VirusColor::Yellow);
        bottle.having_virus(2, 10, VirusColor::Yellow);
        bottle.having_stack(
            3,
            10,
            VirusColor::Yellow,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.having_stack(
            4,
            10,
            VirusColor::Yellow,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        bottle.having_garbage(5, 10, VirusColor::Red);
        bottle.having_garbage(1, 11, VirusColor::Yellow);
        let (pattern, patterns) = bottle.pattern();
        let observed: HashSet<ColoredBlock, RandomState> = HashSet::from_iter(pattern.into_iter());
        assert_eq!(
            observed,
            HashSet::from_iter([
                ColoredBlock::from_block(BottlePoint::new(1, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(BottlePoint::new(2, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(
                    BottlePoint::new(3, 10),
                    Block::Stack(VirusColor::Yellow, Rotation::North, VitaminOrdinal::Left)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(4, 10),
                    Block::Stack(VirusColor::Yellow, Rotation::North, VitaminOrdinal::Right)
                )
            ])
        );
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn pattern_vertical_4() {
        let mut bottle = Bottle::new();
        bottle.having_virus(5, 10, VirusColor::Yellow);
        bottle.having_virus(5, 11, VirusColor::Yellow);
        bottle.having_garbage(5, 12, VirusColor::Yellow);
        bottle.having_garbage(5, 13, VirusColor::Yellow);
        bottle.having_garbage(5, 14, VirusColor::Red);
        bottle.having_garbage(4, 10, VirusColor::Yellow);
        let (pattern, patterns) = bottle.pattern();
        let observed: HashSet<ColoredBlock, RandomState> = HashSet::from_iter(pattern.into_iter());
        assert_eq!(
            observed,
            HashSet::from_iter([
                ColoredBlock::from_block(BottlePoint::new(5, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(BottlePoint::new(5, 11), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(
                    BottlePoint::new(5, 12),
                    Block::Garbage(VirusColor::Yellow)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(5, 13),
                    Block::Garbage(VirusColor::Yellow)
                )
            ])
        );
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn pattern_5() {
        let mut bottle = Bottle::new();
        bottle.having_virus(1, 10, VirusColor::Yellow);
        bottle.having_virus(2, 10, VirusColor::Yellow);
        bottle.having_garbage(3, 10, VirusColor::Yellow);
        bottle.having_garbage(4, 10, VirusColor::Yellow);
        bottle.having_garbage(5, 10, VirusColor::Yellow);
        let (pattern, patterns) = bottle.pattern();
        let observed: HashSet<ColoredBlock, RandomState> = HashSet::from_iter(pattern.into_iter());
        assert_eq!(
            observed,
            HashSet::from_iter([
                ColoredBlock::from_block(BottlePoint::new(1, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(BottlePoint::new(2, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(
                    BottlePoint::new(3, 10),
                    Block::Garbage(VirusColor::Yellow)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(4, 10),
                    Block::Garbage(VirusColor::Yellow)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(5, 10),
                    Block::Garbage(VirusColor::Yellow)
                )
            ])
        );
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn multiple_patterns() {
        let mut bottle = Bottle::new();
        bottle.having_virus(1, 10, VirusColor::Yellow);
        bottle.having_virus(2, 10, VirusColor::Yellow);
        bottle.having_stack(
            3,
            10,
            VirusColor::Yellow,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.having_stack(
            4,
            10,
            VirusColor::Yellow,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        bottle.having_garbage(4, 11, VirusColor::Yellow);
        bottle.having_garbage(4, 12, VirusColor::Yellow);
        bottle.having_garbage(4, 13, VirusColor::Yellow);

        let (pattern, patterns) = bottle.pattern();
        let observed: HashSet<ColoredBlock, RandomState> = HashSet::from_iter(pattern.into_iter());
        assert_eq!(
            observed,
            HashSet::from_iter([
                ColoredBlock::from_block(BottlePoint::new(1, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(BottlePoint::new(2, 10), Block::Virus(VirusColor::Yellow)),
                ColoredBlock::from_block(
                    BottlePoint::new(3, 10),
                    Block::Stack(VirusColor::Yellow, Rotation::North, VitaminOrdinal::Left)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(4, 10),
                    Block::Stack(VirusColor::Yellow, Rotation::North, VitaminOrdinal::Right)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(4, 11),
                    Block::Garbage(VirusColor::Yellow)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(4, 12),
                    Block::Garbage(VirusColor::Yellow)
                ),
                ColoredBlock::from_block(
                    BottlePoint::new(4, 13),
                    Block::Garbage(VirusColor::Yellow)
                ),
            ])
        );
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn pattern_with_repeating_block_on_bottom_of_bottle() {
        let mut bottle = Bottle::new();
        bottle.having_garbage(7, 12, VirusColor::Red);
        bottle.having_garbage(7, 13, VirusColor::Red);
        bottle.having_garbage(7, 14, VirusColor::Red);
        bottle.having_virus(7, 15, VirusColor::Red);
        bottle.having_garbage(6, 15, VirusColor::Red);
        bottle.having_garbage(5, 15, VirusColor::Red);
        bottle.having_garbage(4, 15, VirusColor::Red);
        let (pattern, patterns) = bottle.pattern();
        assert_eq!(pattern.len(), 7, "{:?}", pattern);
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn destroy() {
        let mut bottle = Bottle::new();
        bottle.having_virus(0, 10, VirusColor::Yellow);
        bottle.having_virus(1, 10, VirusColor::Yellow);
        bottle.having_garbage(2, 10, VirusColor::Yellow);
        bottle.having_garbage(3, 10, VirusColor::Yellow);
        bottle.having_virus(0, 11, VirusColor::Yellow);
        let (pattern, _) = bottle.pattern();
        bottle.destroy(pattern);
        bottle.is_empty_at(0, 10);
        bottle.is_empty_at(1, 10);
        bottle.is_empty_at(2, 10);
        bottle.is_empty_at(3, 10);
        bottle.has_virus_at(0, 11, VirusColor::Yellow);
    }

    #[test]
    fn destroy_generates_garbage() {
        let mut bottle = Bottle::new();
        bottle.having_stack(
            5,
            BOTTLE_FLOOR,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.having_stack(
            6,
            BOTTLE_FLOOR,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        bottle.destroy(vec![ColoredBlock::from_block(
            BottlePoint::new(5, BOTTLE_FLOOR as i32),
            Block::Stack(VirusColor::Red, Rotation::North, VitaminOrdinal::Left),
        )]);
        bottle.is_empty_at(5, BOTTLE_FLOOR);
        bottle.is_garbage_at(6, BOTTLE_FLOOR, VirusColor::Blue);
    }

    #[test]
    fn step_down_no_garbage() {
        let mut bottle = Bottle::new();
        bottle.having_virus(0, 10, VirusColor::Yellow);
        bottle.having_garbage(0, 9, VirusColor::Yellow);
        assert!(!bottle.step_down_garbage());
    }

    #[test]
    fn step_down_free_garbage() {
        let mut bottle = Bottle::new();
        bottle.having_virus(0, 10, VirusColor::Yellow);
        bottle.having_garbage(0, 8, VirusColor::Yellow);
        assert!(bottle.step_down_garbage());
        bottle.has_garbage_at(0, 9, VirusColor::Yellow);
    }

    #[test]
    fn step_down_free_stacked_garbage() {
        let mut bottle = Bottle::new();
        bottle.having_virus(0, 10, VirusColor::Yellow);
        bottle.having_garbage(0, 8, VirusColor::Yellow);
        bottle.having_garbage(0, 7, VirusColor::Yellow);
        assert!(bottle.step_down_garbage());
        bottle.has_garbage_at(0, 9, VirusColor::Yellow);
        bottle.has_garbage_at(0, 8, VirusColor::Yellow);
    }

    #[test]
    fn step_down_kitchen_sink() {
        let mut bottle = Bottle::new();
        bottle.having_garbage(0, 14, VirusColor::Yellow);
        bottle.having_stack(
            0,
            13,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.having_stack(
            1,
            13,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        bottle.having_stack(
            1,
            12,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.having_stack(
            2,
            12,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        assert!(bottle.step_down_garbage());
        bottle.has_garbage_at(0, 15, VirusColor::Yellow);
        bottle.has_stack_at(
            0,
            14,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_stack_at(
            1,
            14,
            VirusColor::Red,
            Rotation::North,
            VitaminOrdinal::Right,
        );
        bottle.has_stack_at(
            1,
            13,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Left,
        );
        bottle.has_stack_at(
            2,
            13,
            VirusColor::Blue,
            Rotation::North,
            VitaminOrdinal::Right,
        );
    }

    #[test]
    fn hold() {
        let mut bottle = Bottle::new();
        let shape = PillShape::new(VirusColor::Red, VirusColor::Blue);
        bottle.try_spawn(shape);
        assert_eq!(bottle.hold(), Some(shape));
        bottle.is_empty();
    }

    #[test]
    fn send_garbage() {
        let mut bottle = Bottle::new();
        assert_eq!(
            bottle
                .send_garbage(vec![VirusColor::Blue, VirusColor::Red])
                .len(),
            2
        );
        let mut garbage = HashSet::new();
        for block in bottle.row(0).into_iter() {
            if let Block::Garbage(color) = block {
                garbage.insert(*color);
            }
        }
        assert_eq!(
            garbage,
            HashSet::from_iter([VirusColor::Blue, VirusColor::Red])
        )
    }

    trait BottleTestHarness {
        fn is_empty(&self);
        fn is_empty_at(&self, x: u32, y: u32);
        fn is_garbage_at(&self, x: u32, y: u32, color: VirusColor);
        fn has_vitamin_at(
            &self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        );
        fn has_stack_at(
            &self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        );
        fn has_garbage_at(&self, x: u32, y: u32, color: VirusColor);
        fn has_ghost_at(
            &self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        );
        fn has_virus_at(&self, x: u32, y: u32, color: VirusColor);
        fn having_virus(&mut self, x: u32, y: u32, color: VirusColor);
        fn having_stack(
            &mut self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        );
        fn having_garbage(&mut self, x: u32, y: u32, color: VirusColor);
    }

    impl BottleTestHarness for Bottle {
        fn is_empty(&self) {
            assert_eq!(
                self.blocks,
                [Block::Empty; TOTAL_BLOCKS as usize],
                "{:?}",
                self
            );
        }

        fn is_empty_at(&self, x: u32, y: u32) {
            assert_eq!(self.blocks[index_at(x, y)], Block::Empty, "{:?}", self);
        }

        fn is_garbage_at(&self, x: u32, y: u32, color: VirusColor) {
            assert_eq!(
                self.blocks[index_at(x, y)],
                Block::Garbage(color),
                "{:?}",
                self
            );
        }

        fn has_vitamin_at(
            &self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        ) {
            assert_eq!(
                self.blocks[index_at(x, y)],
                Block::Vitamin(color, rotation, ordinal),
                "{:?}",
                self
            );
        }

        fn has_stack_at(
            &self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        ) {
            assert_eq!(
                self.blocks[index_at(x, y)],
                Block::Stack(color, rotation, ordinal),
                "{:?}",
                self
            );
        }

        fn has_garbage_at(&self, x: u32, y: u32, color: VirusColor) {
            assert_eq!(
                self.blocks[index_at(x, y)],
                Block::Garbage(color),
                "{:?}",
                self
            );
        }

        fn has_ghost_at(
            &self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        ) {
            assert_eq!(
                self.blocks[index_at(x, y)],
                Block::Ghost(color, rotation, ordinal),
                "{:?}",
                self
            );
        }

        fn has_virus_at(&self, x: u32, y: u32, color: VirusColor) {
            assert_eq!(
                self.blocks[index_at(x, y)],
                Block::Virus(color),
                "{:?}",
                self
            );
        }

        fn having_virus(&mut self, x: u32, y: u32, color: VirusColor) {
            self.blocks[index_at(x, y)] = Block::Virus(color);
        }

        fn having_stack(
            &mut self,
            x: u32,
            y: u32,
            color: VirusColor,
            rotation: Rotation,
            ordinal: VitaminOrdinal,
        ) {
            self.blocks[index_at(x, y)] = Block::Stack(color, rotation, ordinal);
        }

        fn having_garbage(&mut self, x: u32, y: u32, color: VirusColor) {
            self.blocks[index_at(x, y)] = Block::Garbage(color);
        }
    }
}
