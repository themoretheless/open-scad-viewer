//! Lazy rotational copies of a rational tooth-profile segment about the Z axis.
use super::{certified_pi, quintic_certificate::trig};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};

pub struct CircularProfilePattern {
    seed: Curve,
    count: usize,
}

impl CircularProfilePattern {
    pub fn new(seed: Curve, count: usize) -> Result<Self> {
        seed.validate()?;
        check(matches!(seed.control_points[0].len(), 2 | 3), "Expected XY or XYZ profile")?;
        check((1..=65536).contains(&count), "Expected 1..65536 rotational copies")?;
        Ok(Self { seed, count })
    }

    /// The bound covers the entire returned rational curve relative to the
    /// ideal rotation of the stored seed, independently of any seed fitting error.
    pub fn instance(&self, index: usize) -> Result<(Curve, f64)> {
        check(index < self.count, "Rotational instance index outside pattern")?;
        let quarter_steps = 4 * index;
        let mut curve = self.seed.clone();
        if quarter_steps % self.count == 0 {
            for pole in &mut curve.control_points {
                let (x, y) = (pole[0], pole[1]);
                (pole[0], pole[1]) = match quarter_steps / self.count {
                    0 => (x, y), 1 => (-y, x), 2 => (-x, -y), _ => (y, -x),
                };
            }
            return Ok((curve, 0.));
        }
        let angle = certified_pi()?.mul(I::point((2 * index) as f64))?.div(I::point(self.count as f64))?;
        let (sin, cos) = trig(angle)?;
        let mut error: f64 = 0.;
        for pole in &mut curve.control_points {
            let (x, y) = (I::point(pole[0]), I::point(pole[1]));
            let bounds = [x.mul(cos)?.sub(y.mul(sin)?)?, x.mul(sin)?.add(y.mul(cos)?)?];
            let mut pole_error = I::point(0.);
            for (coordinate, range) in pole.iter_mut().zip(bounds) {
                *coordinate = range.lo.midpoint(range.hi);
                let delta = range.sub(I::point(*coordinate))?;
                pole_error = pole_error.add(I::point(delta.lo.abs().max(delta.hi.abs())))?;
            }
            // Positive rational basis weights form a convex combination of pole errors.
            error = error.max(pole_error.hi);
        }
        Ok((curve, error))
    }
}
