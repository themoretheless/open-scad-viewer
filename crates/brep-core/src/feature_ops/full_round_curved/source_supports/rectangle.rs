//! Exact tensor factorization and source UV rectangle coverage. All floating
//! values are original stored coefficients; no iso/trim fitting supplies data.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FaceData {
    pub face: usize,
    pub axis: usize,
    pub profile: Curve,
    pub rectangle: [[f64; 2]; 2],
    pub vertical_z: [f64; 2],
    pub ratio: ExactRatio,
    pub top: f64,
    pub top_edges: Vec<usize>,
}
fn value(surface: &nurbs_core::surface::Surface, axis: usize, i: usize, j: usize) -> (&[f64], f64) {
    let (u, v) = if axis == 0 { (j, i) } else { (i, j) };
    (&surface.control_points[u][v], surface.weights[u][v])
}
pub(super) fn derive(model: &Model, face: usize, height: [f64; 2]) -> Result<FaceData> {
    let f = &model.faces[face];
    let s = &f.surface;
    if !f.holes.is_empty() {
        return Err(error("Source extrusion side has nonrectangular holes"));
    }
    let mut selected = None;
    for axis in 0..2 {
        let (degree, count, periodic) = if axis == 0 {
            (s.degree_u, s.control_points.len(), s.periodic_u)
        } else {
            (s.degree_v, s.control_points[0].len(), s.periodic_v)
        };
        if degree != 1 || count != 2 || periodic {
            continue;
        }
        let n = if axis == 0 {
            s.control_points[0].len()
        } else {
            s.control_points.len()
        };
        let (first, w0) = value(s, axis, 0, 0);
        let (last, w1) = value(s, axis, 0, 1);
        let z = [first[2], last[2]];
        if !((z[0] == height[0] && z[1] == height[1]) || (z[0] == height[1] && z[1] == height[0])) {
            continue;
        }
        if w0 <= 0. || w1 <= 0. {
            continue;
        }
        let ratio = q(w1)? / q(w0)?;
        let mut valid = true;
        for i in 0..n {
            let (a, wa) = value(s, axis, i, 0);
            let (b, wb) = value(s, axis, i, 1);
            if a[0] != b[0]
                || a[1] != b[1]
                || a[2] != z[0]
                || b[2] != z[1]
                || wa <= 0.
                || wb <= 0.
                || q(wb)? != q(wa)? * &ratio
            {
                valid = false;
                break;
            }
        }
        if valid {
            selected = Some((axis, n, z, ratio));
            break;
        }
    }
    let (axis, n, z, ratio) = selected
        .ok_or_else(|| error("Source surface has no exact extrusion tensor factorization"))?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut rectangle = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    let w = &model.loops[f.outer];
    if !(4..=256).contains(&w.coedges.len()) {
        return Err(error("Source side rectangle coedge budget exceeded"));
    }
    let mut paths = Vec::new();
    for c in &w.coedges {
        let p = &c.pcurve;
        if p.degree != 1
            || p.control_points.len() != 2
            || p.weights.iter().any(|w| *w <= 0.)
            || p.periodic
            || c.trim().is_some_and(|t| {
                t.curve_parameter != model.edges[c.edge].curve.domain()
                    || t.pcurve_parameter != p.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            })
        {
            return Err(error(
                "Source UV side is not an authored full positive-weight straight segment",
            ));
        }
        let a = [p.control_points[0][0], p.control_points[0][1]];
        let b = [p.control_points[1][0], p.control_points[1][1]];
        for p in [a, b] {
            for k in 0..2 {
                rectangle[k][0] = rectangle[k][0].min(p[k]);
                rectangle[k][1] = rectangle[k][1].max(p[k]);
            }
        }
        paths.push((a, b, c.edge));
    }
    for k in 0..2 {
        if rectangle[k][0] >= rectangle[k][1]
            || rectangle[k][0] < domain[k][0]
            || rectangle[k][1] > domain[k][1]
        {
            return Err(error("Source side trim rectangle leaves its native chart"));
        }
    }
    if rectangle[axis] != domain[axis] {
        return Err(error("Source side does not span the complete height chart"));
    }
    let top = domain[axis][usize::from(z[1] == height[1])];
    let mut coverage = BTreeMap::<(usize, usize), Vec<[f64; 2]>>::new();
    let mut top_edges = Vec::new();
    for (i, (a, b, edge)) in paths.iter().enumerate() {
        if *b != paths[(i + 1) % paths.len()].0 {
            return Err(error("Source UV rectangle has an exact endpoint gap"));
        }
        let constant = if a[0] == b[0] {
            0
        } else if a[1] == b[1] {
            1
        } else {
            return Err(error("Source UV boundary is not axis aligned"));
        };
        let varying = 1 - constant;
        let side = if a[constant] == rectangle[constant][0] {
            0
        } else if a[constant] == rectangle[constant][1] {
            1
        } else {
            return Err(error("Source UV segment is not on the rectangle boundary"));
        };
        if a[varying] == b[varying] {
            return Err(error("Collapsed source UV rectangle side"));
        }
        coverage
            .entry((constant, side))
            .or_default()
            .push([a[varying], b[varying]]);
        if constant == axis && a[constant] == top {
            top_edges.push(*edge);
        }
    }
    if coverage.len() != 4 {
        return Err(error("Incomplete source UV rectangle boundary"));
    }
    for ((constant, _), intervals) in &mut coverage {
        let forward = intervals[0][0] < intervals[0][1];
        if intervals.iter().any(|p| (p[0] < p[1]) != forward) {
            return Err(error("Source UV rectangle reverses a side"));
        }
        let mut spans = intervals
            .iter()
            .map(|p| [p[0].min(p[1]), p[0].max(p[1])])
            .collect::<Vec<_>>();
        spans.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let d = rectangle[1 - *constant];
        if spans[0][0] != d[0]
            || spans.last().unwrap()[1] != d[1]
            || spans.windows(2).any(|p| p[0][1] != p[1][0])
        {
            return Err(error("Source UV rectangle side has gaps or overlaps"));
        }
    }
    top_edges.sort_unstable();
    if top_edges.windows(2).any(|p| p[0] == p[1]) {
        return Err(error("Source side repeats a top rim edge"));
    }
    let profile = Curve {
        degree: if axis == 0 { s.degree_v } else { s.degree_u },
        knots: if axis == 0 {
            s.knots_v.clone()
        } else {
            s.knots_u.clone()
        },
        control_points: (0..n)
            .map(|i| {
                let (p, _) = value(s, axis, i, 0);
                vec![p[0], p[1]]
            })
            .collect(),
        weights: (0..n).map(|i| value(s, axis, i, 0).1).collect(),
        periodic: if axis == 0 {
            s.periodic_v
        } else {
            s.periodic_u
        },
    };
    profile.validate()?;
    Ok(FaceData {
        face,
        axis,
        profile,
        rectangle,
        vertical_z: z,
        ratio: ratio_value(&ratio),
        top,
        top_edges,
    })
}
