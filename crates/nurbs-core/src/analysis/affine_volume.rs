//! Global filled-carrier atlas for coefficient-owned polynomial/rational poses.
//! F(u,v,t)=T(t)+A(t)(Q(u,v)-origin). UV trims, wall/carrier binding, original-law
//! approximation, shell orientation and G1/G2 are separate owner obligations.
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3 as iv;
use crate::sweeps::progressive_sweep::ideal_volume::{
    IdealVolumeReport, atlas,
    cell::{Enclosed, Enclosure},
    charts,
    pose::Pose as EnclosedPose,
};
use crate::{Result, check, surface::Surface};
use std::collections::BTreeMap;
/// Three matrix columns followed by translation; all values are literal source
/// coefficients, not interval samples or caller-supplied successful proofs.
pub type Pose = [[f64; 3]; 4];
mod rational;
pub use rational::{
    ColumnFactor, PreconditionedReport, ProofCoordinates, RationalPiece, certify_rational,
    certify_rational_preconditioned, jacobian_orientation_rational,
};
#[derive(Clone, Copy)]
enum Motion<'a> {
    Quintic(&'a [[Pose; 6]]),
    Rational(&'a [RationalPiece]),
}
impl<'a> Motion<'a> {
    fn len(self) -> usize {
        match self {
            Self::Quintic(p) => p.len(),
            Self::Rational(p) => p.len(),
        }
    }
    fn valid(self) -> bool {
        match self {
            Self::Quintic(p) => p
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1e9),
            Self::Rational(p) => p.iter().all(RationalPiece::valid),
        }
    }
    fn endpoints(self, i: usize) -> (&'a Pose, &'a Pose) {
        match self {
            Self::Quintic(p) => (&p[i][0], &p[i][5]),
            Self::Rational(p) => (&p[i].controls[0], p[i].controls.last().unwrap()),
        }
    }
    fn enclose(self, i: usize, t: I) -> Result<EnclosedPose> {
        match self {
            Self::Quintic(p) => pose_on(&p[i], t, p.len()),
            Self::Rational(p) => rational::pose_on(&p[i], t, p.len()),
        }
    }
}
type Box3 = [[f64; 2]; 3];
fn bezier(mut a: Vec<I>, t: I) -> Result<I> {
    let hull = I::new(
        a.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        a.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max),
    )?;
    let one = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..a.len()).rev() {
        for i in 0..n {
            let x = a[i];
            let y = a[i + 1];
            let direct = x.mul(one)?.add(y.mul(t)?)?;
            let left = x.add(y.sub(x)?.mul(t)?)?;
            let right = y.add(x.sub(y)?.mul(one)?)?;
            a[i] = I::new(
                direct.lo.max(left.lo).max(right.lo),
                direct.hi.min(left.hi).min(right.hi),
            )?;
        }
    }
    a[0].intersect(hull.lo, hull.hi)
}
fn scalar(values: [f64; 6], t: I, count: usize) -> Result<[I; 2]> {
    let d = values
        .windows(2)
        .map(|p| {
            if p[0] == p[1] {
                Ok(I::point(0.))
            } else {
                I::point(p[1])
                    .sub(I::point(p[0]))?
                    .mul(I::point(5. * count as f64))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok([bezier(values.map(I::point).to_vec(), t)?, bezier(d, t)?])
}
fn pose_on(p: &[Pose; 6], t: I, count: usize) -> Result<EnclosedPose> {
    let mut values = [[[I::point(0.); 3]; 2]; 4];
    for col in 0..4 {
        for row in 0..3 {
            let jet = scalar(p.map(|p| p[col][row]), t, count)?;
            for order in 0..2 {
                values[col][order][row] = jet[order];
            }
        }
    }
    Ok(EnclosedPose {
        origin: values[3],
        linear: [values[0], values[1], values[2]],
    })
}
fn union(a: &mut EnclosedPose, b: EnclosedPose) -> Result<()> {
    for order in 0..2 {
        for k in 0..3 {
            a.origin[order][k] = I::new(
                a.origin[order][k].lo.min(b.origin[order][k].lo),
                a.origin[order][k].hi.max(b.origin[order][k].hi),
            )?;
            for col in 0..3 {
                a.linear[col][order][k] = I::new(
                    a.linear[col][order][k].lo.min(b.linear[col][order][k].lo),
                    a.linear[col][order][k].hi.max(b.linear[col][order][k].hi),
                )?;
            }
        }
    }
    Ok(())
}
struct Context<'a> {
    carrier: &'a Surface,
    origin: [f64; 3],
    pieces: Motion<'a>,
    poses: BTreeMap<[u64; 2], EnclosedPose>,
    coordinates: Option<ProofCoordinates>,
}
impl Context<'_> {
    fn temporal(
        &mut self,
        t: [f64; 2],
        out: &mut IdealVolumeReport,
        max: usize,
    ) -> Result<Option<EnclosedPose>> {
        let key = t.map(f64::to_bits);
        if let Some(p) = self.poses.get(&key) {
            return Ok(Some(*p));
        }
        let count = self.pieces.len();
        let scaled = I::new(t[0], t[1])?.mul(I::point(count as f64))?;
        // Outward scaled bounds include both sides of every exact i/count knot.
        // Never select pieces using a rounded f64 value of i/count.
        let first = (scaled.lo.floor().max(0.) as usize)
            .saturating_sub(1)
            .min(count - 1);
        let last = (scaled.hi.ceil().max(0.) as usize).min(count - 1);
        let mut pose: Option<EnclosedPose> = None;
        for i in first..=last {
            if out.cells == max {
                return Ok(None);
            }
            out.cells += 1;
            let local = scaled.sub(I::point(i as f64))?;
            if local.hi < 0. || local.lo > 1. {
                continue;
            }
            let local = local.intersect(0., 1.)?;
            let value = match (self.pieces, self.coordinates.as_ref()) {
                (Motion::Rational(p), Some(c)) => {
                    rational::pose_in_coordinates(&p[i], local, count, c)?
                }
                _ => self.pieces.enclose(i, local)?,
            };
            if let Some(p) = &mut pose {
                union(p, value)?;
            } else {
                pose = Some(value);
            }
        }
        if let Some(p) = pose {
            self.poses.insert(key, p);
        }
        Ok(pose)
    }
    fn enclose(&mut self, b: Box3, out: &mut IdealVolumeReport, max: usize) -> Result<Enclosed> {
        let max = max.min(out.cells.saturating_add(100000));
        if out.cells == max {
            return Ok(Enclosed::Unavailable(None));
        }
        out.cells += 1;
        out.boxes += 1;
        let Some(mut q) = charts::cap_jets(self.carrier, [b[0], b[1]], &mut out.cells, max)? else {
            return Ok(Enclosed::Unavailable(None));
        };
        // Strong rational weights can make a wide carrier jet box ambiguous
        // before time enters the problem. Refine its original UV chart first;
        // the turning-source time heuristic must not endlessly split only t.
        // This is a subdivision proposal, never a regularity/admission claim.
        let normal = iv::cross(q[1], q[2])?;
        if !normal.iter().any(|x| x.lo > 0. || x.hi < 0.) {
            let axis = usize::from(b[1][1] - b[1][0] > b[0][1] - b[0][0]);
            return Ok(Enclosed::Unavailable(Some(axis)));
        }
        q[0] = iv::sub(q[0], self.origin.map(I::point))?;
        if let Some(f) = self
            .coordinates
            .as_ref()
            .and_then(|c| c.column_factor.as_ref())
        {
            for jet in &mut q {
                jet[f.base] = jet[f.base].add(jet[f.dependent].mul(I::point(f.scale))?)?;
            }
        }
        let Some(pose) = self.temporal(b[2], out, max)? else {
            return Ok(Enclosed::Unavailable(Some(2)));
        };
        let e: Enclosure = pose.enclose(q, [I::point(0.); 3])?;
        Ok(Enclosed::Value(e))
    }
}
/// Exhaustive atlas of the ACTUAL affine filled volume, over the normalized
/// carrier UV rectangle and normalized time. Every original time piece and
/// one-sided derivative is enclosed; every distinct parameter pair is covered.
/// Closed time is a proved S1 quotient only after literal full-pose closure.
/// No valid UV material region, BRep binding, original-law error or G2 is inferred.
/// Geometry cells include every query, original carrier rectangle and attempted
/// temporal piece. Exact work includes every scalar station/closure comparison.
pub fn certify(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: &[[Pose; 6]],
    closed: bool,
    projection: [[f64; 3]; 3],
    max_cells: usize,
    max_exact_work: u64,
) -> Result<IdealVolumeReport> {
    certify_motion(
        carrier,
        origin,
        Motion::Quintic(pieces),
        closed,
        projection,
        max_cells,
        max_exact_work,
    )
}
fn certify_motion(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: Motion<'_>,
    closed: bool,
    projection: [[f64; 3]; 3],
    max_cells: usize,
    max_exact_work: u64,
) -> Result<IdealVolumeReport> {
    certify_motion_impl(
        carrier,
        origin,
        pieces,
        closed,
        projection,
        max_cells,
        max_exact_work,
        false,
        &mut None,
    )
}
fn certify_motion_impl(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: Motion<'_>,
    closed: bool,
    projection: [[f64; 3]; 3],
    max_cells: usize,
    max_exact_work: u64,
    precondition: bool,
    coordinates: &mut Option<ProofCoordinates>,
) -> Result<IdealVolumeReport> {
    carrier.validate()?;
    check(
        carrier
            .control_points
            .iter()
            .flatten()
            .all(|p| p.len() == 3)
            && (1..=1024).contains(&pieces.len())
            && max_cells <= 1000000
            && max_exact_work <= 1000000
            && origin
                .iter()
                .chain(projection.iter().flatten())
                .all(|x| x.is_finite() && x.abs() <= 1e9)
            && pieces.valid(),
        "Invalid affine filled-volume inputs or budgets",
    )?;
    let mut out = IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: 0,
        exact_work: 0,
        boxes: 0,
        monotonicity_lower: None,
        projection,
        reason: Some("affine-volume-work-limit"),
    };
    if !charts::surface_c0(carrier) {
        out.reason = Some("affine-volume-carrier-basis-unproved");
        return Ok(out);
    }
    let pairs = (0..pieces.len() - 1)
        .map(|i| (pieces.endpoints(i).1, pieces.endpoints(i + 1).0))
        .chain(closed.then(|| (pieces.endpoints(pieces.len() - 1).1, pieces.endpoints(0).0)));
    for (a, b) in pairs {
        for col in 0..4 {
            for k in 0..3 {
                if out.exact_work == max_exact_work {
                    out.reason = Some("affine-volume-station-identity-work-limit");
                    return Ok(out);
                }
                out.exact_work += 1;
                if a[col][k] != b[col][k] {
                    out.reason = Some("affine-volume-station-identity-unproved");
                    return Ok(out);
                }
            }
        }
    }
    let mut context = Context {
        carrier,
        origin,
        pieces,
        poses: BTreeMap::new(),
        coordinates: None,
    };
    if precondition && out.cells < max_cells {
        // A single native proposal from ORIGINAL point jets. Its invertibility
        // is independently proved; no point regularity supplies atlas credit.
        if let Enclosed::Value(e) =
            context.enclose([[0.5; 2], [0.5; 2], [0.125; 2]], &mut out, max_cells)?
        {
            if out.cells < max_cells {
                out.cells += 1;
                // Literal coefficient directions retain the global radial and
                // carrier axes better than an arbitrary rotated point basis.
                // This average is ONLY an inverse proposal; the atlas still
                // evaluates every original coefficient with outward rounding.
                let proposal = match pieces {
                    Motion::Rational(p) => {
                        let a = p[0].controls[0];
                        let b = p[p.len() / 2].controls[0];
                        let d = p[0].controls[1];
                        [
                            a[0],
                            std::array::from_fn(|k| a[1][k] * 0.5 + b[1][k] * 0.5),
                            std::array::from_fn(|k| d[0][k] - a[0][k]),
                        ]
                    }
                    _ => [[0.; 3]; 3],
                };
                if let Some(p) = atlas::inverse_proposal(proposal.map(|v| v.map(I::point)))
                    .or_else(|| atlas::inverse_proposal(e.columns))
                {
                    let determinant = iv::dot_tight(
                        p[0].map(I::point),
                        iv::cross(p[1].map(I::point), p[2].map(I::point))?,
                    )?;
                    if determinant.lo > 0. || determinant.hi < 0. {
                        let factor = match pieces {
                            Motion::Rational(p) => {
                                match rational::prove_factor(p, &mut out.exact_work, max_exact_work)
                                {
                                    Ok(f) => f,
                                    Err(()) => {
                                        out.reason = Some("affine-volume-column-factor-work-limit");
                                        return Ok(out);
                                    }
                                }
                            }
                            _ => None,
                        };
                        let c = ProofCoordinates {
                            rows: p,
                            offset: pieces.endpoints(0).0[3],
                            determinant: [determinant.lo, determinant.hi],
                            column_factor: factor,
                        };
                        context.poses.clear();
                        context.coordinates = Some(c.clone());
                        *coordinates = Some(c);
                    }
                }
            }
        }
    }
    atlas::certify_map(
        |b, out, max| context.enclose(b, out, max),
        projection,
        &mut out,
        max_cells,
        closed,
    )?;
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct OrientationReport {
    /// Sign of the actual world Jacobian at this owned parameter point. Its
    /// GLOBAL orientation meaning requires a separately proved volume atlas.
    pub sign: Option<i8>,
    pub determinant: Option<[f64; 2]>,
    pub cells: usize,
    pub reason: &'static str,
}
pub fn jacobian_orientation(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: &[[Pose; 6]],
    point: [f64; 3],
    max_cells: usize,
) -> Result<OrientationReport> {
    orientation_motion(carrier, origin, Motion::Quintic(pieces), point, max_cells)
}
fn orientation_motion(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: Motion<'_>,
    point: [f64; 3],
    max_cells: usize,
) -> Result<OrientationReport> {
    // Reuse complete raw-input validation; a zero-budget certificate performs
    // no geometry queries and supplies no atlas/closure credit to this helper.
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let mut work = certify_motion(carrier, origin, pieces, false, projection, 0, 0)?;
    check(
        max_cells <= 100000
            && point
                .iter()
                .all(|x| x.is_finite() && (0. ..=1.).contains(x)),
        "Invalid affine Jacobian orientation point or budget",
    )?;
    let mut out = OrientationReport {
        sign: None,
        determinant: None,
        cells: 0,
        reason: "affine-orientation-unproved",
    };
    if !charts::surface_c0(carrier) {
        return Ok(out);
    }
    let mut context = Context {
        carrier,
        origin,
        pieces,
        poses: BTreeMap::new(),
        coordinates: None,
    };
    if let Enclosed::Value(e) = context.enclose(point.map(|x| [x, x]), &mut work, max_cells)? {
        let d = iv::dot_tight(e.columns[0], iv::cross(e.columns[1], e.columns[2])?)?;
        out.determinant = Some([d.lo, d.hi]);
        out.sign = if d.lo > 0. {
            Some(1)
        } else if d.hi < 0. {
            Some(-1)
        } else {
            None
        };
        if out.sign.is_some() {
            out.reason = "owned-world-jacobian-sign";
        }
    }
    out.cells = work.cells;
    Ok(out)
}

#[cfg(test)]
mod tests;
