use super::*;
use crate::Curve;
use crate::profile_certificate::{
    AlgebraicEventPosition, SpanIntersectionCertificate, certify_point_winding,
    certify_span_intersections, own_curve_event_nodes, verify_span_intersections,
};
use num_bigint::BigInt;
use nurbs_core::curve_partition_agreement::exact_bezier_span_partition;
use std::collections::BTreeMap;
mod contact;

type SharedRange = ([[Q; 2]; 2], bool);

// Candidate recovery only. The isolator's uniqueness and exact polynomial
// substitution establish identity, independently of the approximation method.
fn recover_rational_root(polynomial: &[Q], lo: &Q, hi: &Q) -> Option<Q> {
    let mut midpoint = (lo + hi) / Q::from_integer(2.into());
    let mut p = [BigInt::from(0), BigInt::from(1)];
    let mut q = [BigInt::from(1), BigInt::from(0)];
    for _ in 0..64 {
        let integer = midpoint.floor().to_integer();
        p = [p[1].clone(), &integer * &p[1] + &p[0]];
        q = [q[1].clone(), &integer * &q[1] + &q[0]];
        let candidate = Q::new(p[1].clone(), q[1].clone());
        if lo < &candidate
            && &candidate < hi
            && polynomial
                .iter()
                .rev()
                .fold(Q::zero(), |value, coefficient| {
                    value * &candidate + coefficient
                })
                .is_zero()
        {
            return Some(candidate);
        }
        let remainder = midpoint - Q::from_integer(integer);
        if remainder.is_zero() {
            break;
        }
        midpoint = remainder.recip();
    }
    None
}

fn isolated_rational_root(polynomial: &[Q], lo: Q, hi: Q) -> Option<Q> {
    if lo == hi {
        return Some(lo);
    }
    let contains = |root: &Q| lo < *root && *root < hi;
    match polynomial {
        [constant, linear] if !linear.is_zero() => Some(-constant / linear).filter(contains),
        [constant, linear, quadratic] if !quadratic.is_zero() => {
            let two = Q::from_integer(2.into());
            let discriminant = linear * linear - &two * &two * quadratic * constant;
            if discriminant.is_negative() {
                return None;
            }
            let numerator = discriminant.numer().sqrt();
            let denominator = discriminant.denom().sqrt();
            if &numerator * &numerator != *discriminant.numer()
                || &denominator * &denominator != *discriminant.denom()
            {
                return None;
            }
            let radical = Q::new(numerator, denominator);
            let divisor = two * quadratic;
            let mut roots = [
                (-linear - &radical) / &divisor,
                (-linear + radical) / &divisor,
            ];
            roots.sort();
            if roots[0] == roots[1] {
                return Some(roots[0].clone()).filter(contains);
            }
            let mut inside = roots.into_iter().filter(contains);
            let root = inside.next()?;
            inside.next().is_none().then_some(root)
        }
        _ if polynomial.len() > 3 => recover_rational_root(polynomial, &lo, &hi),
        _ => None,
    }
}

fn exact_event_parameters(event: &AlgebraicEventPosition) -> Result<[Q; 2]> {
    let ratio = |value: &[String; 2]| Q::new(value[0].parse().unwrap(), value[1].parse().unwrap());
    let [lo, hi] = event.isolating_interval.each_ref().map(ratio);
    let polynomial: Vec<_> = event.primitive.iter().map(ratio).collect();
    let root = isolated_rational_root(&polynomial, lo, hi);
    let read = |polynomial: &Vec<[String; 2]>| {
        if let Some(root) = &root {
            return Ok(polynomial
                .iter()
                .rev()
                .fold(Q::zero(), |sum, c| sum * root + ratio(c)));
        }
        match polynomial.as_slice() {
            [] => Ok(Q::zero()),
            [constant] => Ok(ratio(constant)),
            _ => Err(unsupported(
                "Curved event has no exactly representable clock",
            )),
        }
    };
    Ok([read(&event.parameters[0])?, read(&event.parameters[1])?])
}

