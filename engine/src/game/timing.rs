use std::time::Duration;

/// Lock delay, spawn delay and soft drop rules shared by every falling block game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    /// how long a piece may rest on the stack before it locks
    pub lock: Duration,
    /// lock delay while soft dropping
    pub soft_drop_lock: Duration,
    /// how many moves/rotations may reset the lock delay before the piece is forced to lock
    pub max_lock_placements: u32,
    /// the spawn delay never drops below this (Dr. Mario)
    pub min_spawn_delay: Option<Duration>,
    /// the spawn delay never exceeds this (Tetris)
    pub max_spawn_delay: Option<Duration>,
    /// soft drop divides the fall step by this
    pub soft_drop_step_factor: u32,
    /// soft drop divides the spawn delay by this
    pub soft_drop_spawn_factor: u32,
}

impl Timing {
    pub const fn new(lock: Duration, soft_drop_lock: Duration) -> Self {
        Self {
            lock,
            soft_drop_lock,
            max_lock_placements: 15,
            min_spawn_delay: Some(Duration::from_millis(500)),
            max_spawn_delay: None,
            soft_drop_step_factor: 20,
            soft_drop_spawn_factor: 10,
        }
    }

    /// cap the spawn delay instead of flooring it
    pub const fn with_spawn_delay_cap(self, cap: Duration) -> Self {
        Self {
            min_spawn_delay: None,
            max_spawn_delay: Some(cap),
            ..self
        }
    }

    pub fn lock_duration(&self, soft_drop: bool) -> Duration {
        if soft_drop {
            self.soft_drop_lock
        } else {
            self.lock
        }
    }

    /// the fall step for this level, sped up when soft dropping but never faster than `min_step`
    pub fn step_delay(&self, base: Duration, soft_drop: bool, min_step: Duration) -> Duration {
        if soft_drop {
            (base / self.soft_drop_step_factor).max(min_step)
        } else {
            base
        }
    }

    pub fn spawn_delay(&self, base: Duration, soft_drop: bool, min_step: Duration) -> Duration {
        let mut delay = if soft_drop {
            (base / self.soft_drop_spawn_factor).max(min_step)
        } else {
            base
        };
        if let Some(min) = self.min_spawn_delay {
            delay = delay.max(min);
        }
        if let Some(max) = self.max_spawn_delay {
            delay = delay.min(max);
        }
        delay
    }
}

/// A board that tracks how many times the active piece has moved while resting on the stack.
pub trait LockPlacements {
    fn lock_placements(&self) -> u32;
    /// record a placement and return the new count
    fn register_lock_placement(&mut self) -> u32;
}

/// What a move attempted during lock delay did to the lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMove {
    /// the move was refused: the lock delay had already elapsed or the board blocked it
    Blocked,
    /// the move was refused because the piece had already used all its placements; lock now
    Exhausted,
    /// the move happened. Unless it used the last allowed placement the lock delay restarts.
    Moved { last_placement: bool },
}

/// Attempt a move or rotation while the piece is in its lock delay (`lock_duration` elapsed
/// so far). Encodes the guideline lock-delay rules: a move is refused once the delay has
/// fully elapsed, and each successful move restarts the delay until `max_lock_placements`
/// moves have been made, after which the piece locks immediately.
pub fn lock_move<B: LockPlacements>(
    timing: &Timing,
    lock_duration: Duration,
    board: &mut B,
    mut f: impl FnMut(&mut B) -> bool,
) -> LockMove {
    // 1. the lock is already breached (movements are sent before a lock update)
    if lock_duration >= timing.lock {
        return LockMove::Blocked;
    }
    // 2. this piece used all its lock movements for this altitude
    if board.lock_placements() >= timing.max_lock_placements {
        return LockMove::Exhausted;
    }
    // 3. the movement was blocked by the board
    if !f(board) {
        return LockMove::Blocked;
    }
    let last_placement = board.register_lock_placement() >= timing.max_lock_placements;
    LockMove::Moved { last_placement }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Board {
        placements: u32,
        movable: bool,
    }

    impl LockPlacements for Board {
        fn lock_placements(&self) -> u32 {
            self.placements
        }
        fn register_lock_placement(&mut self) -> u32 {
            self.placements += 1;
            self.placements
        }
    }

    const TIMING: Timing = Timing::new(Duration::from_millis(500), Duration::from_millis(250));

    #[test]
    fn breached_lock_blocks_moves() {
        let mut board = Board {
            placements: 0,
            movable: true,
        };
        assert_eq!(
            lock_move(&TIMING, Duration::from_millis(500), &mut board, |_| true),
            LockMove::Blocked
        );
        assert_eq!(board.placements, 0);
    }

    #[test]
    fn blocked_move_does_not_count() {
        let mut board = Board {
            placements: 0,
            movable: false,
        };
        assert_eq!(
            lock_move(&TIMING, Duration::ZERO, &mut board, |b| b.movable),
            LockMove::Blocked
        );
        assert_eq!(board.placements, 0);
    }

    #[test]
    fn moves_reset_lock_until_placements_run_out() {
        let mut board = Board {
            placements: 13,
            movable: true,
        };
        assert_eq!(
            lock_move(&TIMING, Duration::ZERO, &mut board, |_| true),
            LockMove::Moved {
                last_placement: false
            }
        );
        assert_eq!(
            lock_move(&TIMING, Duration::ZERO, &mut board, |_| true),
            LockMove::Moved {
                last_placement: true
            }
        );
        assert_eq!(
            lock_move(&TIMING, Duration::ZERO, &mut board, |_| true),
            LockMove::Exhausted
        );
        assert_eq!(board.placements, 15);
    }

    #[test]
    fn soft_drop_speeds_up_but_respects_minimum() {
        let base = Duration::from_millis(1000);
        assert_eq!(
            TIMING.step_delay(base, false, Duration::from_millis(16)),
            base
        );
        assert_eq!(
            TIMING.step_delay(base, true, Duration::from_millis(16)),
            Duration::from_millis(50)
        );
        assert_eq!(
            TIMING.step_delay(base, true, Duration::from_millis(100)),
            Duration::from_millis(100)
        );
        assert_eq!(
            TIMING.spawn_delay(base, true, Duration::ZERO),
            Duration::from_millis(500)
        );
        let capped = TIMING.with_spawn_delay_cap(Duration::from_millis(500));
        assert_eq!(
            capped.spawn_delay(base, false, Duration::ZERO),
            Duration::from_millis(500)
        );
        assert_eq!(
            capped.spawn_delay(base, true, Duration::ZERO),
            Duration::from_millis(100)
        );
    }
}
