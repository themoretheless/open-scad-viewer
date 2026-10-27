//! Continuous branch boxes, not a hull of sampled stations. The owner proves
//! original support membership; this mutable diagnostic is not topology authority.
use super::*;
#[derive(Clone, Debug)]
pub struct OriginalGraphSSRange {
    pub clock: [f64; 2],
    pub normalized_uv: [[f64; 2]; 2],
    pub first_uv: [[f64; 2]; 2],
    pub second_uv: [[f64; 2]; 2],
    /// Contains the entire original spatial branch over `clock`, including
    /// curved original height terms and binary64 rounding.
    pub world: [[f64; 2]; 3],
    pub operations: usize,
}
impl QualifiedQuadraticGraphSS {
    /// Enclose EVERY point in a closed clock range of one owned branch edge.
    /// No samples are used. Consumers must retain this original-source owner;
    /// the returned public coordinates alone do not authorize a trim or Solid.
    pub fn edge_enclosure(
        &self,
        edge: usize,
        clock: [f64; 2],
        max_operations: usize,
    ) -> Result<OriginalGraphSSRange> {
        check(
            clock.iter().all(|t| t.is_finite())
                && 0. <= clock[0]
                && clock[0] <= clock[1]
                && clock[1] <= 1.,
            "Original edge range must lie in0..1 in ascending order",
        )?;
        let mut w = Work::new(max_operations)?;
        let edge = self
            .edges()
            .get(edge)
            .ok_or_else(|| crate::input("Unknown original graph edge"))?;
        let t = I::new(clock[0], clock[1])?;
        let mut uv = match edge.geometry {
            OriginalGraphSSEdgeGeometry::EllipseQuarter(q) => {
                self.ellipse_interval(q, t, &mut w)?
            }
            OriginalGraphSSEdgeGeometry::Segment(points) => {
                let mut uv = [I::point(0.); 2];
                for axis in 0..2 {
                    let delta = w.is(I::point(points[1][axis]), I::point(points[0][axis]))?;
                    let scaled = w.im(delta, t)?;
                    uv[axis] = w.ia(I::point(points[0][axis]), scaled)?;
                }
                uv
            }
        };
        // Complete ellipse containment / exact original rectangle clipping
        // already prove every actual parameter lies in the original domain.
        // Intersect the interval overestimate with that owned geometric fact.
        for axis in 0..2 {
            uv[axis] = uv[axis].intersect(0., 1.)?;
        }
        let enclosed = self.enclose(uv, &mut w)?;
        Ok(OriginalGraphSSRange {
            clock,
            normalized_uv: enclosed.normalized_uv,
            first_uv: enclosed.first_uv,
            second_uv: enclosed.second_uv,
            world: enclosed.world,
            operations: w.used,
        })
    }
}
