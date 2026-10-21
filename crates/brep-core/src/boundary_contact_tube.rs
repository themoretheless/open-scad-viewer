//! Original-control proof that every pair contact stays in an owned-edge tube.
//! Tolerance-qualified scope only: never upgrades exact embedding/volume flags.
use crate::{Coedge, Model};
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
use nurbs_core::{Error, Result, curve::Curve, surface_g2_pcurves};
pub struct Certificate {
    pub plane_face: usize,
    pub sided_face: usize,
    pub edges: Vec<usize>,
    pub boundary_error_upper: f64,
    pub boundary_axis: usize,
    pub boundary_end: usize,
}
pub struct Attempt {
    pub certificate: Option<Certificate>,
    pub cells: usize,
    pub exact_work: u64,
}
fn strict_trace(pc: &Curve, axis: usize, value: f64, end: usize) -> bool {
    let positive = |p: &Vec<f64>| {
        if end == 0 {
            p[axis] > value
        } else {
            p[axis] < value
        }
    };
    let n = pc.control_points.len();
    let d = pc.domain();
    if pc.periodic
        || !pc.knots[..=pc.degree].iter().all(|&x| x == d[0])
        || !pc.knots[n..].iter().all(|&x| x == d[1])
    {
        return false;
    }
    if n == pc.degree + 1 {
        pc.control_points.iter().any(positive)
    } else {
        n > 2 && pc.control_points[1..n - 1].iter().all(positive)
    }
}
fn covers<'a>(
    model: &'a Model,
    face: usize,
    axis: usize,
    end: usize,
    value: f64,
) -> Option<Vec<&'a Coedge>> {
    let f = &model.faces[face];
    let s = &f.surface;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut edges = Vec::new();
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        let cs = &model.loops[wire].coedges;
        let mut on = vec![false; cs.len()];
        for (i, c) in cs.iter().enumerate() {
            let pc = &c.pcurve;
            if pc
                .control_points
                .iter()
                .any(|p| (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]))
            {
                return None;
            }
            on[i] = pc.control_points.iter().all(|p| p[axis] == value);
            if on[i] {
                edges.push(c);
            } else if !strict_trace(pc, axis, value, end) {
                return None;
            }
        }
        for (i, c) in cs.iter().enumerate() {
            if !on[i] {
                if c.pcurve.control_points[0][axis] == value && !on[(i + cs.len() - 1) % cs.len()] {
                    return None;
                }
                if c.pcurve.control_points.last().unwrap()[axis] == value && !on[(i + 1) % cs.len()]
                {
                    return None;
                }
            }
        }
    }
    if edges.is_empty() { None } else { Some(edges) }
}
fn ordered(
    model: &Model,
    faces: [usize; 2],
    cells: &mut usize,
    work: &mut u64,
    max: usize,
    max_work: u64,
) -> Result<Option<Certificate>> {
    let plane = &model.faces[faces[0]].surface;
    let sided = &model.faces[faces[1]].surface;
    if plane.periodic_u
        || plane.periodic_v
        || sided.periodic_u
        || sided.periodic_v
        || plane.control_points.len() * plane.control_points[0].len()
            + sided.control_points.len() * sided.control_points[0].len()
            > 4096
    {
        return Ok(None);
    }
    let a = plane.control_points.iter().flatten().collect::<Vec<_>>();
    let b = sided.control_points.iter().flatten().collect::<Vec<_>>();
    let values = a
        .iter()
        .chain(&b)
        .flat_map(|p| p.iter())
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let arena = SourceArena::authored("original-plane-boundary-tube", 1, values).map_err(|_| {
        Error::new(
            "BREP_BOUNDARY_TUBE_INPUT",
            "Invalid original coefficient arena",
        )
    })?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work - *work,
            ..Limits::default()
        },
        None,
    );
    let point = |i: usize| {
        [
            arena.leaf(3 * i).unwrap(),
            arena.leaf(3 * i + 1).unwrap(),
            arena.leaf(3 * i + 2).unwrap(),
        ]
    };
    let signs = (|| -> Option<Vec<Sign>> {
        let p1 = (1..a.len()).find(|&i| a[i] != a[0])?;
        let mut p2 = None;
        'search: for i in 1..a.len() {
            for axes in [[0, 1], [0, 2], [1, 2]] {
                if *cells == max {
                    return None;
                }
                *cells += 1;
                let p = |j: usize| axes.map(|k| arena.leaf(3 * j + k).unwrap());
                if matches!(
                    cad_predicates::orient2d(&mut ctx, p(0), p(p1), p(i))
                        .ok()?
                        .outcome,
                    Outcome::Sign(Sign::Positive | Sign::Negative)
                ) {
                    p2 = Some(i);
                    break 'search;
                }
            }
        }
        let p2 = p2?;
        for i in 0..a.len() {
            if *cells == max {
                return None;
            }
            *cells += 1;
            if cad_predicates::orient3d(&mut ctx, point(0), point(p1), point(p2), point(i))
                .ok()?
                .outcome
                != Outcome::Sign(Sign::Zero)
            {
                return None;
            }
        }
        let mut signs = Vec::new();
        for i in 0..b.len() {
            if *cells == max {
                return None;
            }
            *cells += 1;
            let Outcome::Sign(sign) = cad_predicates::orient3d(
                &mut ctx,
                point(0),
                point(p1),
                point(p2),
                point(a.len() + i),
            )
            .ok()?
            .outcome
            else {
                return None;
            };
            signs.push(sign);
        }
        Some(signs)
    })();
    *work += ctx.work_used();
    let Some(signs) = signs else {
        return Ok(None);
    };
    let mut boundary = None;
    for axis in 0..2 {
        let (degree, knots, n, free) = if axis == 0 {
            (
                sided.degree_u,
                &sided.knots_u,
                sided.control_points.len(),
                sided.control_points[0].len(),
            )
        } else {
            (
                sided.degree_v,
                &sided.knots_v,
                sided.control_points[0].len(),
                sided.control_points.len(),
            )
        };
        if !knots[..=degree].iter().all(|&x| x == knots[degree])
            || !knots[n..].iter().all(|&x| x == knots[n])
        {
            continue;
        }
        let at = |i: usize, j: usize| signs[if axis == 0 { i * free + j } else { j * n + i }];
        for end in 0..2 {
            let row = if end == 0 { 0 } else { n - 1 };
            let other = n - 1 - row;
            if !(0..free).all(|j| at(row, j) == Sign::Zero) {
                continue;
            }
            let sign = at(other, 0);
            if !matches!(sign, Sign::Positive | Sign::Negative) {
                continue;
            }
            if !(0..free).all(|j| at(other, j) == sign)
                || !signs.iter().all(|&s| s == Sign::Zero || s == sign)
            {
                continue;
            }
            // For a multi-span B-spline, strict sign must persist after the
            // first degree rows (or before the last degree rows). Positivity
            // of its active basis then excludes every interior zero, including
            // knots. Single Bernstein charts retain the less restrictive
            // endpoint-basis proof below.
            if n!=degree+1 && !(0..n).filter(|&i|if end==0 {i>=degree}else{i<n-degree})
                .all(|i|(0..free).all(|j|at(i,j)==sign)) {
                continue;
            }
            // Endpoint Bernstein basis is strictly positive away from its
            // opposite endpoint. Additional zero rows (G2 jets) are harmless.
            boundary = Some((axis, end, knots[if end == 0 { degree } else { n }]));
            break;
        }
        if boundary.is_some() {
            break;
        }
    }
    let Some((axis, end, value)) = boundary else {
        return Ok(None);
    };
    let Some(edges) = covers(model, faces[1], axis, end, value) else {
        return Ok(None);
    };
    // Closed, simple trimmed regions: no sampled witness authorizes coverage.
    for &face in &faces {
        if max - *cells < 2 {
            return Ok(None);
        }
        let f = &model.faces[face];
        let loops = std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .map(|w| {
                model.loops[w]
                    .coedges
                    .iter()
                    .map(|c| c.pcurve.clone())
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        let stage = ((max - *cells) / 2).min(100000);
        let r = nurbs_core::trim_region_audit::inspect(&loops, 1e-8, 10000, stage, stage)?;
        *cells += r.cells + r.domain_cells;
        if r.valid != Some(true) {
            return Ok(None);
        }
    }
    let f = &model.faces[faces[0]];
    let uses = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| &model.loops[w].coedges)
        .collect::<Vec<_>>();
    let mut ids = Vec::new();
    let mut upper = 0_f64;
    for c in edges {
        let same = uses.iter().filter(|u| u.edge == c.edge).collect::<Vec<_>>();
        if same.len() != 1 || *cells == max {
            return Ok(None);
        }
        let p = surface_g2_pcurves::verify_positions(
            &model.edges[c.edge].curve,
            [&c.pcurve, &same[0].pcurve],
            [sided, plane],
            [c.reversed, same[0].reversed],
            model.tolerance_mm,
            max - *cells,
            max_work - *work,
        )?;
        *cells += p.cells;
        *work += p.exact_work;
        if !p.proven {
            return Ok(None);
        }
        upper = upper.max(p.upper.unwrap());
        ids.push(c.edge);
    }
    Ok(Some(Certificate {
        plane_face: faces[0],
        sided_face: faces[1],
        edges: ids,
        boundary_error_upper: upper,
        boundary_axis: axis,
        boundary_end: end,
    }))
}
/// Scope: all possible surface contacts lie within model tolerance of these
/// actual shared edge definitions. This does not claim exact contact ownership,
/// absence of every mathematical boundary contact, or a valid filled volume.
pub fn inspect_pair(
    model: &Model,
    faces: [usize; 2],
    max_cells: usize,
    max_exact: u64,
) -> Result<Attempt> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= model.faces.len())
        || max_cells > 100000
        || max_exact > 1000000
    {
        return Err(Error::new(
            "BREP_BOUNDARY_TUBE_LIMITS",
            "Invalid pair or shared boundary-tube budget",
        ));
    }
    let mut cells = 0;
    let mut exact_work = 0;
    let mut certificate = None;
    for order in [faces, [faces[1], faces[0]]] {
        if cells == max_cells {
            break;
        }
        certificate = ordered(
            model,
            order,
            &mut cells,
            &mut exact_work,
            max_cells,
            max_exact,
        )?;
        if certificate.is_some() {
            break;
        }
    }
    if certificate.is_none() {
        for order in [faces, [faces[1], faces[0]]] {
            if cells == max_cells {
                break;
            }
            certificate = coordinate_end(
                model,
                order,
                &mut cells,
                &mut exact_work,
                max_cells,
                max_exact,
            )?;
            if certificate.is_some() {
                break;
            }
        }
    }
    Ok(Attempt {
        certificate,
        cells,
        exact_work,
    })
}