fn verified_shared_ranges(
    a: &Curve,
    b: &Curve,
    census: &SpanIntersectionCertificate,
) -> Result<Vec<SharedRange>> {
    let owner = own_curve_event_nodes(a, b, census)?;
    owner
        .overlap_endpoints()?
        .into_iter()
        .map(|(_, endpoints)| {
            let [lo, hi] = endpoints.map(|end| exact_event_parameters(&end.exact));
            let [lo, hi] = [lo?, hi?];
            if lo.iter().zip(&hi).any(|(a, b)| a == b) {
                return Err(unsupported("Common branch has a collapsed endpoint clock"));
            }
            Ok((
                [0, 1].map(|side| {
                    [
                        (&lo[side]).min(&hi[side]).clone(),
                        (&lo[side]).max(&hi[side]).clone(),
                    ]
                }),
                (lo[0] < hi[0]) == (lo[1] < hi[1]),
            ))
        })
        .collect()
}

/// Lift each active pcurve into the common affine chart.
fn map_wire(wire: &Loop, source: &AffineChart, target: &AffineChart) -> Result<Vec<Curve>> {
    let mut mapped = Vec::new();
    for coedge in &wire.coedges {
        if coedge
            .trim()
            .is_some_and(|trim| trim.periodic_lift != [[0; 2]; 2])
        {
            return Err(unsupported(
                "Affine curved overlap requires zero periodic lift",
            ));
        }
        for mut curve in crate::profile::pcurve_trim::active_pcurves(coedge)? {
            let [lo, hi] = curve.domain();
            if curve.periodic
                || curve.knots[..=curve.degree].iter().any(|&v| v != lo)
                || curve.knots[curve.control_points.len()..]
                    .iter()
                    .any(|&v| v != hi)
            {
                return Err(unsupported("Curved overlap needs clamped active pcurves"));
            }
            for point in &mut curve.control_points {
                let exact = |i: usize| {
                    Q::from_float(point[i]).ok_or_else(|| invalid("Nonfinite pcurve pole"))
                };
                let world = source.lift(&[exact(0)?, exact(1)?]);
                let [u, v] = target.inverse(&world)?;
                *point = vec![representable(&u)?, representable(&v)?];
            }
            mapped.push(curve);
        }
    }
    Ok(mapped)
}

fn parametric_cells(curve: &Curve) -> usize {
    curve
        .knots
        .windows(2)
        .filter(|knots| knots[0] < knots[1])
        .count()
}

/// Exact identity of whole clamped Bernstein curves under an affine clock
/// change. The boolean reports whether their traversals agree.
fn shared_span(a: &Curve, b: &Curve) -> Option<bool> {
    if a.degree != b.degree
        || a.control_points.len() != a.degree + 1
        || b.control_points.len() != b.degree + 1
    {
        return None;
    }
    [true, false].into_iter().find(|&forward| {
        let index = |i| if forward { i } else { b.degree - i };
        let weight = |curve: &Curve, i: usize| Q::from_float(curve.weights[i]).unwrap();
        let (wa, wb) = (weight(a, 0), weight(b, index(0)));
        (0..=a.degree).all(|i| {
            let j = index(i);
            a.control_points[i] == b.control_points[j] && weight(a, i) * &wb == weight(b, j) * &wa
        })
    })
}

fn shared_lines(a: &Curve, b: &Curve) -> Result<Option<SharedRange>> {
    if [a, b]
        .iter()
        .any(|c| c.degree != 1 || c.control_points.len() != 2 || c.weights[0] != c.weights[1])
    {
        return Ok(None);
    }
    let exact =
        |c: &Curve| [0, 1].map(|i| [0, 1].map(|k| Q::from_float(c.control_points[i][k]).unwrap()));
    let [p, q] = exact(a);
    let [r, s] = exact(b);
    let delta: [Q; 2] = std::array::from_fn(|i| &q[i] - &p[i]);
    let Some(axis) = delta.iter().position(|v| !v.is_zero()) else {
        return Ok(None);
    };
    let on_line = |x: &[Q; 2]| &delta[0] * (&x[1] - &p[1]) == &delta[1] * (&x[0] - &p[0]);
    if !on_line(&r) || !on_line(&s) || r == s {
        return Ok(None);
    }
    let low = p[axis]
        .clone()
        .min(q[axis].clone())
        .max(r[axis].clone().min(s[axis].clone()));
    let high = p[axis]
        .clone()
        .max(q[axis].clone())
        .min(r[axis].clone().max(s[axis].clone()));
    if low >= high {
        return Ok(None);
    }
    let clock = |curve: &Curve, start: &Q, end: &Q| {
        let [lo, hi] = curve.domain().map(|v| Q::from_float(v).unwrap());
        let param = |point: &Q| &lo + (point - start) * (&hi - &lo) / (end - start);
        let mut interval = [param(&low), param(&high)];
        interval.sort();
        interval
    };
    Ok(Some((
        [clock(a, &p[axis], &q[axis]), clock(b, &r[axis], &s[axis])],
        (p[axis] < q[axis]) == (r[axis] < s[axis]),
    )))
}

