//! Winding via convex homotopies of complete original UV arc enclosures.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
type Box2 = [[f64; 2]; 2];
#[derive(Clone, Debug)]
pub struct QualifiedRootLoopWinding {
    source: Arc<RootBrep>,
    face: usize,
    loop_in_face: usize,
    query: Box2,
    limits: ss::OriginalSsWindingLimits,
    winding: i32,
    segments: usize,
    lifted: bool,
}
pub(super) fn outward(q: &Q, lower: bool) -> Result<f64> {
    let x = q
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| unproved("UV clock overflow"))?;
    let exact = Q::from_float(x).unwrap();
    Ok(if lower && exact > *q {
        x.next_down()
    } else if !lower && exact < *q {
        x.next_up()
    } else {
        x
    })
}
fn charge(r: &mut ss::OriginalSsCurveLimits, b: &ss::OriginalSsCurveBounds) -> Result<()> {
    r.native_rectangles = r
        .native_rectangles
        .checked_sub(b.native_rectangles)
        .ok_or_else(|| unproved("UV rectangle budget exhausted"))?;
    r.interval_operations = r
        .interval_operations
        .checked_sub(b.interval_operations)
        .ok_or_else(|| unproved("UV interval budget exhausted"))?;
    r.projection.max_work = r
        .projection
        .max_work
        .checked_sub(b.projection_work)
        .ok_or_else(|| unproved("UV projection budget exhausted"))?;
    r.world_rectangles = r
        .world_rectangles
        .checked_sub(b.world_rectangles)
        .ok_or_else(|| unproved("UV world budget exhausted"))?;
    Ok(())
}
pub(super) fn project(
    curve: &ss::OriginalSsRootCurve,
    d: [f64; 2],
    support: usize,
    r: &mut ss::OriginalSsCurveLimits,
) -> Result<Box2> {
    let b = curve.enclose(d, *r)?;
    charge(r, &b)?;
    Ok([b.uv[2 * support], b.uv[2 * support + 1]])
}
fn endpoint(
    port: &ss::QualifiedOriginalSsPort,
    support: usize,
    r: &mut ss::OriginalSsCurveLimits,
) -> Result<Box2> {
    let p = port.parameter_bounds();
    let d = port.target().parameter_domain();
    project(
        port.target(),
        [
            outward(&p[0], true)?.max(d[0]),
            outward(&p[1], false)?.min(d[1]),
        ],
        support,
        r,
    )
}
pub(super) fn translate(bounds: Box2, shift: &[Q; 2]) -> Result<Box2> {
    let mut translated = bounds;
    for (axis, offset) in shift.iter().enumerate() {
        if offset.is_zero() {
            continue;
        }
        for side in 0..2 {
            let native = Q::from_float(bounds[axis][side])
                .ok_or_else(|| unproved("Nonfinite native UV enclosure"))?;
            translated[axis][side] = outward(&(native + offset), side == 0)?;
        }
    }
    Ok(translated)
}

pub(super) fn face_periods(surface: &Surface) -> [Q; 2] {
    let period = |knots: &[f64], degree: usize, count: usize| {
        Q::from_float(knots[count]).unwrap() - Q::from_float(knots[degree]).unwrap()
    };
    [
        period(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        ),
        period(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ),
    ]
}

