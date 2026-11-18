//! Two natural charts with a common projection partition a Jordan region along
//! an exactly lifted edge chain. Only the joint exact-domain audit may call
//! this, after all coedge lifts and domains have been independently checked.
use crate::{Model, boundary_hull_contact::Certificate};
use cad_predicates::Sign;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use nurbs_core::{
    curve::Curve,
    editing::exact_curve_merge::clamp_exact_curve,
    editing::exact_knot_insertion,
    editing::exact_projective::affine_curve_clock,
    surface_injectivity::{certify_chart, certify_jordan_projection},
};

#[derive(Clone, Copy, PartialEq)]
enum Projection {
    Coordinate([usize; 2]),
    Linear([[f64; 3]; 2]),
}

impl Projection {
    fn rows(self) -> [[i8; 3]; 2] {
        match self {
            Self::Coordinate(axes) => axes.map(|axis| {
                let mut row = [0; 3];
                row[axis] = 1;
                row
            }),
            // The chart proof proposes integer rows in -64..=64.
            Self::Linear(rows) => rows.map(|row| row.map(|value| value as i8)),
        }
    }

    fn point(self, point: &[f64]) -> Option<[f64; 2]> {
        match self {
            Self::Coordinate(axes) => Some(axes.map(|axis| point[axis])),
            Self::Linear(rows) => {
                let dot = |row: [f64; 3]| {
                    let mut exact = BigRational::from_integer(0.into());
                    for (coefficient, &value) in row.into_iter().zip(point) {
                        exact +=
                            BigRational::from_float(coefficient)? * BigRational::from_float(value)?;
                    }
                    let stored = exact.to_f64()?;
                    (BigRational::from_float(stored).as_ref() == Some(&exact)).then_some(stored)
                };
                Some([dot(rows[0])?, dot(rows[1])?])
            }
        }
    }
}

pub(crate) fn certify(
    model: &Model,
    faces: [usize; 2],
    limit: usize,
) -> (Option<Certificate>, usize) {
    let mut cells = 0;
    let mut ordered = faces;
    ordered.sort_unstable();
    let result = prove(model, ordered, limit, &mut cells).map(|mut certificate| {
        certificate.faces = faces;
        certificate
    });
    (result, cells)
}

fn charge(cells: &mut usize, limit: usize, cost: usize) -> Option<()> {
    if cost > limit - *cells {
        return None;
    }
    *cells += cost;
    Some(())
}

fn prove(model: &Model, faces: [usize; 2], limit: usize, cells: &mut usize) -> Option<Certificate> {
    let patches = faces.map(|i| &model.faces[i]);
    let surfaces = patches.map(|f| &f.surface);
    if !surfaces.iter().any(|s| {
        s.control_points.len() > s.degree_u + 1 || s.control_points[0].len() > s.degree_v + 1
    }) {
        return None;
    }
    let wires = patches.map(|f| model.loops[f.outer].coedges.as_slice());
    if wires.iter().any(|w| !(4..=128).contains(&w.len())) {
        return None;
    }
    let (starts, edges) = shared_chain(wires, cells, limit)?;
    for patch in patches {
        interior_holes(model, &patch.surface, &patch.holes, cells, limit)?;
    }
    let mut proposals = [None; 2];
    for ((s, wire), proposal) in surfaces.into_iter().zip(wires).zip(&mut proposals) {
        natural_wire(s, wire, cells, limit)?;

        let remaining = limit - *cells;
        if remaining == 0 {
            return None;
        }
        let proof = certify_chart(s, remaining).ok()?;
        charge(cells, limit, proof.spans)?;
        *proposal = match (proof.proven, proof.reason) {
            (
                true,
                "global-rational-jordan-jacobian" | "global-rational-multispan-jordan-jacobian",
            ) => proof.projection.map(Projection::Coordinate),
            (true, "global-exact-oblique-jordan-jacobian") => {
                proof.linear_projection.map(Projection::Linear)
            }
            _ => None,
        };
    }
    let mut candidates = proposals;
    if candidates[0] == candidates[1] {
        candidates[1] = None;
    }
    for projection in candidates.into_iter().flatten() {
        if common_orientation(surfaces, proposals, projection, cells, limit).is_some()
            && projected_exterior(model, wires, starts, edges.len(), projection, cells, limit)
                .is_some()
        {
            return edge_certificate(model, faces, edges, cells, limit);
        }
    }
    None
}

