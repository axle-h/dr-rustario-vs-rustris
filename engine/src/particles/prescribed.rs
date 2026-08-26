use crate::game::geometry::Point as CellPoint;
use crate::game::{CellId, PieceId, PlacedCell};
use crate::particles::particle::ParticleAnimationType;
use crate::render::context::ThemeContext;
use crate::render::sprite_sheet::MascotKind;
use crate::particles::color::ParticleColor;
use crate::particles::geometry::Vec2D;
use crate::particles::meta::ParticleSprite;
use crate::particles::quantity::ProbabilityTable;
use crate::particles::scale::Scale;
use crate::particles::source::{
    AggregateParticleSource, ParticleModulation, ParticleProperties, ParticleSource,
    RandomParticleSource,
};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use std::time::Duration;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlayerParticleTarget {
    /// whole cells
    Cells(Vec<CellPoint>),
    /// the opaque pixels of these cells, so particles take their shape
    MaskedCells(Vec<PlacedCell>),
    /// whole rows of the board
    Rows(Vec<u32>),
    Board,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrescribedParticles {
    FadeInLatticeBurstAndFall { fade_in: Duration, color: Color },
    LightBurstUpAndOut { color: Color },
    BurstUp { color: Color },
    BurstDown { color: Color },
    PerimeterBurst { color: Color },
    PerimeterSpray { color: Color },
}

impl PrescribedParticles {
    pub fn into_targeted(
        self,
        player: u32,
        target: PlayerParticleTarget,
    ) -> PlayerTargetedParticles {
        PlayerTargetedParticles {
            player,
            target,
            particles: self,
        }
    }

    pub fn into_lattice_source(
        self,
        scale: &Scale,
        mut lattice: Vec<Point>,
        n_blocks: u32,
        is_horizontal: bool,
    ) -> Box<dyn ParticleSource> {
        match self {
            PrescribedParticles::FadeInLatticeBurstAndFall { fade_in, color } => {
                let limit = lattice.len() as u32 / n_blocks;
                if is_horizontal {
                    lattice.sort_by(|p1, p2| p2.x().cmp(&p1.x()));
                } else {
                    lattice.sort_by(|p1, p2| p1.y().cmp(&p2.y()));
                }

                RandomParticleSource::new(
                    scale.build_ephemeral_lattice(lattice.into_iter()),
                    ParticleModulation::Constant {
                        count: limit,
                        step: Duration::from_millis(200) / n_blocks,
                    },
                )
                .with_static_properties(
                    ParticleSprite::Circle05,
                    ParticleColor::from_sdl(color),
                    1.0,
                    0.0,
                )
                .with_velocity((Vec2D::new(0.0, -0.4), Vec2D::new(0.1, 0.1)))
                .with_acceleration(Vec2D::new(0.0, 1.5)) // gravity
                .with_anchor(fade_in)
                .with_fade_in(fade_in)
                .with_alpha((0.9, 0.1))
                .into_box()
            }
            _ => unreachable!(),
        }
    }

    pub fn into_source(self, scale: &Scale, rects: &[Rect]) -> Box<dyn ParticleSource> {
        match self {
            PrescribedParticles::FadeInLatticeBurstAndFall { fade_in, color } => {
                RandomParticleSource::new(
                    scale.rect_lattice_source(rects),
                    ParticleModulation::Cascade,
                )
                .with_static_properties(
                    ParticleSprite::Circle05,
                    ParticleColor::from_sdl(color),
                    1.0,
                    0.0,
                )
                .with_velocity((Vec2D::new(0.0, -0.4), Vec2D::new(0.1, 0.1)))
                .with_acceleration(Vec2D::new(0.0, 1.5)) // gravity
                .with_anchor(fade_in)
                .with_fade_in(fade_in)
                .with_alpha((0.9, 0.1))
                .into_box()
            }
            PrescribedParticles::LightBurstUpAndOut { color } => RandomParticleSource::burst(
                scale.rect_lattice_source(rects),
                ParticleSprite::Circle05,
                ParticleColor::from_sdl(color),
                (Vec2D::new(0.0, -0.1), Vec2D::new(0.2, 0.2)),
                (1.0, 0.1),
                (0.4, 0.1),
            )
            .into_box(),
            PrescribedParticles::BurstUp { color } => RandomParticleSource::burst(
                scale.rect_lattice_source(rects),
                ParticleSprite::Circle05,
                ParticleColor::from_sdl(color),
                (Vec2D::new(0.0, -0.2), Vec2D::new(0.05, 0.1)),
                (1.0, 0.1),
                (0.7, 0.3),
            )
            .into_box(),
            PrescribedParticles::BurstDown { color } => RandomParticleSource::burst(
                scale.rect_lattice_source(rects),
                ParticleSprite::Circle05,
                ParticleColor::from_sdl(color),
                (Vec2D::new(0.0, 0.2), Vec2D::new(0.1, 0.1)),
                (1.0, 0.1),
                (0.7, 0.3),
            )
            .into_box(),
            PrescribedParticles::PerimeterBurst { color } => {
                let color = ParticleColor::from_sdl(color);
                let sources = rects
                    .iter()
                    .flat_map(|r| perimeter_sources(scale, *r, color))
                    .collect();
                AggregateParticleSource::new(sources).into_box()
            }
            PrescribedParticles::PerimeterSpray { color } => {
                let color = ParticleColor::from_sdl(color);
                let sources = rects
                    .iter()
                    .flat_map(|r| perimeter_sources(scale, *r, color))
                    .map(|s| {
                        s.with_modulation(ParticleModulation::Constant {
                            count: u32::MAX,
                            step: Duration::from_millis(750),
                        })
                    })
                    .collect();
                AggregateParticleSource::new(sources).into_box()
            }
        }
    }
}

pub fn prescribed_fireworks(window: Rect, scale: &Scale) -> Box<dyn ParticleSource> {
    let modulation = ParticleModulation::Constant {
        count: 100,
        step: Duration::from_millis(500),
    };
    let buffer = window.height() / 5;
    let rect = Rect::from_center(
        window.center(),
        window.width() - buffer,
        window.height() - buffer,
    );
    RandomParticleSource::new(scale.random_rect_source(rect), modulation)
        .with_static_properties(
            ParticleSprite::Circle05,
            (
                ParticleColor::rgb(0.5, 0.5, 0.5),
                ParticleColor::rgb(0.5, 0.5, 0.5),
            ),
            1.0,
            0.0,
        )
        .with_velocity((Vec2D::new(0.0, -0.05), Vec2D::new(0.15, 0.15)))
        .with_fade_out((1.5, 0.5))
        .with_acceleration(Vec2D::new(0.0, 0.1)) // gravity
        .with_alpha((0.9, 0.1))
        .into_box()
}

/// What one theme contributes to the piece race: `theme` indexes the particle renderer's
/// themes, `scale` shrinks its sprites to match the other themes.
#[derive(Clone, Debug)]
pub struct RaceTheme {
    pub theme: usize,
    pub pieces: Vec<PieceId>,
    pub cells: Vec<(CellId, ParticleAnimationType)>,
    pub mascot: Option<ParticleAnimationType>,
    pub scale: f64,
}

/// Pieces, animated cells and mascots from every theme drift across the window.
pub fn prescribed_piece_race(
    window: Rect,
    scale: &Scale,
    themes: &[RaceTheme],
) -> Box<dyn ParticleSource> {
    let modulation = ParticleModulation::Constant {
        count: 1,
        step: Duration::from_millis(1000),
    };
    let buffer_y = window.height() / 10;
    let rect = Rect::new(
        window.left() - 50,
        window.top() + buffer_y as i32,
        50,
        window.height() - 2 * buffer_y,
    );
    let rotation = (0.0, 30.0);
    let p_cell = 1.0 / 3.0;
    let p_mascot = 1.0 / 3.0;
    let mut table = ProbabilityTable::new();
    for race in themes {
        let size = (race.scale, race.scale / 5.0);
        let pieces = race
            .pieces
            .iter()
            .map(|piece| ParticleSprite::Piece {
                theme: race.theme,
                piece: *piece,
            })
            .collect::<Vec<ParticleSprite>>();
        if !pieces.is_empty() {
            table = table.with_1(ParticleProperties::simple(&pieces, size).angular_velocity(rotation));
        }
        let cells = race
            .cells
            .iter()
            .map(|(cell, animation)| ParticleSprite::Cell {
                theme: race.theme,
                cell: *cell,
                animation: *animation,
            })
            .collect::<Vec<ParticleSprite>>();
        if !cells.is_empty() {
            table = table.with(
                ParticleProperties::simple(&cells, size).angular_velocity(rotation),
                p_cell,
            );
        }
        if let Some(animation) = race.mascot {
            table = table.with(
                ParticleProperties::simple(
                    &[ParticleSprite::Mascot {
                        theme: race.theme,
                        kind: MascotKind::Idle,
                        animation,
                    }],
                    (race.scale / 2.0, race.scale / 10.0),
                )
                .angular_velocity(rotation),
                p_mascot,
            );
        }
    }
    RandomParticleSource::new(scale.rect_source(rect), modulation)
        .with_properties(table)
        .with_velocity((Vec2D::new(0.2, 0.0), Vec2D::new(0.05, 0.02)))
        .with_alpha((0.9, 0.1))
        .into_box()
}

fn perimeter_sources(scale: &Scale, rect: Rect, color: ParticleColor) -> [RandomParticleSource; 4] {
    const V: f64 = 0.2;
    const FADE_OUT: (f64, f64) = (1.0, 0.1);
    const ALPHA: (f64, f64) = (0.7, 0.3);
    const SPRITE: ParticleSprite = ParticleSprite::Circle05;
    let [top, right, bottom, left] = scale.perimeter_lattice_sources(rect);
    [
        RandomParticleSource::burst(
            top,
            SPRITE,
            color,
            (Vec2D::new(0.0, -V), Vec2D::new(0.2, 0.1)),
            FADE_OUT,
            ALPHA,
        ),
        RandomParticleSource::burst(
            right,
            SPRITE,
            color,
            (Vec2D::new(V, 0.0), Vec2D::new(0.1, 0.2)),
            FADE_OUT,
            ALPHA,
        ),
        RandomParticleSource::burst(
            bottom,
            SPRITE,
            color,
            (Vec2D::new(0.0, V), Vec2D::new(0.2, 0.1)),
            FADE_OUT,
            ALPHA,
        ),
        RandomParticleSource::burst(
            left,
            SPRITE,
            color,
            (Vec2D::new(-V, 0.0), Vec2D::new(0.1, 0.2)),
            FADE_OUT,
            ALPHA,
        ),
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerTargetedParticles {
    player: u32,
    target: PlayerParticleTarget,
    particles: PrescribedParticles,
}

impl PlayerTargetedParticles {
    pub fn into_source(
        self,
        themes: &ThemeContext,
        particle_scale: &Scale,
    ) -> Box<dyn ParticleSource> {
        let target_rects = match self.target {
            PlayerParticleTarget::Board => vec![themes.player_board_snip(self.player)],
            PlayerParticleTarget::Cells(cells) => themes.player_block_snips(self.player, cells),
            PlayerParticleTarget::Rows(rows) => themes.player_row_snips(self.player, rows),
            PlayerParticleTarget::MaskedCells(cells) => {
                let is_horizontal = iter_all_eq(cells.iter().map(|(p, _)| p.y));
                let n_blocks = cells.len();
                let points = themes.player_block_snips_masked(self.player, cells, 5);
                return self.particles.into_lattice_source(
                    particle_scale,
                    points,
                    n_blocks as u32,
                    is_horizontal,
                );
            }
        };

        self.particles
            .into_source(particle_scale, target_rects.as_slice())
    }
}

pub fn iter_all_eq<T: PartialEq>(iter: impl IntoIterator<Item = T>) -> bool {
    let mut iter = iter.into_iter();
    iter.next()
        .map(|first| iter.all(|elem| elem == first))
        .unwrap_or(false)
}
