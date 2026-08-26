//! Dr. Mario 64's own opponent, reimplemented.
//!
//! This is a port of `aiset.c` from the Nintendo 64 game's decompilation: a hand written,
//! deterministic scorer with no learning in it anywhere. For every place the pill in play can
//! come to rest it drops the two halves into a copy of the bottle, measures the runs of colour
//! they land in, takes away whatever clears, measures what is left, asks whether the bottle it
//! leaves behind would chain, and adds the answers up with a table of weights. The highest
//! total wins, and a tie goes to the first candidate in the original's own order.
//!
//! Three things decide the weights. The *skill* row ([`params::DEFAULT_SKILL`]) is fixed - a
//! Dr. Rustario difficulty only decides how fast the agent may press keys - and the *situation*
//! ([`Situation`]) and *wall* are read off the bottle at the start of every pill, which is what
//! makes the ai play differently with a bottle full of viruses than with two left in the
//! corner. Everything the original does that is not about choosing a square is left out: the
//! sixteen characters and their moods, the deliberate mistakes, and the frame level key pacing,
//! which [`engine::ai::KeyPacer`] already does.
//!
//! Where the two games disagree the bottle wins: the candidates come from
//! [`crate::game::ai::placement`], which walks real [`Bottle`] moves and so honours Dr.
//! Rustario's own wall kicks, and only the scoring is the N64's.

mod chain;
mod field;
mod params;
mod routes;
mod score;

use crate::game::ai::placement::Placement;
use crate::game::bottle::Bottle;
use crate::game::pill::VirusColor;
use chain::rensa_check_core;
use field::{
    colour, Cell, Field, COLS, ROWS, ST_HORIZONTAL_LEFT, ST_HORIZONTAL_RIGHT, ST_VERTICAL_BOTTOM,
    ST_VERTICAL_TOP,
};
use params::Params;
pub use params::{DEFAULT_SKILL, SKILLS};
use score::{search_line_ms, Flag};

/// What the bottle is asking for, which picks a column of the weight table.
/// Original name: the `var_s5` that indexes `ai_param`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// the original's practice mode, which this port never selects
    #[allow(dead_code)]
    Training = 0,
    /// almost nowhere left to put a pill
    Cornered = 1,
    /// the end is in sight and there are still viruses within reach
    Finishing = 2,
    /// the end is in sight and the viruses left are buried
    Digging = 3,
    /// the end is in sight and the bottle is low and clean
    Idle = 4,
    /// a column of one colour is building in the middle, where the pills come in
    CentreStack = 5,
    /// a column of one colour is building off to the side
    SideStack = 6,
    /// a bottle full of viruses and room to work
    Normal = 7,
}

/// Half a pill, where it came to rest.
#[derive(Clone, Copy, Debug)]
pub struct Half {
    pub row: usize,
    pub col: usize,
    /// which way its partner lies, or that it is on its own
    pub st: u8,
    pub co: u8,
}

impl Half {
    fn cell(&self) -> Cell {
        Cell::new(self.st, self.co)
    }

    fn recoloured(&self, co: u8) -> Self {
        Self { co, ..*self }
    }
}

/// One place the pill can come to rest, in the ai's own terms.
/// Original name: `struct_aiFlag`, before it is scored.
pub struct Candidate {
    /// which placement in the list handed to [`N64Ai::choose`] this is
    placement: usize,
    /// 0 upright, 1 flat. Original name: `tory`
    tory: u8,
    /// the lower half of an upright pill, or the left half of a flat one
    row: usize,
    col: usize,
    /// whether the colours are the other way round from the pill as it comes in
    rev: u8,
    /// the half with something under it, which is the one gravity leaves alone
    main: Half,
    second: Half,
    /// both halves are the same colour, so one line does for both
    ec: bool,
}

/// Dr. Mario 64's opponent: hand the pill's landing places to [`Self::choose`] and it names one.
#[derive(Clone, Copy, Debug)]
pub struct N64Ai {
    skill: u8,
}

impl Default for N64Ai {
    fn default() -> Self {
        Self::new()
    }
}

impl N64Ai {
    pub fn new() -> Self {
        Self {
            skill: DEFAULT_SKILL,
        }
    }

    /// one of the original's six rows of weights, 0 the meekest and 5 the most aggressive
    pub fn with_skill(skill: u8) -> Self {
        Self {
            skill: skill.min(SKILLS as u8 - 1),
        }
    }

