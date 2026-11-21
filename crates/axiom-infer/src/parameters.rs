use axiom_model::Parameter;
use axiom_sim::SplitMix64;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};

use crate::black_box::in_range;
use crate::error::InferError;

/// The bits of each draw used for a parameter value: the range is cut into
/// `2^32` equal slots and a value is the exact midpoint of a uniformly drawn
/// slot.
pub const POINT_BITS: u32 = 32;

/// The most parameter points one request may draw.
pub const MAX_POINTS: usize = 1_000_000;

/// The most cells a coverage grid may have.
pub const MAX_CELLS: u64 = 1 << 20;

/// Draws `count` parameter points uniformly from the box of the parameters'
/// ranges, one value per parameter per point, with a [`SplitMix64`] seeded
/// by `seed`. Each value is the exact rational midpoint
/// `lo + (hi - lo) (2m + 1) / 2^33` of slot `m`, drawn uniformly from the
/// `2^32` slots, so it lies strictly inside the range whether its ends are
/// open or closed.
///
/// # Errors
///
/// [`InferError::TooManyPoints`] when `count` is above [`MAX_POINTS`].
pub fn sample_parameters(
    parameters: &[Parameter],
    count: usize,
    seed: u64,
) -> Result<Vec<Vec<BigRational>>, InferError> {
    if count > MAX_POINTS {
        return Err(InferError::TooManyPoints { limit: MAX_POINTS });
    }
    let slots = BigInt::from(1_u64 << (POINT_BITS + 1));
    let mut rng = SplitMix64::new(seed);
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let point = parameters
            .iter()
            .map(|parameter| {
                let slot = rng.next_u64() >> (64 - POINT_BITS);
                let width = &parameter.upper.value - &parameter.lower.value;
                let offset = BigRational::new(BigInt::from(2 * slot + 1), slots.clone());
                &parameter.lower.value + width * offset
            })
            .collect();
        points.push(point);
    }
    Ok(points)
}

/// How much of the parameter box a set of points covers: the box is cut
/// into `per_axis` equal intervals along each parameter, and `hit` of the
/// `cells` cells hold at least one point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    /// The cells that hold a point.
    pub hit: u64,
    /// The cells of the grid.
    pub cells: u64,
}

impl Coverage {
    /// The covered fraction `hit / cells`.
    #[must_use]
    pub fn fraction(&self) -> BigRational {
        BigRational::new(BigInt::from(self.hit), BigInt::from(self.cells))
    }
}

/// The [`Coverage`] of `points` on the grid with `per_axis` intervals per
/// parameter.
///
/// # Errors
///
/// [`InferError::InvalidGrid`] when the grid has no cells or more than
/// [`MAX_CELLS`], [`InferError::TooManyPoints`] for more than
/// [`MAX_POINTS`] points, [`InferError::ParameterCount`] for a point of
/// the wrong length and [`InferError::InvalidInstance`] for a value outside
/// its range.
pub fn coverage(
    parameters: &[Parameter],
    points: &[Vec<BigRational>],
    per_axis: u64,
) -> Result<Coverage, InferError> {
    let grid = InferError::InvalidGrid { limit: MAX_CELLS };
    let cells = u32::try_from(parameters.len())
        .ok()
        .and_then(|dimensions| per_axis.checked_pow(dimensions))
        .filter(|cells| (1..=MAX_CELLS).contains(cells) && per_axis > 0)
        .ok_or(grid)?;
    if points.len() > MAX_POINTS {
        return Err(InferError::TooManyPoints { limit: MAX_POINTS });
    }
    let mut seen = vec![false; usize::try_from(cells).unwrap_or(0)];
    for point in points {
        if point.len() != parameters.len() {
            return Err(InferError::ParameterCount {
                expected: parameters.len(),
                actual: point.len(),
            });
        }
        let mut index = 0_u64;
        for (parameter, value) in parameters.iter().zip(point) {
            if !in_range(parameter, value) {
                return Err(InferError::InvalidInstance);
            }
            index = index * per_axis + cell(parameter, value, per_axis);
        }
        if let Some(slot) = usize::try_from(index).ok().and_then(|i| seen.get_mut(i)) {
            *slot = true;
        }
    }
    let hit = seen.iter().filter(|&&cell| cell).count() as u64;
    Ok(Coverage { hit, cells })
}

/// The interval of `value` along one axis cut into `per_axis` pieces:
/// `⌊(value - lo) / (hi - lo) · per_axis⌋`, with the upper end in the last
/// piece and a single-point range in the first.
fn cell(parameter: &Parameter, value: &BigRational, per_axis: u64) -> u64 {
    let width = &parameter.upper.value - &parameter.lower.value;
    if width.is_zero() {
        return 0;
    }
    let scaled = (value - &parameter.lower.value) / width * BigInt::from(per_axis);
    scaled
        .floor()
        .to_integer()
        .to_u64()
        .unwrap_or(0)
        .min(per_axis - 1)
}
