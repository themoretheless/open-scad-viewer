//! Source-owned natural support edges, exact world nets and neutral auxiliary wires.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_ratio, ratio};
use nurbs_core::surface::Surface;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicSupportWireRole {
    Authored { wire: usize },
    NaturalSupport,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalNaturalBoundaryCurve {
    pub degree: usize,
    pub knots: Vec<ExactRatio>,
    /// Original exact homogeneous world controls; never rounded to f64 poles.
    pub controls: Vec<[ExactRatio; 4]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicNaturalBoundary {
    pub fixed_axis: usize,
    pub end: usize,
    pub source: usize,
    pub wire: usize,
    pub reversed: bool,
    pub level: ExactRatio,
    pub native_domain: [ExactRatio; 2],
    pub world: OriginalNaturalBoundaryCurve,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportBoundaries {
    original: Arc<OwnedOriginalPeriodicWires>,
    augmented: Arc<OwnedOriginalPeriodicWires>,
    max_work: usize,
    roles: Vec<OriginalPeriodicSupportWireRole>,
    boundaries: Vec<OriginalPeriodicNaturalBoundary>,
    work: usize,
}
fn bump(work: &mut usize, max: usize) -> Result<()> {
    if *work >= max {
        return Err(refused(
            "Complete natural support source construction exhausted its allowance",
        ));
    }
    *work += 1;
    Ok(())
}
fn exact(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| refused("Natural boundary source has a nonfinite coefficient"))
}


fn scalar_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
}
fn row_bits(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| scalar_bits(a, b))
}
fn curves_bits(a: &[Curve], b: &[Curve]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.degree == b.degree
                && a.periodic == b.periodic
                && scalar_bits(&a.knots, &b.knots)
                && scalar_bits(&a.weights, &b.weights)
                && row_bits(&a.control_points, &b.control_points)
        })
}
fn surface_bits(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u
        && a.periodic_v == b.periodic_v
        && scalar_bits(&a.knots_u, &b.knots_u)
        && scalar_bits(&a.knots_v, &b.knots_v)
        && row_bits(&a.weights, &b.weights)
        && a.control_points.len() == b.control_points.len()
        && a.control_points
            .iter()
            .zip(&b.control_points)
            .all(|(a, b)| row_bits(a, b))
}
struct Plan {
    sources: Vec<Curve>,
    uses: Vec<Vec<OriginalPeriodicSourceUse>>,
    roles: Vec<OriginalPeriodicSupportWireRole>,
    boundaries: Vec<OriginalPeriodicNaturalBoundary>,
    work: usize,
}