    /// Which of `placements` to play. `bottle` is the bottle those placements were found in,
    /// with the pill still in play; it is read for the stack alone.
    pub fn choose(&self, bottle: &Bottle, placements: &[Placement]) -> Option<usize> {
        if placements.is_empty() {
            return None;
        }

        let field = Field::of(bottle);
        let candidates = candidates(&field, placements);
        if candidates.is_empty() {
            return None;
        }

        let (situation, wall) =
            classify(&field, bottle.virus_count(), routes::average_route(&field));
        let params = Params::of(self.skill, situation);
        let wall = if params.wall { wall } else { 0 };

        let mut best = i32::MIN;
        let mut chosen = None;
        for candidate in &candidates {
            let priority = self.priority(&field, &candidates, candidate, &params, wall);
            // a tie goes to whichever came first in the original's ordering
            if chosen.is_none() || priority > best {
                best = priority;
                chosen = Some(candidate.placement);
            }
        }
        chosen
    }

    /// What one candidate is worth. Original name: the body of `aiHiruAllPriSet`'s loop.
    fn priority(
        &self,
        original: &Field,
        candidates: &[Candidate],
        candidate: &Candidate,
        params: &Params,
        wall: usize,
    ) -> i32 {
        let mut field = *original;
        let (main, second) = (candidate.main, candidate.second);
        if main.row != 0 {
            field.set(main.row, main.col, main.cell());
        }
        if second.row != 0 {
            field.set(second.row, second.col, second.cell());
        }

        let mut flag = Flag::new(candidate.tory);
        let made = search_line_ms(
            &mut flag,
            &mut field,
            original,
            candidates,
            params,
            (main.col, main.row, main.co),
            (second.col, second.row, second.co),
            candidate.ec,
            wall,
        );

        if made == 0 || params.rensa_p == 0 {
            return flag.pri;
        }

        // whichever half made the line is the one the chain has to be built around
        let (main, second) = if made == 2 {
            (second, main)
        } else {
            (main, second)
        };

        let mut chains = [0u8; VirusColor::N];
        let mut relieved = [0i32; VirusColor::N];
        for (co, (strength, relief)) in chains.iter_mut().zip(relieved.iter_mut()).enumerate() {
            let chain = rensa_check_core(original, main, second.recoloured(co as u8));
            *strength = chain.strength;
            *relief = chain.relieved;
        }

        // and the same again with the other half turned about the one that made the line, since
        // a chain that only wants the pill the other way round is still a chain worth having
        let mut elsewhere = 0;
        for alternative in alternatives(main, second) {
            if !original.at(alternative.row, alternative.col).is_empty() {
                continue;
            }
            for co in 0..VirusColor::N as u8 {
                elsewhere |= rensa_check_core(original, main, alternative.recoloured(co)).strength;
            }
        }

        let played = second.co as usize;
        flag.pri += relieved[played] * params.pri_point[7];
        if chains[played] != 0 {
            flag.pri += params.rensa_p * chains[played] as i32;
        } else if elsewhere != 0
            || chains
                .iter()
                .enumerate()
                .any(|(co, c)| co != played && *c != 0)
        {
            // the chain is there, but only for a colour that is not in this pill
            if main.row >= 3 {
                flag.pri += params.rensa_mp;
            }
        }

        flag.pri
    }
}

/// Where else the second half could have gone, turned about the half that made the line: flat
/// either side of it, or upright above it. Original name: the tail of `aiHiruAllPriSet`.
fn alternatives(main: Half, second: Half) -> Vec<Half> {
    let mut places = vec![];
    let mut push = |row: i32, col: i32, st: u8| {
        if row > 0 && (row as usize) < ROWS && col >= 0 && (col as usize) < COLS {
            places.push(Half {
                row: row as usize,
                col: col as usize,
                st,
                co: second.co,
            });
        }
    };
    let (row, col) = (main.row as i32, main.col as i32);

    if main.col == second.col {
        // upright: lay it flat, to the left of the main half and then to the right
        push(row, col - 1, ST_HORIZONTAL_LEFT);
        push(row, col + 1, ST_HORIZONTAL_RIGHT);
    } else {
        // flat: put it on the other side, then stand the pill up
        if second.col < main.col {
            push(row, col + 1, ST_HORIZONTAL_RIGHT);
        } else {
            push(row, col - 1, ST_HORIZONTAL_LEFT);
        }
        push(row - 1, col, ST_VERTICAL_TOP);
    }
    places
}

/// Turn the placements the bottle found into the ai's own candidates, in the order
/// `aifPlaceSearch` produces them: every upright place, then the same again with the colours
/// swapped, then every flat one and its swap, each by row and then by column. That order is
/// what settles a tie.
fn candidates(field: &Field, placements: &[Placement]) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = placements
        .iter()
        .enumerate()
        .filter_map(|(index, placement)| candidate(field, index, placement))
        .collect();
    candidates.sort_by_key(|c| (c.tory, c.rev, c.row, c.col));
    candidates
}

