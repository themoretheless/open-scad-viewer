//! Native intersection parameter and budget validation.
use super::{Result, dot, invalid};

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    /// The equation is normal.dot(point) = offset; need not be normalized.
    pub normal: [f64; 3],
    pub offset: f64,
}

impl Plane {
    pub(crate) fn normalized(self) -> Result<Self> {
        if !self.normal.iter().all(|v| v.is_finite()) || !self.offset.is_finite() {
            return Err(invalid(
                "Plane needs a finite, nonzero normal and finite offset",
            ));
        }
        let scale = self.normal.iter().map(|v| v.abs()).fold(0., f64::max);
        if scale == 0. {
            return Err(invalid(
                "Plane needs a finite, nonzero normal and finite offset",
            ));
        }
        // Never form the original norm: it may overflow, or round a
        // subnormal vector's length down to a single component. The scaled
        // norm is in [1,sqrt(3)] even at binary64's extreme exponents.
        let scaled = self.normal.map(|x| x / scale);
        let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
        // Divide the larger offset by length first when offset/scale could
        // overflow. For a smaller offset, divide by scale first so a tiny
        // offset is not lost before a subsequent division by a tiny scale.
        let scaled_offset = self.offset / scale;
        let offset = if !scaled_offset.is_finite() {
            (self.offset / length) / scale
        } else {
            scaled_offset / length
        };
        let plane = Self {
            normal: scaled.map(|x| x / length),
            offset,
        };
        if !plane.offset.is_finite() {
            return Err(invalid("Normalized plane offset is not finite"));
        }
        Ok(plane)
    }
    pub(crate) fn distance(self, point: [f64; 3]) -> f64 {
        dot(self.normal, point) - self.offset
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Absolute model-space residual target. Does not grow during a query.
    pub distance_tolerance: f64,
    /// Absolute parameter interval target, in the input knot coordinates.
    pub parameter_tolerance: f64,
    pub max_depth: usize,
    pub max_boxes: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            distance_tolerance: 1e-9,
            parameter_tolerance: 1e-10,
            max_depth: 48,
            max_boxes: 8192,
        }
    }
}
impl Options {
    pub(crate) fn validate(self) -> Result<Self> {
        if !self.distance_tolerance.is_finite()
            || self.distance_tolerance <= 0.
            || !self.parameter_tolerance.is_finite()
            || self.parameter_tolerance <= 0.
            || !(1..=64).contains(&self.max_depth)
            || !(1..=65536).contains(&self.max_boxes)
        {
            return Err(invalid(
                "Intersection tolerances must be positive and finite; depth 1..64, boxes 1..65536",
            ));
        }
        Ok(self)
    }
}