fn interior_holes(
    model: &Model,
    surface: &crate::Surface,
    holes: &[usize],
    cells: &mut usize,
    limit: usize,
) -> Option<()> {
    let lower = [
        surface.knots_u[surface.degree_u],
        surface.knots_v[surface.degree_v],
    ];
    let upper = [
        surface.knots_u[surface.control_points.len()],
        surface.knots_v[surface.control_points[0].len()],
    ];
    for &hole in holes {
        charge(cells, limit, 1)?;
        for use_ in &model.loops[hole].coedges {
            interior_curve(&use_.pcurve, [lower, upper], 12, cells, limit)?;
        }
    }
    Some(())
}

fn interior_curve(
    curve: &Curve,
    [lower, upper]: [[f64; 2]; 2],
    depth: u8,
    cells: &mut usize,
    limit: usize,
) -> Option<()> {
    charge(cells, limit, curve.control_points.len())?;
    // Positive weights place each exact restriction in its control hull.
    let inside = |p: &[f64]| (0..2).all(|axis| lower[axis] < p[axis] && p[axis] < upper[axis]);
    if curve.control_points.iter().all(|p| inside(p)) {
        return Some(());
    }
    let remaining = ((limit - *cells) as u64).min(cad_predicates::MAX_WORK);
    if depth == 0 || remaining == 0 {
        return None;
    }
    let (proved, used) =
        exact_knot_insertion::strict_box_containment(curve, [lower, upper], depth, remaining)
            .ok()?;
    charge(cells, limit, used as usize)?;
    proved.then_some(())
}

fn shared_chain(
    wires: [&[crate::Coedge]; 2],
    cells: &mut usize,
    limit: usize,
) -> Option<([usize; 2], Vec<usize>)> {
    charge(cells, limit, (wires[0].len() * wires[1].len()).div_ceil(4))?;
    let common = [0, 1].map(|side| {
        wires[side]
            .iter()
            .map(|c| wires[1 - side].iter().any(|d| c.edge == d.edge))
            .collect::<Vec<_>>()
    });
    let run = |flags: &[bool]| {
        let count = flags.iter().filter(|&&shared| shared).count();
        let start =
            (0..flags.len()).find(|&i| flags[i] && !flags[(i + flags.len() - 1) % flags.len()])?;
        (0..count)
            .all(|offset| flags[(start + offset) % flags.len()])
            .then_some((start, count))
    };
    let (a, count) = run(&common[0])?;
    let (b, other_count) = run(&common[1])?;
    if count != other_count {
        return None;
    }
    let mut edges = Vec::with_capacity(count);
    for offset in 0..count {
        let left = &wires[0][(a + offset) % wires[0].len()];
        let right = &wires[1][(b + count - 1 - offset) % wires[1].len()];
        if left.edge != right.edge
            || left.reversed == right.reversed
            || left.trim().is_some()
            || right.trim().is_some()
            || edges.contains(&left.edge)
        {
            return None;
        }
        edges.push(left.edge);
    }
    Some(([a, b], edges))
}