fn candidate(field: &Field, placement: usize, landed: &Placement) -> Option<Candidate> {
    let [(left, left_colour), (right, right_colour)] = landed.landing();
    // the ai numbers rows from the phantom row above the bottle
    let (lx, ly) = (left.x() as usize, left.y() as usize + 1);
    let (rx, ry) = (right.x() as usize, right.y() as usize + 1);
    let (lco, rco) = (colour(left_colour), colour(right_colour));
    if ly >= ROWS || ry >= ROWS || lx >= COLS || rx >= COLS {
        return None;
    }

    let (tory, row, col, main, second, rev) = if lx == rx {
        // upright, and the ai works from the lower half
        let (top, bottom, top_co, bottom_co, rev) = if ly < ry {
            (ly, ry, lco, rco, 0)
        } else {
            (ry, ly, rco, lco, 1)
        };
        (
            0,
            bottom,
            lx,
            Half {
                row: bottom,
                col: lx,
                st: ST_VERTICAL_BOTTOM,
                co: bottom_co,
            },
            Half {
                row: top,
                col: lx,
                st: ST_VERTICAL_TOP,
                co: top_co,
            },
            rev,
        )
    } else {
        let (west, east, west_co, east_co, rev) = if lx < rx {
            (lx, rx, lco, rco, 0)
        } else {
            (rx, lx, rco, lco, 1)
        };
        let west_half = Half {
            row: ly,
            col: west,
            st: ST_HORIZONTAL_LEFT,
            co: west_co,
        };
        let east_half = Half {
            row: ly,
            col: east,
            st: ST_HORIZONTAL_RIGHT,
            co: east_co,
        };
        // the half with something under it is the one the ai measures first
        let (main, second) = if !field.below(ly, west).is_empty() {
            (west_half, east_half)
        } else {
            (east_half, west_half)
        };
        (1, ly, west, main, second, rev)
    };

    let ec = if tory == 0 {
        // the original compares the two cells after placing them, and at the very top of the
        // bottle the upper half is never placed
        second.row != 0 && main.co == second.co
    } else {
        main.co == second.co
    };

    Some(Candidate {
        placement,
        tory,
        row,
        col,
        rev,
        main,
        second,
        ec,
    })
}