fn coordinate_end(
    model: &Model,
    faces: [usize; 2],
    cells: &mut usize,
    work: &mut u64,
    max: usize,
    max_work: u64,
) -> Result<Option<Certificate>> {
    let plane = &model.faces[faces[0]].surface;
    let side = &model.faces[faces[1]].surface;
    let points = plane.control_points.iter().flatten().collect::<Vec<_>>();
    // Only an exactly authored coordinate plane, not a sampled plane fit.
    let Some(coordinate) = (0..3).find(|&k| points.iter().all(|p| p[k] == points[0][k])) else {
        return Ok(None);
    };
    let f = &model.faces[faces[1]];
    let uses = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| &model.loops[w].coedges)
        .collect::<Vec<_>>();
    let pf = &model.faces[faces[0]];
    let other_uses = std::iter::once(pf.outer)
        .chain(pf.holes.iter().copied())
        .flat_map(|w| &model.loops[w].coedges)
        .collect::<Vec<_>>();
    let natural = [
        [
            side.knots_u[side.degree_u],
            side.knots_u[side.control_points.len()],
        ],
        [
            side.knots_v[side.degree_v],
            side.knots_v[side.control_points[0].len()],
        ],
    ];
    for axis in 0..2 {
        if *cells == max {
            return Ok(None);
        }
        let flow =
            nurbs_core::surface_coordinate_flow::inspect(side, axis, coordinate, max - *cells)?;
        *cells += flow.cells;
        let Some(factor) = flow.inverse_distance_factor else {
            continue;
        };
        let free = 1 - axis;
        for c in &uses {
            let pc = &c.pcurve;
            if pc.degree != 1
                || pc.control_points.len() != 2
                || pc.periodic
                || pc.weights[0] != pc.weights[1]
                || pc
                    .control_points
                    .iter()
                    .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
            {
                continue;
            }
            let ends = [pc.control_points[0][free], pc.control_points[1][free]];
            if ends != natural[free] && ends != [natural[free][1], natural[free][0]] {
                continue;
            }
            let other = other_uses
                .iter()
                .filter(|u| u.edge == c.edge)
                .collect::<Vec<_>>();
            if other.len() != 1 || *cells == max {
                continue;
            }
            let tolerance = model.tolerance_mm * 0.25;
            if tolerance <= 0. {
                return Ok(None);
            }
            let positions = surface_g2_pcurves::verify_positions(
                &model.edges[c.edge].curve,
                [pc, &other[0].pcurve],
                [side, plane],
                [c.reversed, other[0].reversed],
                tolerance,
                max - *cells,
                max_work - *work,
            )?;
            *cells += positions.cells;
            *work += positions.exact_work;
            if !positions.proven {
                continue;
            }
            // Let Q be the skin point on the actual owned end pcurve with the
            // same free parameter as an arbitrary plane contact P. Monotone
            // coordinate flow bounds |P-Q| by factor times Q's plane residual.
            // The verified shared edge bounds both that residual and |Q-edge|.
            use nurbs_core::interval_eval::Interval as I;
            let upper = I::point(positions.upper.unwrap())
                .mul(I::point(factor).add(I::point(1.))?)?
                .hi;
            if upper > model.tolerance_mm {
                continue;
            }
            // Recheck complete trim regions; supplied topology alone does not
            // promote an invalid end pcurve to an authored boundary certificate.
            let mut valid = true;
            for &face in &faces {
                if max - *cells < 2 {
                    valid = false;
                    break;
                }
                let f = &model.faces[face];
                let loops = std::iter::once(f.outer)
                    .chain(f.holes.iter().copied())
                    .map(|w| {
                        model.loops[w]
                            .coedges
                            .iter()
                            .map(|c| c.pcurve.clone())
                            .collect()
                    })
                    .collect::<Vec<Vec<_>>>();
                let stage = ((max - *cells) / 2).min(100000);
                let r = nurbs_core::trim_region_audit::inspect(&loops, 1e-8, 10000, stage, stage)?;
                *cells += r.cells + r.domain_cells;
                if r.valid != Some(true) {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }
            return Ok(Some(Certificate {
                plane_face: faces[0],
                sided_face: faces[1],
                edges: vec![c.edge],
                boundary_error_upper: upper,
                boundary_axis: axis,
                boundary_end: 2,
            }));
        }
    }
    Ok(None)
}