fn natural_wire(
    s: &crate::Surface,
    wire: &[crate::Coedge],
    cells: &mut usize,
    limit: usize,
) -> Option<()> {
    charge(cells, limit, wire.len())?;
    let [[u0, u1], [v0, v1]] = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let corners = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
    let segments = wire
        .iter()
        .map(|c| natural_segment(c, cells, limit))
        .collect::<Option<Vec<_>>>()?;
    let first = segments.iter().position(|p| p[0] == corners[0])?;
    let mut side = 0;
    let mut current = corners[0];
    for offset in 0..wire.len() {
        corners.get(side)?;
        let [start, point] = segments[(first + offset) % wire.len()];
        if start != current {
            return None;
        }
        let end = corners[(side + 1) % 4];
        let axis = side % 2;
        let progress = if side < 2 {
            current[axis] < point[axis] && point[axis] <= end[axis]
        } else {
            current[axis] > point[axis] && point[axis] >= end[axis]
        };
        if !progress || point[1 - axis] != end[1 - axis] {
            return None;
        }
        current = point;
        side += usize::from(current == end);
    }
    (side == 4).then_some(())
}

fn exact_endpoint(value: &BigRational) -> Option<f64> {
    let stored = value.to_f64()?;
    (BigRational::from_float(stored).as_ref() == Some(value)).then_some(stored)
}

fn natural_segment(c: &crate::Coedge, cells: &mut usize, limit: usize) -> Option<[[f64; 2]; 2]> {
    let p = &c.pcurve;
    if p.periodic
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || p.knots != [p.knots[0], p.knots[0], p.knots[3], p.knots[3]]
    {
        return None;
    }
    let point = |i: usize| [p.control_points[i][0], p.control_points[i][1]];
    let Some(trim) = c.trim() else {
        return Some([point(0), point(1)]);
    };
    if trim.periodic_lift != [[0; 2]; 2] {
        return None;
    }
    charge(cells, limit, 1)?;
    let [lo, hi] = p.domain().map(BigRational::from_float);
    let (lo, hi) = (lo?, hi?);
    let evaluate = |t: f64| {
        let fraction = (BigRational::from_float(t)? - &lo) / (&hi - &lo);
        let at = |axis: usize| {
            let a = BigRational::from_float(point(0)[axis])?;
            let b = BigRational::from_float(point(1)[axis])?;
            exact_endpoint(&(&a + &fraction * (b - &a)))
        };
        Some([at(0)?, at(1)?])
    };
    let [a, b] = trim.pcurve_parameter;
    Some([evaluate(a)?, evaluate(b)?])
}

fn common_orientation(
    surfaces: [&crate::Surface; 2],
    proposals: [Option<Projection>; 2],
    projection: Projection,
    cells: &mut usize,
    limit: usize,
) -> Option<()> {
    let mut orientation = None;
    for (s, proposed) in surfaces.into_iter().zip(proposals) {
        if proposed != Some(projection) {
            let (certified, used) =
                certify_jordan_projection(s, projection.rows(), limit - *cells).ok()?;
            charge(cells, limit, used)?;
            if !certified {
                return None;
            }
        }
        charge(cells, limit, 1)?;
        // Strict rank on the original chart makes the corner sign valid
        // throughout its projected image.
        let points = [
            &s.control_points[0][0],
            &s.control_points[1][0],
            &s.control_points[0][1],
        ];
        let [a, b, c] = points.map(|p| projection.point(p));
        let [[ax, ay], [bx, by], [cx, cy]] = [a?, b?, c?];
        let sign = super::orient(&[&[ax, ay, 0.], &[bx, by, 0.], &[cx, cy, 0.]], Some([0, 1]))?;
        if sign == Sign::Zero || orientation.is_some_and(|p| p != sign) {
            return None;
        }
        orientation = Some(sign);
    }
    Some(())
}

