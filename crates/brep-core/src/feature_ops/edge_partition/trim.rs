//! Exact ownership partitions the ORIGINAL authored relation. Stored binary64
//! child clocks are proposals, admitted against that original relation anew.
use super::*;
use brep_topology::CoedgeTrim;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
use nurbs_core::{curve::Curve, interval_eval::Interval as I, surface::Surface};
#[derive(Clone, Debug, PartialEq)]
pub struct RelationFragment {
    /// Exact rational fraction of the original authored affine relation.
    pub relation_range: [Q; 2],
    pub edge: usize,
    pub transferred: CoedgeTrim,
    /// Whole original requested world clock versus the actual saved UV lift.
    pub original_distance_upper: f64,
    /// Original requested world clock versus the stored child 3D curve.
    pub original_curve_distance_upper: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredRelation {
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
    pub original: CoedgeTrim,
    /// Ordered in original relation direction, covering exactly [0,1].
    pub fragments: Vec<RelationFragment>,
}
fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
fn rounded(x: &Q, domain: [f64; 2]) -> Result<f64> {
    let v = x
        .to_f64()
        .ok_or_else(|| error("Relation fragment scalar is not representable"))?;
    if !v.is_finite() || v < domain[0] || v > domain[1] {
        return Err(error("Rounded fragment leaves its actual child domain"));
    }
    Ok(v)
}
fn upper(x: Q) -> Result<f64> {
    let x = if x < Q::zero() { -x } else { x };
    let mut v = x
        .to_f64()
        .ok_or_else(|| error("Fragment clock error overflow"))?;
    if !v.is_finite() {
        return Err(error("Fragment clock error overflow"));
    }
    if q(v) < x {
        v = v.next_up();
    }
    if !v.is_finite() {
        return Err(error("Fragment clock error overflow"));
    }
    Ok(v)
}
pub(super) struct Transfer<'a> {
    original: CoedgeTrim,
    source: &'a Curve,
    surface: &'a Surface,
    reversed: bool,
    face: usize,
    wire: usize,
    slot: usize,
    fragments: Vec<RelationFragment>,
}
impl<'a> Transfer<'a> {
    pub(super) fn new(
        source: &'a Model,
        face: usize,
        wire: usize,
        slot: usize,
    ) -> Result<Option<Self>> {
        let c = &source.loops[wire].coedges[slot];
        let Some(original) = c.trim() else {
            return Ok(None);
        };
        Ok(Some(Self {
            original,
            source: &source.edges[c.edge].curve,
            surface: &source.faces[face].surface,
            reversed: c.reversed,
            face,
            wire,
            slot,
            fragments: Vec::new(),
        }))
    }
    pub(super) fn piece(
        &mut self,
        edge: usize,
        piece: &partition::Piece,
        pc: &Curve,
        tol: f64,
        limits: Limits,
        work: &mut Work,
    ) -> Result<Option<CoedgeTrim>> {
        let sd = self.source.domain();
        let world = &piece.curve;
        let cd = world.domain();
        let pd = pc.domain();
        let a = q(self.original.curve_parameter[0]);
        let b = q(self.original.curve_parameter[1]);
        let delta = &b - &a;
        // Physical source clock, then exact preimage of the child's world domain.
        let physical = |g: Q| {
            if self.reversed {
                q(sd[0]) + q(sd[1]) - g
            } else {
                g
            }
        };
        let x0 = physical(a.clone());
        let x1 = physical(b.clone());
        let t0 = (q(cd[0]) - &x0) / (&x1 - &x0);
        let t1 = (q(cd[1]) - &x0) / (&x1 - &x0);
        let lo = t0.clone().min(t1.clone()).max(Q::zero());
        let hi = t0.max(t1).min(Q::one());
        if lo >= hi {
            return Ok(None);
        }
        work.remaining(limits)?;
        work.cells += 1;
        let g = [&a + &delta * &lo, &a + &delta * &hi];
        let local = g.clone().map(|v| {
            if self.reversed {
                v + q(cd[0]) + q(cd[1]) - q(sd[0]) - q(sd[1])
            } else {
                v
            }
        });
        let nominal = local
            .clone()
            .map(|v| q(pd[0]) + (q(pd[1]) - q(pd[0])) * (v - q(cd[0])) / (q(cd[1]) - q(cd[0])));
        let curve = [rounded(&local[0], cd)?, rounded(&local[1], cd)?];
        let pcurve = [rounded(&nominal[0], pd)?, rounded(&nominal[1], pd)?];
        let original_clock = [rounded(&g[0], sd)?, rounded(&g[1], sd)?];
        let drift = upper(&g[0] - q(original_clock[0]))?
            .max(upper(&g[1] - q(original_clock[1]))?)
            .max(upper(&local[0] - q(curve[0]))?)
            .max(upper(&local[1] - q(curve[1]))?);
        let (clock_error, cells) = lift::parameter_clock_error_bound(
            self.source,
            drift,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += cells;
        let clock_error =
            clock_error.ok_or_else(|| error("Original fragment clock error bound exhausted"))?;
        let partition = piece
            .agreement
            .as_ref()
            .filter(|r| r.agreement_proven)
            .ok_or_else(|| error("Original child partition evidence is missing"))?;
        let mut partition_upper = 0f64;
        for cell in &partition.cells {
            partition_upper = partition_upper.max(
                cell.error_upper
                    .ok_or_else(|| error("Original child curve bound is missing"))?,
            );
        }
        let original_curve_distance_upper =
            I::point(partition_upper).add(I::point(clock_error))?.hi;
        if original_curve_distance_upper > tol {
            return Err(error(
                "Original fragment versus child world curve exceeds allowance",
            ));
        }
        let allowance = I::point(tol).sub(I::point(clock_error))?.lo;
        if allowance <= 0. {
            return Err(error(
                "Original fragment clock conversion exceeds world tolerance",
            ));
        }
        let nodes = std::array::from_fn::<_, 2, _>(|i| [original_clock[i], pcurve[i]]);
        let proof = lift::verify_parameter_correspondence(
            self.source,
            pc,
            self.surface,
            &nodes,
            self.reversed,
            allowance,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += proof.cells;
        if proof.status != lift::Status::WithinTolerance {
            return Err(error(
                "Original authored fragment versus actual child lift remains unproven",
            ));
        }
        let transferred = CoedgeTrim {
            curve_parameter: curve,
            pcurve_parameter: pcurve,
            periodic_lift: crate::boundary_agreement::periodic_lifts(self.surface, pc, pcurve)?,
        };
        transferred.validate()?;
        let nodes = std::array::from_fn::<_, 2, _>(|i| [curve[i], pcurve[i]]);
        let actual = lift::verify_parameter_correspondence(
            world,
            pc,
            self.surface,
            &nodes,
            self.reversed,
            tol,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += actual.cells;
        if actual.status != lift::Status::WithinTolerance {
            return Err(error("Stored child fragment lift remains unproven"));
        }
        let upper = I::point(
            proof
                .distance_upper
                .ok_or_else(|| error("Original fragment bound is missing"))?,
        )
        .add(I::point(clock_error))?
        .hi;
        if upper > tol {
            return Err(error("Original fragment total allowance exceeded"));
        }
        self.fragments.push(RelationFragment {
            relation_range: [lo, hi],
            edge,
            transferred,
            original_distance_upper: upper,
            original_curve_distance_upper,
        });
        Ok(Some(transferred))
    }
    pub(super) fn finish(mut self) -> Result<AuthoredRelation> {
        self.fragments
            .sort_by(|a, b| a.relation_range[0].cmp(&b.relation_range[0]));
        if self.fragments.is_empty()
            || self.fragments[0].relation_range[0] != Q::zero()
            || self.fragments.last().unwrap().relation_range[1] != Q::one()
            || self
                .fragments
                .windows(2)
                .any(|p| p[0].relation_range[1] != p[1].relation_range[0])
        {
            return Err(error(
                "Original authored relation has an uncovered fragment or overlap",
            ));
        }
        Ok(AuthoredRelation {
            face: self.face,
            wire: self.wire,
            coedge: self.slot,
            original: self.original,
            fragments: self.fragments,
        })
    }
}