fn derive(
    o: &OwnedPeriodicSupportBoundaries,
) -> Result<(
    Vec<OriginalPeriodicSupportWireRole>,
    Vec<OriginalPeriodicNaturalBoundary>,
    usize,
)> {
    let p = plan(&o.original, o.max_work)?;
    super::periodic_replay::verify_arc(&o.augmented, |x| x.reverify_source())?;
    if !surface_bits(
        o.original.surface().surface(),
        o.augmented.surface().surface(),
    ) || o.original.surface().pieces().wrapped() != o.augmented.surface().pieces().wrapped()
        || o.augmented.source_uses() != p.uses
        || !curves_bits(&p.sources, o.augmented.surface().pieces().sources())
    {
        return Err(refused(
            "Natural support augmentation changed original surface bits, source curves or wire roles",
        ));
    }
    Ok((p.roles, p.boundaries, p.work))
}
impl OwnedPeriodicSupportBoundaries {
    pub fn original(&self) -> &Arc<OwnedOriginalPeriodicWires> {
        &self.original
    }
    pub fn augmented(&self) -> &Arc<OwnedOriginalPeriodicWires> {
        &self.augmented
    }
    pub fn roles(&self) -> &[OriginalPeriodicSupportWireRole] {
        &self.roles
    }
    pub fn boundaries(&self) -> &[OriginalPeriodicNaturalBoundary] {
        &self.boundaries
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (r, b, w) = derive(self)?;
        if r != self.roles || b != self.boundaries || w != self.work {
            return Err(refused(
                "Natural support source roles, original world nets or work changed",
            ));
        }
        Ok(())
    }
    /// Auxiliary support wires carry ZERO authored-material weight. The caller
    /// retains its original fill rule and declared base value unchanged.
    pub fn lift_operand(
        &self,
        operand: &OriginalPeriodicOperand,
    ) -> Result<OriginalPeriodicOperand> {
        self.reverify_source()?;
        if operand.wire_weights.len() != self.original.wires().len() {
            return Err(refused(
                "Material operand does not cover the original authored wire columns",
            ));
        }
        let mut out = operand.clone();
        out.wire_weights = self
            .roles
            .iter()
            .map(|r| match *r {
                OriginalPeriodicSupportWireRole::Authored { wire } => operand.wire_weights[wire],
                OriginalPeriodicSupportWireRole::NaturalSupport => 0,
            })
            .collect();
        Ok(out)
    }
    /// Evaluate the original surface at exact rational UV coordinates. Wrapped
    /// coordinates are reduced by exact whole periods; unwrapped coordinates
    /// must remain in the closed original active knot domain. This preserves
    /// the represented author function, without constructing rounded poles.
    /// A point query alone does not qualify a whole edge or publish a face.
    pub fn world_uv_point(&self, uv: &[ExactRatio; 2]) -> Result<[ExactRatio; 3]> {
        self.reverify_source()?;
        let source = self.original.surface();
        let s = source.surface();
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v]
            .map(|k| k.iter().copied().map(exact).collect::<Result<Vec<_>>>())
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let wrapped = source.pieces().wrapped();
        let mut work = 0;
        let mut values = Vec::new();
        for axis in 0..2 {
            let mut t = decode_ratio(&uv[axis])?;
            let lo = &knots[axis][degrees[axis]];
            let hi = &knots[axis][counts[axis]];
            if wrapped[axis] && (t < *lo || t > *hi) {
                let period = hi - lo;
                let turns = ((&t - lo) / &period).floor();
                t -= turns * period;
            }
            values.push(basis(
                degrees[axis],
                &knots[axis],
                &t,
                &mut work,
                self.max_work,
            )?);
        }
        let mut h = std::array::from_fn::<_, 4, _>(|_| Q::zero());
        for (u, a) in values[0].iter().enumerate() {
            for (v, b) in values[1].iter().enumerate() {
                bump(&mut work, self.max_work)?;
                let w = a * b * exact(s.weights[u][v])?;
                for axis in 0..3 {
                    h[axis] += &w * exact(s.control_points[u][v][axis])?;
                }
                h[3] += w;
            }
        }
        if h[3] <= Q::zero() {
            return Err(refused(
                "Original surface query denominator is not positive",
            ));
        }
        Ok(std::array::from_fn(|axis| ratio(&(&h[axis] / &h[3]))))
    }
    /// Exact world point in the ORIGINAL varying-axis clock, even for nonclamped
    /// fixed-axis boundaries whose world poles cannot be represented in f64.
    pub fn world_point(&self, boundary: usize, parameter: &ExactRatio) -> Result<[ExactRatio; 3]> {
        self.reverify_source()?;
        let b = self
            .boundaries
            .get(boundary)
            .ok_or_else(|| refused("Unknown actual natural boundary source"))?;
        let knots = b
            .world
            .knots
            .iter()
            .map(decode_ratio)
            .collect::<Result<Vec<_>>>()?;
        let mut work = 0;
        let values = basis(
            b.world.degree,
            &knots,
            &decode_ratio(parameter)?,
            &mut work,
            self.max_work,
        )?;
        let mut h = std::array::from_fn::<_, 4, _>(|_| Q::zero());
        for (w, c) in values.iter().zip(&b.world.controls) {
            bump(&mut work, self.max_work)?;
            for axis in 0..4 {
                h[axis] += w * decode_ratio(&c[axis])?;
            }
        }
        if h[3] <= Q::zero() {
            return Err(refused(
                "Original natural boundary denominator is not positive",
            ));
        }
        Ok(std::array::from_fn(|axis| ratio(&(&h[axis] / &h[3]))))
    }
}
/// Add every natural unwrapped-domain edge to the ORIGINAL source census.
/// UV lines use exact native axis clocks; world nets use exact homogeneous
/// source basis values. This is geometry/role ownership, not half-disc sectors,
/// manifold face publication or new supporting-surface embedding authority.
pub fn own_periodic_support_boundaries(
    original: Arc<OwnedOriginalPeriodicWires>,
    width: f64,
    limits: ParameterOrderLimits,
    max_levels: usize,
    max_work: usize,
) -> Result<OwnedPeriodicSupportBoundaries> {
    let _replay = super::periodic_replay::scope();
    if !width.is_finite()
        || !(1e-18..=1e-3).contains(&width)
        || limits.max_refinements > 512
        || !(1..=128).contains(&limits.max_resultant_size)
        || !(1..=4096).contains(&max_levels)
    {
        return Err(refused(
            "Invalid natural support source construction settings",
        ));
    }
    let p = plan(&original, max_work)?;
    let augmented = if p.boundaries.is_empty() {
        original.clone()
    } else {
        let source = original.surface();
        let cover = Arc::new(own_original_periodic_pieces(
            &p.sources,
            source.pieces().chart(),
            source.pieces().wrapped(),
            width,
            limits,
            max_levels,
        )?);
        let support = Arc::new(bind_original_periodic_pieces_to_surface(
            cover,
            source.surface(),
            limits,
        )?);
        Arc::new(own_original_periodic_wires(support, &p.uses)?)
    };
    let o = OwnedPeriodicSupportBoundaries {
        original,
        augmented,
        max_work,
        roles: p.roles,
        boundaries: p.boundaries,
        work: p.work,
    };
    o.reverify_source()?;
    Ok(o)
}
#[cfg(test)]
#[path = "periodic_support_boundaries/tests/tests_fixtures.rs"]
pub(in crate::profile_certificate::span_intersections::translated) mod tests ;

#[path = "periodic_support_boundaries/support_plan.rs"]
mod support_plan;
use support_plan::plan;
use support_plan::world_curve;
use support_plan::basis;