fn sample(b: Box2) -> [f64; 2] {
    b.map(|d| (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1]))
}
fn sign(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> Q {
    let q = |v| Q::from_float(v).unwrap();
    (q(b[0]) - q(a[0])) * (q(p[1]) - q(a[1])) - (q(b[1]) - q(a[1])) * (q(p[0]) - q(a[0]))
}
pub(super) fn lifted_loop_bounds(
    source: &RootBrep,
    face: usize,
    id: usize,
    remaining: &mut ss::OriginalSsCurveLimits,
) -> Result<Box2> {
    let topology = source.topology();
    let periods = face_periods(&topology.faces[face].surface);
    let lift = super::uv_closure::qualify_loop_lift(source, face, id, false)?;
    if lift.winding != [0, 0] || lift.coedge_offsets.is_empty() {
        return Err(unproved(
            "Hole placement requires a nonempty closed UV lift",
        ));
    }
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for (coedge, offset) in topology.loops[id].coedges.iter().zip(lift.coedge_offsets) {
        let edge = &topology.edges[coedge.edge].curve;
        let enclosure = edge.enclose(*remaining)?;
        charge(remaining, &enclosure)?;
        let uv = [
            enclosure.uv[2 * coedge.pcurve.support],
            enclosure.uv[2 * coedge.pcurve.support + 1],
        ];
        let shift =
            std::array::from_fn(|axis| &periods[axis] * Q::from_integer(offset[axis].into()));
        let uv = translate(uv, &shift)?;
        for axis in 0..2 {
            bounds[axis][0] = bounds[axis][0].min(uv[axis][0]);
            bounds[axis][1] = bounds[axis][1].max(uv[axis][1]);
        }
    }
    Ok(bounds)
}

fn joint_enclosures(
    topology: &RootTopology,
    uses: &[brep_topology::Coedge<RootPcurve>],
    shifts: &[[Q; 2]],
    remaining: &mut ss::OriginalSsCurveLimits,
) -> Result<Vec<Box2>> {
    let mut starts = Vec::new();
    // A single box/sample at each joint makes the temporary polygon
    // connected even when adjacent SS pairs have different UV enclosures.
    for i in 0..uses.len() {
        let a = &uses[(i + uses.len() - 1) % uses.len()];
        let b = &uses[i];
        let ea = &topology.edges[a.edge].curve;
        let eb = &topology.edges[b.edge].curve;
        let end = if a.reversed { ea.start() } else { ea.end() };
        let start = if b.reversed { eb.end() } else { eb.start() };
        let previous = (i + uses.len() - 1) % uses.len();
        let x = translate(
            endpoint(end, a.pcurve.support, remaining)?,
            &shifts[previous],
        )?;
        let y = translate(endpoint(start, b.pcurve.support, remaining)?, &shifts[i])?;
        let joint: Box2 = std::array::from_fn(|k| [x[k][0].max(y[k][0]), x[k][1].min(y[k][1])]);
        if joint.iter().any(|d| d[0] > d[1]) {
            return Err(unproved("Proved UV joint enclosures are inconsistent"));
        }
        starts.push(joint);
    }
    Ok(starts)
}

impl QualifiedRootLoopWinding {
    pub fn winding(&self) -> i32 {
        self.winding
    }
    pub fn segments(&self) -> usize {
        self.segments
    }
    pub fn qualify(
        source: Arc<RootBrep>,
        face: usize,
        loop_in_face: usize,
        query: Box2,
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        Self::qualify_impl(source, face, loop_in_face, query, limits, false)
    }

    /// Compute winding in the universal cover. The query uses the lift whose
    /// first coedge has offset zero. Nonzero period winding has no bounded
    /// planar interior for an individual loop.
    pub fn qualify_lifted(
        source: Arc<RootBrep>,
        face: usize,
        loop_in_face: usize,
        query: Box2,
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        Self::qualify_impl(source, face, loop_in_face, query, limits, true)
    }

    fn qualify_impl(
        source: Arc<RootBrep>,
        face: usize,
        loop_in_face: usize,
        query: Box2,
        limits: ss::OriginalSsWindingLimits,
        lifted: bool,
    ) -> Result<Self> {
        if !query
            .iter()
            .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1])
            || !(1..=4096).contains(&limits.max_segments)
        {
            return Err(unproved("Invalid mixed-loop winding query or budget"));
        }
        let t = source.topology();
        let f = t
            .faces
            .get(face)
            .ok_or_else(|| unproved("Missing winding face"))?;
        let id = if loop_in_face == 0 {
            f.outer
        } else {
            *f.holes
                .get(loop_in_face - 1)
                .ok_or_else(|| unproved("Missing winding loop"))?
        };
        let uses = &t.loops[id].coedges;
        if uses.is_empty() {
            return Err(unproved("Empty winding contour"));
        }
        if uses.len() > limits.max_segments {
            return Err(unproved("Mixed-loop winding subdivision exhausted"));
        }
        let lift = super::uv_closure::qualify_loop_lift(&source, face, id, !lifted)?;
        if lift.winding != [0, 0] {
            return Err(unproved("Lifted contour has nonzero period winding"));
        }
        let periods = face_periods(&f.surface);
        let shifts: Vec<[Q; 2]> = if lifted {
            lift.coedge_offsets
                .iter()
                .map(|offset| {
                    std::array::from_fn(|axis| {
                        &periods[axis] * Q::from_integer(offset[axis].into())
                    })
                })
                .collect()
        } else {
            vec![[Q::zero(), Q::zero()]; uses.len()]
        };
        let mut remaining = limits.geometry;
        let starts = joint_enclosures(t, uses, &shifts, &mut remaining)?;
        let point = sample(query);
        let mut winding = 0;
        let mut segments = 0;
        for (i, c) in uses.iter().enumerate() {
            let e = &t.edges[c.edge].curve;
            let (a, b) = if c.reversed {
                (e.end(), e.start())
            } else {
                (e.start(), e.end())
            };
            let forward = e.forward() != c.reversed;
            let (lo, hi) = if forward {
                (&a.parameter_bounds()[1], &b.parameter_bounds()[0])
            } else {
                (&b.parameter_bounds()[1], &a.parameter_bounds()[0])
            };
            let p = e.parameter_bounds();
            let natural = e.curve().parameter_domain();
            let low = outward(&p[0], true)?.max(natural[0]);
            let high = outward(&p[1], false)?.min(natural[1]);
            let (c0, c1) = if forward { (low, high) } else { (high, low) };
            let mut pending = vec![(
                c0,
                c1,
                starts[i],
                starts[(i + 1) % uses.len()],
                lo.clone(),
                hi.clone(),
            )];
            while let Some((c0, c1, start, end, lo, hi)) = pending.pop() {
                if segments == limits.max_segments {
                    return Err(unproved("Mixed-loop winding subdivision exhausted"));
                }
                segments += 1;
                let arc = translate(
                    project(
                        e.curve(),
                        [c0.min(c1), c0.max(c1)],
                        c.pcurve.support,
                        &mut remaining,
                    )?,
                    &shifts[i],
                )?;
                let hull: Box2 = std::array::from_fn(|k| {
                    [
                        arc[k][0].min(start[k][0]).min(end[k][0]),
                        arc[k][1].max(start[k][1]).max(end[k][1]),
                    ]
                });
                if !(0..2).any(|k| query[k][1] < hull[k][0] || hull[k][1] < query[k][0]) {
                    let q = (&lo + &hi) / Q::from_integer(2.into());
                    let x = outward(&q, true)?;
                    let xq = Q::from_float(x).unwrap();
                    if !(lo < xq && xq < hi && c0.min(c1) < x && x < c0.max(c1)) {
                        return Err(unproved("Mixed winding has no strict interior clock"));
                    }
                    let mid = translate(
                        project(e.curve(), [x; 2], c.pcurve.support, &mut remaining)?,
                        &shifts[i],
                    )?;
                    let (left, right) = if forward {
                        ((lo, xq.clone()), (xq, hi))
                    } else {
                        ((xq.clone(), hi), (lo, xq))
                    };
                    pending.push((x, c1, mid, end, right.0, right.1));
                    pending.push((c0, x, start, mid, left.0, left.1));
                    continue;
                }
                let a = sample(start);
                let b = sample(end);
                let s = sign(a, b, point);
                if a[1] <= point[1] && point[1] < b[1] && s > Q::zero() {
                    winding += 1;
                }
                if b[1] <= point[1] && point[1] < a[1] && s < Q::zero() {
                    winding -= 1;
                }
            }
        }
        Ok(Self {
            source,
            face,
            loop_in_face,
            query,
            limits,
            winding,
            segments,
            lifted,
        })
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh = Self::qualify_impl(
            self.source.clone(),
            self.face,
            self.loop_in_face,
            self.query,
            self.limits,
            self.lifted,
        )?;
        if self.winding != fresh.winding || self.segments != fresh.segments {
            return Err(unproved("Mixed winding proof changed"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_translation_encloses_nonrepresentable_period_multiple() {
        let third = Q::new(1.into(), 3.into());
        let translated = translate([[1., 1.], [2., 3.]], &[third.clone(), -third.clone()]).unwrap();
        let exact = [
            Q::from_integer(1.into()) + &third,
            Q::from_integer(2.into()) - &third,
        ];
        for axis in 0..2 {
            assert!(Q::from_float(translated[axis][0]).unwrap() <= exact[axis]);
        }
        assert!(Q::from_float(translated[0][1]).unwrap() >= exact[0]);
        assert!(Q::from_float(translated[1][1]).unwrap() >= Q::from_integer(3.into()) - third);
    }
}