fn projected_exterior(
    model: &Model,
    wires: [&[crate::Coedge]; 2],
    starts: [usize; 2],
    shared_count: usize,
    projection: Projection,
    cells: &mut usize,
    limit: usize,
) -> Option<()> {
    let count = wires[0].len() + wires[1].len() - 2 * shared_count;
    let mut exterior = Vec::with_capacity(count);
    for (wire, start) in wires.into_iter().zip(starts) {
        for offset in shared_count..wire.len() {
            charge(cells, limit, 1)?;
            let c = &wire[(start + offset) % wire.len()];
            let source = &model.edges[c.edge].curve;
            // A partial world interval is a diagnostic window and does
            // not establish the full topology edge used by this proof.
            if c.trim()
                .is_some_and(|t| t.curve_parameter != source.domain())
            {
                return None;
            }
            exterior.push(projected_member(
                source, c.reversed, projection, cells, limit,
            )?);
        }
    }

    let pairs = count * (count - 1) / 2;
    charge(cells, limit, pairs)?;
    let remaining = (limit - *cells).min(64);
    if remaining == 0 {
        return None;
    }
    let boundary = nurbs_core::trim_simplicity::inspect(&exterior, 1e-8, pairs, remaining).ok()?;
    charge(cells, limit, boundary.cells)?;
    if !boundary.proven_simple {
        return None;
    }
    // Off the boundaries, the projected winding numbers have the same sign.
    // Their sum is the winding of the simple exterior (the shared edge cancels),
    // so the two interiors cannot overlap. Nor can either interior contain the
    // other's boundary by openness and density. Simplicity of the exterior and
    // the two original loops leaves only the authored seam as boundary contact.
    Some(())
}

fn projected_member(
    source: &Curve,
    reversed: bool,
    projection: Projection,
    cells: &mut usize,
    limit: usize,
) -> Option<Curve> {
    let d = source.degree;
    let count = source.control_points.len();
    if source.periodic || !(1..=5).contains(&d) {
        return None;
    }
    let clamped = source.knots[..=d].iter().all(|k| *k == source.knots[d])
        && source.knots[count..]
            .iter()
            .all(|k| *k == source.knots[count]);
    let single_span = clamped && count == d + 1;
    if !single_span {
        charge(cells, limit, source.knots.len())?;
    }
    if !single_span || matches!(projection, Projection::Linear(_)) {
        charge(cells, limit, count)?;
    }
    let control_points = source
        .control_points
        .iter()
        .map(|p| projection.point(p).map(|point| point.to_vec()))
        .collect::<Option<Vec<_>>>()?;
    let mut curve = Curve {
        degree: d,
        knots: if single_span {
            [vec![0.; d + 1], vec![1.; d + 1]].concat()
        } else {
            source.knots.clone()
        },
        control_points,
        weights: source.weights.clone(),
        periodic: false,
    };
    if !clamped {
        let remaining = ((limit - *cells) as u64).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return None;
        }
        let report = clamp_exact_curve(&curve, remaining).ok()?;
        charge(cells, limit, report.work_used as usize)?;
        curve = report.curve?;
    }
    if reversed {
        if single_span {
            curve.control_points.reverse();
            curve.weights.reverse();
        } else {
            let remaining = ((limit - *cells) as u64).min(cad_predicates::MAX_WORK);
            if remaining == 0 {
                return None;
            }
            let [start, end] = curve.domain();
            let report = affine_curve_clock(&curve, [end, start], remaining).ok()?;
            charge(cells, limit, report.work_used as usize)?;
            curve = report.geometry?;
        }
    }
    Some(curve)
}