pub(crate) fn reverse_exact(mut curve: Curve) -> Result<Curve> {
    let [a, b] = curve.domain().map(|v| Q::from_float(v).unwrap());
    let sum = a + b;
    for knot in &mut curve.knots {
        *knot = representable(&(&sum - Q::from_float(*knot).unwrap()))?;
    }
    curve.knots.reverse();
    curve.control_points.reverse();
    curve.weights.reverse();
    curve.validate()?;
    Ok(curve)
}

#[derive(Clone, Copy)]
struct RetainedWire {
    side: usize,
    wire: usize,
}

fn copy_patch(
    models: [&Model; 2],
    faces: [usize; 2],
    source: [&SourceFace; 2],
    outer: RetainedWire,
    holes: &[RetainedWire],
) -> Result<Model> {
    let owner = outer.side;
    let reversed = source[owner].reversed;
    let mut patch = Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            faces: vec![],
            loops: vec![],
            shells: vec![Shell {
                faces: vec![FaceUse { face: 0, reversed }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: models[0].tolerance_mm.min(models[1].tolerance_mm),
        },
        TopologyIds::default(),
    );
    let mut f = models[owner].faces[faces[owner]].clone();
    let mut vertices = BTreeMap::new();
    let mut edges = BTreeMap::new();
    for retained in std::iter::once(&outer).chain(holes) {
        let side = retained.side;
        let model = models[side];
        let mut wire = model.loops[retained.wire].clone();
        if side != owner {
            let mapped = map_wire(&wire, &source[side].chart, &source[owner].chart)?;
            for (coedge, pcurve) in wire.coedges.iter_mut().zip(mapped) {
                if source[side].reversed == reversed {
                    coedge.pcurve = pcurve;
                } else {
                    coedge.pcurve = reverse_exact(pcurve)?;
                    coedge.reversed = !coedge.reversed;
                }
            }
            if source[side].reversed != reversed {
                wire.coedges.reverse();
            }
        }
        for coedge in &mut wire.coedges {
            coedge.edge = *edges.entry((side, coedge.edge)).or_insert_with(|| {
                let mut edge = model.edges[coedge.edge].clone();
                edge.vertices = edge.vertices.map(|v| {
                    *vertices.entry((side, v)).or_insert_with(|| {
                        let id = patch.vertices.len();
                        patch.vertices.push(model.vertices[v].clone());
                        id
                    })
                });
                let id = patch.edges.len();
                patch.edges.push(edge);
                id
            });
        }
        patch.loops.push(wire);
    }
    f.outer = 0;
    f.holes = (1..patch.loops.len()).collect();
    patch.faces.push(f);
    patch.rebuild_topology_ids();
    patch.validate()?;
    Ok(patch)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlanarOperation {
    Intersection,
    Difference,
}

fn contact_census(
    wires: [&[Vec<Curve>]; 2],
    max_tests: usize,
    operation: PlanarOperation,
) -> Result<([Vec<contact::CurveCuts>; 2], bool, usize)> {
    let mut cuts = wires.map(|loops| vec![Vec::new(); loops.iter().map(Vec::len).sum()]);
    let mut shared = cuts.each_ref().map(|cuts| vec![Vec::new(); cuts.len()]);
    let mut has_contacts = false;
    let mut remaining = max_tests;
    for (i, first) in wires[0].iter().flatten().enumerate() {
        for (j, second) in wires[1].iter().flatten().enumerate() {
            let count = parametric_cells(first).saturating_mul(parametric_cells(second));
            remaining = remaining.checked_sub(count).ok_or_else(|| {
                crate::Error::new(
                    "BREP_RESOURCE_LIMIT",
                    "Curved overlap exceeded its span-pair budget",
                )
            })?;
            let overlap = match shared_span(first, second) {
                Some(forward) => Some((
                    [first, second].map(|c| c.domain().map(|v| Q::from_float(v).unwrap())),
                    forward,
                )),
                None => shared_lines(first, second)?,
            };
            if let Some(([a, b], forward)) = overlap {
                shared[0][i].push((
                    a.clone(),
                    forward == (operation == PlanarOperation::Intersection),
                ));
                shared[1][j].push((b.clone(), false));
                cuts[0][i].extend(a);
                cuts[1][j].extend(b);
                has_contacts = true;
                continue;
            }
            let certificate = certify_span_intersections(first, second, false, 1e-8)?;
            let verified = verify_span_intersections(first, second, &certificate)?;
            if !verified.algebraic_loci().is_empty()
                || !verified.singular_parameters().is_empty()
                || !verified.stationary_spans().is_empty()
            {
                return Err(unsupported(
                    "Curved overlap has an unresolved singularity or parameter locus",
                ));
            }
            if !verified.overlaps().is_empty() {
                let ranges = verified_shared_ranges(first, second, &certificate)?;
                if ranges.len() != verified.overlaps().len() {
                    return Err(unsupported("Common branch endpoint census is incomplete"));
                }
                for ([a, b], forward) in ranges {
                    shared[0][i].push((
                        a.clone(),
                        forward == (operation == PlanarOperation::Intersection),
                    ));
                    shared[1][j].push((b.clone(), false));
                    cuts[0][i].extend(a);
                    cuts[1][j].extend(b);
                    has_contacts = true;
                }
            }
            if verified
                .events()
                .iter()
                .any(|event| event.iter().any(|[lo, hi]| lo != hi))
            {
                let owner = own_curve_event_nodes(first, second, &certificate)?;
                for node in owner.nodes() {
                    let [a, b] = exact_event_parameters(&node.exact)?;
                    cuts[0][i].push(a);
                    cuts[1][j].push(b);
                }
            } else {
                for &[[a, _], [b, _]] in verified.events() {
                    cuts[0][i].push(Q::from_float(a).unwrap());
                    cuts[1][j].push(Q::from_float(b).unwrap());
                }
            }
            has_contacts |= !verified.events().is_empty();
        }
    }
    let cuts = [0, 1].map(|side| {
        std::mem::take(&mut cuts[side])
            .into_iter()
            .zip(std::mem::take(&mut shared[side]))
            .collect()
    });
    Ok((cuts, has_contacts, remaining))
}

pub(crate) fn split_pcurve_on_line(curve: &Curve, line: &Curve) -> Result<[Vec<Curve>; 2]> {
    let census = certify_span_intersections(curve, line, false, 1e-8)?;
    let verified = verify_span_intersections(curve, line, &census)?;
    if !verified.overlaps().is_empty()
        || !verified.algebraic_loci().is_empty()
        || !verified.singular_parameters().is_empty()
        || !verified.stationary_spans().is_empty()
    {
        return Err(unsupported("Chart seam has an unresolved pcurve contact"));
    }
    let owner = own_curve_event_nodes(curve, line, &census)?;
    let [node] = owner.nodes() else {
        return Err(unsupported(
            "Chart seam needs one exact interior pcurve contact",
        ));
    };
    let [clock, _] = exact_event_parameters(&node.exact)?;
    let [a, b] = curve.domain().map(|v| Q::from_float(v).unwrap());
    if clock <= a || clock >= b {
        return Err(unsupported(
            "Chart seam clock is outside the pcurve interior",
        ));
    }
    use crate::profile::pcurve_trim::restrict_pcurve;
    Ok([
        restrict_pcurve(curve, [a, clock.clone()])?,
        restrict_pcurve(curve, [clock, b])?,
    ])
}

pub(crate) fn intersect_planar_pcurves(
    wires: [&[Vec<Curve>]; 2],
    max_tests: usize,
) -> Result<Vec<Vec<Curve>>> {
    let (cuts, _, _) = contact_census(wires, max_tests, PlanarOperation::Intersection)?;
    contact::trace_cycles(wires, cuts, PlanarOperation::Intersection)
}

pub(crate) fn planar_boundaries_are_disjoint(
    wires: [&[Vec<Curve>]; 2],
    max_tests: usize,
) -> Result<bool> {
    let (_, contacts, _) = contact_census(wires, max_tests, PlanarOperation::Intersection)?;
    Ok(!contacts)
}

pub(crate) fn subtract_planar_pcurves(
    wires: [&[Vec<Curve>]; 2],
    max_tests: usize,
) -> Result<Vec<Vec<Curve>>> {
    let (cuts, _, _) = contact_census(wires, max_tests, PlanarOperation::Difference)?;
    contact::trace_cycles(wires, cuts, PlanarOperation::Difference)
}

pub(super) fn strict_intersection(
    models: [&Model; 2],
    faces: [usize; 2],
    source: [&SourceFace; 2],
    max_tests: usize,
) -> Result<Vec<Model>> {
    let wire_ids = [0, 1].map(|side| {
        let f = &models[side].faces[faces[side]];
        std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .collect::<Vec<_>>()
    });
    let [first, second] = [0, 1].map(|side| {
        wire_ids[side]
            .iter()
            .map(|&id| {
                let mapped = map_wire(
                    &models[side].loops[id],
                    &source[side].chart,
                    &source[0].chart,
                )?;
                if source[side].reversed != source[0].reversed {
                    mapped.into_iter().rev().map(reverse_exact).collect()
                } else {
                    Ok(mapped)
                }
            })
            .collect::<Result<Vec<_>>>()
    });
    let partition = |wires: Vec<Vec<Curve>>| -> Result<Vec<Vec<Curve>>> {
        wires
            .into_iter()
            .map(|wire| {
                let mut spans = Vec::new();
                for curve in wire {
                    if let Some(parts) = exact_bezier_span_partition(&curve)? {
                        spans.extend(parts);
                    } else {
                        // Preserve exact contact census for general B-splines.
                        spans.push(curve);
                    }
                }
                Ok(spans)
            })
            .collect()
    };
    let wires = [partition(first?)?, partition(second?)?];
    // When the source uses restricted pcurves, build the output from the
    // active mapped fragments even if no cross-boundary event is found.
    let uses_trim = wire_ids.iter().enumerate().any(|(side, wires)| {
        wires.iter().any(|&i| {
            models[side].loops[i]
                .coedges
                .iter()
                .any(|c| c.trim().is_some())
        })
    });
    let (cuts, has_contacts, remaining) = contact_census(
        [&wires[0], &wires[1]],
        max_tests,
        PlanarOperation::Intersection,
    )?;
    if has_contacts || uses_trim {
        return contact::intersection(
            [&wires[0], &wires[1]],
            cuts,
            &source[0].chart,
            &models[0].faces[faces[0]].surface,
            source[0].reversed,
            models[0].tolerance_mm.min(models[1].tolerance_mm),
            remaining,
        );
    }
    // With no boundary contacts, each source loop lies wholly inside or
    // outside the other face. Intersection keeps exactly the inside loops.
    let mut outers = Vec::new();
    let mut holes = Vec::new();
    for side in [0, 1] {
        for (index, wire) in wires[side].iter().enumerate() {
            let point = [wire[0].control_points[0][0], wire[0].control_points[0][1]];
            let winding: i64 = certify_point_winding(&wires[1 - side], point)?
                .windings
                .iter()
                .sum();
            if winding == 0 {
                continue;
            }
            let retained = RetainedWire {
                side,
                wire: wire_ids[side][index],
            };
            if index == 0 {
                outers.push(retained);
            } else {
                holes.push(retained);
            }
        }
    }
    match outers.as_slice() {
        [] if holes.is_empty() => Ok(Vec::new()),
        [outer] => Ok(vec![copy_patch(models, faces, source, *outer, &holes)?]),
        _ => Err(unsupported(
            "Curved overlap has inconsistent retained loop roles",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_event_primitive_recovers_exact_clocks_from_a_wide_interval() {
        let q = |n: i32, d: i32| [n.to_string(), d.to_string()];
        let mut event = AlgebraicEventPosition {
            primitive: vec![q(-1, 1), q(2, 1)],
            isolating_interval: [q(1, 4), q(3, 4)],
            parameters: [vec![q(0, 1), q(1, 1)], vec![q(1, 1), q(-1, 2)]],
            position: std::array::from_fn(|_| std::array::from_fn(|_| Vec::new())),
        };
        assert_eq!(
            exact_event_parameters(&event).unwrap(),
            [Q::new(1.into(), 2.into()), Q::new(3.into(), 4.into())]
        );
        event.primitive[1] = q(3, 1);
        assert_eq!(
            exact_event_parameters(&event).unwrap()[0],
            Q::new(1.into(), 3.into())
        );
    }

    #[test]
    fn exact_clock_recovery_uses_the_verified_curve_intersection() {
        let x = 2_f64.powi(-30);
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = crate::line(vec![x, 0.], vec![x, 2.]);
        let census = certify_span_intersections(&a, &b, false, 1e-8).unwrap();
        let verified = verify_span_intersections(&a, &b, &census).unwrap();
        assert!(verified.events()[0].iter().any(|[lo, hi]| lo < hi));
        let owner = own_curve_event_nodes(&a, &b, &census).unwrap();
        assert_eq!(owner.nodes().len(), 1);
        assert_eq!(
            exact_event_parameters(&owner.nodes()[0].exact).unwrap(),
            [x, x * x / 2.].map(|v| Q::from_float(v).unwrap()),
        );
    }

    #[test]
    fn quadratic_event_primitive_recovers_both_nondyadic_contacts() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 2.], vec![1.5, -2.5], vec![3., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = crate::line(vec![0., 0.], vec![3., 0.]);
        let census = certify_span_intersections(&a, &b, false, 1e-8).unwrap();
        let owner = own_curve_event_nodes(&a, &b, &census).unwrap();
        assert_eq!(owner.nodes().len(), 2);
        assert!(
            owner
                .nodes()
                .iter()
                .all(|node| node.exact.primitive.len() == 3)
        );
        let mut exact: Vec<_> = owner
            .nodes()
            .iter()
            .map(|node| exact_event_parameters(&node.exact).unwrap())
            .collect();
        exact.sort();
        assert_eq!(
            exact,
            [1, 2].map(|n| std::array::from_fn(|_| Q::new(n.into(), 3.into())))
        );
    }

    #[test]
    fn cubic_event_recovers_all_three_exact_curve_contacts() {
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., -6.], vec![1., 7.], vec![2., -7.], vec![3., 6.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let b = crate::line(vec![0., 0.], vec![3., 0.]);
        let census = certify_span_intersections(&a, &b, false, 1e-8).unwrap();
        let owner = own_curve_event_nodes(&a, &b, &census).unwrap();
        assert_eq!(owner.nodes().len(), 3);
        assert!(
            owner
                .nodes()
                .iter()
                .any(|node| node.exact.primitive.len() == 4
                    && node.exact.isolating_interval[0] != node.exact.isolating_interval[1])
        );
        let mut exact: Vec<_> = owner
            .nodes()
            .iter()
            .map(|node| exact_event_parameters(&node.exact).unwrap())
            .collect();
        exact.sort();
        assert_eq!(
            exact,
            [(1, 3), (1, 2), (2, 3)]
                .map(|(n, d)| { std::array::from_fn(|_| Q::new(n.into(), d.into())) })
        );
    }

    #[test]
    fn partial_quadratic_overlap_has_exact_clock_ranges() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 0.], vec![2., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut b = a.trim(0.5, 1.).unwrap();
        b = reverse_exact(b).unwrap();
        let census = certify_span_intersections(&a, &b, false, 1e-8).unwrap();
        let ranges = verified_shared_ranges(&a, &b, &census).unwrap();
        assert_eq!(
            ranges,
            vec![(
                [[0.5, 1.], [0.5, 1.]].map(|range| range.map(|v| Q::from_float(v).unwrap())),
                false
            )]
        );
    }
}