/// Read the bottle for the two things that pick the weights: which situation this is, and which
/// side of the bottle is stacked up. Original name: the first half of `aiSetCharacter`.
fn classify(field: &Field, viruses: u32, average_route: f32) -> (Situation, usize) {
    // a column of one colour running down from the top, in the middle where the pills come in
    let mut stack = 0;
    for col in 2..6 {
        let mut run = 0;
        let mut colour = 0;
        for row in 1..4 {
            if run == 0 && !field.at(row, col).is_empty() {
                run = 1;
                colour = field.at(row, col).co;
            }
            if run != 0 {
                let below = field.at(row + 1, col);
                if below.is_empty() || below.co != colour {
                    run = if col == 3 || col == 4 { 2 } else { 0 };
                    break;
                }
            }
        }
        if run != 0 {
            stack = run;
            if run == 2 {
                break;
            }
        }
    }

    // which side is filled to the neck, how high the rest of it reaches, and how many viruses
    // are still down there where a pill can get at them
    let mut wall = 0usize;
    let mut top = 0x11usize;
    let mut reachable_viruses = 0u32;
    let mut blocked_right = COLS;

    for col in 4..COLS {
        let mut row = 1;
        while row < 4 && field.at(row, col).is_empty() {
            row += 1;
        }
        if row < 4 {
            wall |= 2;
            blocked_right = col;
            break;
        }
        for row in row..ROWS {
            if !field.at(row, col).is_empty() && row < top {
                top = row;
            }
            if field.at(row, col).is_virus() {
                reachable_viruses += 1;
            }
        }
    }

    for col in (0..4).rev() {
        let mut row = 1;
        while row < 4 && field.at(row, col).is_empty() {
            row += 1;
        }
        if row < 4 {
            wall |= 1;
            if blocked_right.saturating_sub(col) < 4 {
                // both sides are up and there is no gap left to lean into
                wall = 0;
                reachable_viruses = viruses;
            }
            break;
        }
        for row in row..ROWS {
            if !field.at(row, col).is_empty() && row < top {
                top = row;
            }
            if field.at(row, col).is_virus() {
                reachable_viruses += 1;
            }
        }
    }

    // the deepest virus in the bottle
    let mut deepest = 0;
    for col in 0..COLS {
        for row in (4..ROWS).rev() {
            if field.at(row, col).is_virus() {
                deepest = deepest.max(row);
                break;
            }
        }
    }

    // there is no opponent to be behind, so the original's "am I winning" test is always yes
    let leading = true;

    let situation = if average_route < 4.0 {
        Situation::Cornered
    } else if (viruses < 7 && leading) || reachable_viruses < 3 {
        if reachable_viruses != 0 {
            Situation::Finishing
        } else if top + 4 < deepest || top < 9 {
            Situation::Digging
        } else {
            Situation::Idle
        }
    } else if stack == 2 {
        Situation::CentreStack
    } else if stack == 1 {
        Situation::SideStack
    } else {
        Situation::Normal
    };

    (situation, wall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::features::BottleAnalysis;
    use crate::game::ai::placement::PlacementSearch;
    use crate::game::block::Block;
    use crate::game::bottle::{BOTTLE_FLOOR, BOTTLE_WIDTH};
    use crate::game::pill::PillShape;
    use VirusColor::{Blue, Red, Yellow};

    fn with_pill(shape: PillShape, blocks: &[(u32, u32, Block)]) -> Bottle {
        let mut bottle = Bottle::new();
        for (x, y, block) in blocks {
            bottle.place(*x, *y, *block);
        }
        bottle.try_spawn(shape);
        bottle
    }

    /// where the ai's pick comes to rest, as (x, y, colour) pairs
    fn played(bottle: &Bottle) -> Vec<(i32, i32, VirusColor)> {
        let placements = bottle.placements(bottle.stats());
        let chosen = N64Ai::new()
            .choose(bottle, &placements)
            .expect("nothing chosen");
        let mut cells: Vec<(i32, i32, VirusColor)> = placements[chosen]
            .landing()
            .iter()
            .map(|(point, colour)| (point.x(), point.y(), *colour))
            .collect();
        cells.sort_by_key(|(x, y, _)| (*y, *x));
        cells
    }

    #[test]
    fn takes_the_clear_that_kills_a_virus() {
        // three reds on the floor and a red half will take all three viruses with it
        let bottle = with_pill(
            PillShape::new(Red, Blue),
            &[
                (0, BOTTLE_FLOOR, Block::Virus(Red)),
                (1, BOTTLE_FLOOR, Block::Virus(Red)),
                (2, BOTTLE_FLOOR, Block::Virus(Red)),
            ],
        );
        let cells = played(&bottle);
        assert!(
            cells.contains(&(3, BOTTLE_FLOOR as i32, Red)),
            "did not complete the row: {:?}",
            cells
        );
    }

    #[test]
    fn builds_the_column_a_virus_is_buried_in() {
        // a red virus on the floor with two reds stacked on it: a third red finishes the column
        let bottle = with_pill(
            PillShape::new(Red, Yellow),
            &[
                (4, BOTTLE_FLOOR, Block::Virus(Red)),
                (4, BOTTLE_FLOOR - 1, Block::Garbage(Red)),
                (4, BOTTLE_FLOOR - 2, Block::Garbage(Red)),
            ],
        );
        let cells = played(&bottle);
        assert!(
            cells.contains(&(4, BOTTLE_FLOOR as i32 - 3, Red)),
            "did not cap the column: {:?}",
            cells
        );
    }

    #[test]
    fn every_reachable_placement_is_a_candidate() {
        let bottle = with_pill(PillShape::new(Red, Blue), &[]);
        let placements = bottle.placements(bottle.stats());
        let candidates = candidates(&Field::of(&bottle), &placements);
        assert_eq!(candidates.len(), placements.len());
        // upright first, then flat, and within each by row and then column, which is what
        // settles a tie between two placements worth the same
        assert!(candidates.windows(2).all(|pair| {
            let (a, b) = (&pair[0], &pair[1]);
            (a.tory, a.rev, a.row, a.col) <= (b.tory, b.rev, b.row, b.col)
        }));
    }

    #[test]
    fn a_bottle_with_room_is_not_cornered() {
        let bottle = with_pill(PillShape::new(Red, Blue), &[]);
        let field = Field::of(&bottle);
        let (situation, wall) = classify(&field, 0, routes::average_route(&field));
        assert_ne!(situation, Situation::Cornered);
        assert_eq!(wall, 0);

        // fill everything but the top two rows and there is nowhere left to work
        let mut blocks = vec![];
        for x in 0..BOTTLE_WIDTH {
            for y in 2..=BOTTLE_FLOOR {
                blocks.push((x, y, Block::Garbage(Blue)));
            }
        }
        let bottle = with_pill(PillShape::new(Red, Blue), &blocks);
        let field = Field::of(&bottle);
        let (situation, _) = classify(&field, 0, routes::average_route(&field));
        assert_eq!(situation, Situation::Cornered);
    }

    #[test]
    fn the_same_bottle_always_gets_the_same_answer() {
        let bottle = with_pill(
            PillShape::new(Red, Yellow),
            &[
                (2, BOTTLE_FLOOR, Block::Virus(Blue)),
                (5, BOTTLE_FLOOR, Block::Virus(Yellow)),
                (5, BOTTLE_FLOOR - 1, Block::Garbage(Red)),
            ],
        );
        let first = played(&bottle);
        for _ in 0..4 {
            assert_eq!(played(&bottle), first);
        }
    }
}