fn edge_certificate(
    model: &Model,
    faces: [usize; 2],
    edges: Vec<usize>,
    cells: &mut usize,
    limit: usize,
) -> Option<Certificate> {
    let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for pole in edges
        .iter()
        .flat_map(|&edge| &model.edges[edge].curve.control_points)
    {
        charge(cells, limit, 1)?;
        for (interval, &value) in enclosure.iter_mut().zip(pole) {
            interval[0] = interval[0].min(value);
            interval[1] = interval[1].max(value);
        }
    }
    Some(Certificate {
        faces,
        contact_enclosure: enclosure,
        edges,
        vertex: None,
        joined_proof: None,
        polar_join: None,
        source_join: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_period_proves_strict_hole_containment_through_the_wrap() {
        let mut curve = Curve {
            degree: 2,
            knots: (-2..=6).map(f64::from).collect(),
            control_points: vec![
                vec![-0.046875, 0.25],
                vec![0.375, 0.5],
                vec![0.75, 0.75],
                vec![0.375, 0.75],
                vec![-0.046875, 0.25],
                vec![0.375, 0.5],
            ],
            weights: vec![2., 1., 1., 1., 2., 1.],
            periodic: true,
        };
        let bounds = [[0., 0.], [1., 1.]];
        let mut used = 0;
        assert!(interior_curve(&curve, bounds, 12, &mut used, 1000).is_some());
        assert!(interior_curve(&curve, bounds, 12, &mut 0, used - 1).is_none());
        let mut shifted = curve.clone();
        shifted.knots[0] = shifted.knots[0].next_up();
        shifted.validate().unwrap();
        assert!(interior_curve(&shifted, bounds, 12, &mut 0, 1000).is_none());
        // On the final span the minimum distance becomes exactly zero.
        curve.control_points[0][0] = -0.0625;
        curve.control_points[4][0] = -0.0625;
        assert!(interior_curve(&curve, bounds, 12, &mut 0, 1000).is_none());
    }

    #[test]
    fn exact_active_interval_proves_strict_hole_containment_for_unclamped_trims() {
        let mut curve = Curve {
            degree: 2,
            knots: vec![-1., -0.5, 0., 0.5, 1., 1.5, 2.],
            control_points: vec![
                vec![-0.0625, 0.25],
                vec![0.25, 0.375],
                vec![0.5, 0.5],
                vec![0.75, 0.625],
            ],
            weights: vec![2., 1., 1., 1.],
            periodic: false,
        };
        let bounds = [[0., 0.], [1., 1.]];
        let mut used = 0;
        assert!(interior_curve(&curve, bounds, 12, &mut used, 1000).is_some());
        assert!(interior_curve(&curve, bounds, 12, &mut 0, used - 1).is_none());
        curve.control_points.reverse();
        curve.weights.reverse();
        assert!(interior_curve(&curve, bounds, 12, &mut 0, 1000).is_some());
        // The weighted average at the active end is now exactly zero.
        curve.control_points[3][0] = -0.125;
        assert!(interior_curve(&curve, bounds, 12, &mut 0, 1000).is_none());
    }

    #[test]
    fn exact_span_extraction_proves_strict_hole_containment_at_a_nonbinary_join() {
        let mut curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0.25, 0.25],
                vec![-0.046875, 0.375],
                vec![0.25, 0.5],
                vec![0.25, 0.75],
            ],
            weights: vec![1., 2., 1., 1.],
            periodic: false,
        };
        assert!(
            exact_knot_insertion::split(&curve, 0.5, 100_000)
                .unwrap()
                .pieces
                .is_none()
        );
        let bounds = [[0., 0.], [1., 1.]];
        let mut used = 0;
        assert!(interior_curve(&curve, bounds, 12, &mut used, 1000).is_some());
        assert!(interior_curve(&curve, bounds, 12, &mut 0, used - 1).is_none());
        // The first span now has an exact zero at an interior parameter.
        curve.control_points[1][0] = -0.0625;
        assert!(interior_curve(&curve, bounds, 12, &mut 0, 100_000).is_none());
    }

    #[test]
    fn exact_subdivision_proves_strict_hole_containment_when_the_hull_overhangs() {
        let mut curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.25, 0.25], vec![-0.0625, 0.375], vec![0.25, 0.5]],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let bounds = [[0., 0.], [1., 1.]];
        let split = exact_knot_insertion::split(&curve, 0.5, 100_000).unwrap();
        assert!(
            split.pieces.is_none(),
            "the midpoint pole has a nonbinary coordinate"
        );
        let mut used = 0;
        assert!(interior_curve(&curve, bounds, 12, &mut used, 1000).is_some());
        assert!(interior_curve(&curve, bounds, 12, &mut 0, used - 1).is_none());
        // This change puts the exact midpoint on the outer boundary.
        curve.control_points[1][0] = -0.125;
        assert!(interior_curve(&curve, bounds, 12, &mut 0, 100_000).is_none());
    }
}
