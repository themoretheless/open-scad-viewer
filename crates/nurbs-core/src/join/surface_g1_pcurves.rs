//! Whole owned-pcurve tangent-plane agreement, including one-sided knot limits.
use crate::{
    Result, check,
    curve::Curve,
    interval_eval::Interval as I,
    surface::Surface,
    surface_g2::Orientation,
    surface_g2_pcurves::{self, Positions},
    surface_join::Decision,
};
type V = [I; 3];
pub struct Report {
    pub positions: Positions,
    pub decision: Decision,
    pub orientation: Option<Orientation>,
    pub normal_chord_upper: Option<f64>,
    pub regularity_proven: bool,
    pub violation_parameter: Option<f64>,
    pub cells: usize,
}
fn cross(a: V, b: V) -> Result<V> {
    Ok([
        a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
        a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
        a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
    ])
}
fn normalized(n: V) -> Result<Option<V>> {
    let lower = |v: I| {
        if v.lo > 0. {
            v.lo
        } else if v.hi < 0. {
            -v.hi
        } else {
            0.
        }
    };
    let k = (0..3)
        .max_by(|&a, &b| lower(n[a]).total_cmp(&lower(n[b])))
        .unwrap();
    if lower(n[k]) <= 0. {
        return Ok(None);
    }
    let mut ratios = [I::point(0.); 3];
    let mut sum = I::point(1.);
    for j in 0..3 {
        if j != k {
            ratios[j] = n[j].div_signed(n[k])?;
            sum = sum.add(ratios[j].mul(ratios[j])?.intersect(0., f64::INFINITY)?)?;
        }
    }
    let length = I::new(
        sum.lo.max(0.).sqrt().next_down().max(0.),
        sum.hi.sqrt().next_up(),
    )?;
    let dominant = I::point(if n[k].lo > 0. { 1. } else { -1. }).div(length)?;
    let mut out = [I::point(0.); 3];
    out[k] = dominant;
    for j in 0..3 {
        if j != k {
            out[j] = ratios[j].mul(dominant)?;
        }
    }
    Ok(Some(out))
}
fn normal_range(
    pc: &Curve,
    s: &Surface,
    q: [f64; 2],
    reverse: bool,
    cells: &mut usize,
    max: usize,
) -> Result<Option<V>> {
    let domain = pc.domain();
    let mapped = |t: f64| -> Result<I> {
        let t = if reverse {
            I::point(1.).sub(I::point(t))?
        } else {
            I::point(t)
        };
        I::point(domain[0])
            .add(I::point(domain[1]).sub(I::point(domain[0]))?.mul(t)?)?
            .intersect(domain[0], domain[1])
    };
    let a = mapped(q[0])?;
    let b = mapped(q[1])?;
    let range = I::new(a.lo.min(b.lo), a.hi.max(b.hi))?;
    let mut uv = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for i in pc.degree..pc.control_points.len() {
        let lo = range.lo.max(pc.knots[i]);
        let hi = range.hi.min(pc.knots[i + 1]);
        if pc.knots[i] >= pc.knots[i + 1] || lo > hi {
            continue;
        }
        if *cells == max {
            return Ok(None);
        }
        *cells += 1;
        let enclosure = crate::curve_distance::enclosure(pc, i, I::new(lo, hi)?)?;
        for k in 0..2 {
            uv[k][0] = uv[k][0].min(enclosure[k].lo);
            uv[k][1] = uv[k][1].max(enclosure[k].hi);
        }
    }
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for k in 0..2 {
        // Positive rational weights preserve an authored constant coordinate
        // exactly. Do not turn its outward evaluation halo into a subnormal
        // transverse chart whose artificial width cannot be differentiated.
        let constant = pc.control_points[0][k];
        if pc.control_points.iter().all(|p| p[k] == constant) {
            uv[k] = [constant, constant];
        }
        uv[k][0] = uv[k][0].max(natural[k][0]);
        uv[k][1] = uv[k][1].min(natural[k][1]);
    }
    normal_rectangle(s, uv, cells, max)
}
pub(crate) fn normal_rectangle(
    s: &Surface,
    uv: [[f64; 2]; 2],
    cells: &mut usize,
    max: usize,
) -> Result<Option<V>> {
    let mut result: Option<V> = None;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let full = [
                [s.knots_u[u], s.knots_u[u + 1]],
                [s.knots_v[v], s.knots_v[v + 1]],
            ];
            let clipped =
                std::array::from_fn(|k| [uv[k][0].max(full[k][0]), uv[k][1].min(full[k][1])]);
            if (0..2).any(|k| full[k][0] >= full[k][1] || clipped[k][0] > clipped[k][1]) {
                continue;
            }
            if *cells == max {
                return Ok(None);
            }
            *cells += 1;
            let j = crate::surface_measure::jets::calculate_partial_stable_up_to(
                s,
                [u, v],
                clipped,
                [None; 2],
                1,
            )?;
            let Some(n) = normalized(cross(j[1][0], j[0][1])?)? else {
                return Ok(None);
            };
            result = Some(if let Some(old) = result {
                std::array::from_fn(|k| I {
                    lo: old[k].lo.min(n[k].lo),
                    hi: old[k].hi.max(n[k].hi),
                })
            } else {
                n
            });
        }
    }
    Ok(result)
}
fn distance(a: V, b: V, opposite: bool) -> Result<f64> {
    let mut sum = I::point(0.);
    for k in 0..3 {
        let d = if opposite {
            a[k].add(b[k])?
        } else {
            a[k].sub(b[k])?
        };
        let high = d.lo.abs().max(d.hi.abs());
        sum = sum.add(I::point(high).mul(I::point(high))?)?;
    }
    Ok(sum.hi.max(0.).sqrt().next_up())
}
fn distance_lower(a: V, b: V, opposite: bool) -> Result<f64> {
    let mut sum = I::point(0.);
    for k in 0..3 {
        let d = if opposite {
            a[k].add(b[k])?
        } else {
            a[k].sub(b[k])?
        };
        let low = if d.lo <= 0. && d.hi >= 0. {
            0.
        } else {
            d.lo.abs().min(d.hi.abs())
        };
        sum = sum.add(I::point(low).mul(I::point(low))?)?;
    }
    Ok(sum.lo.max(0.).sqrt().next_down().max(0.))
}
/// Position evidence uses the actual shared 3D curve; both pcurves and every
/// intersected original knot cell supply fresh normals. One coherent sign is
/// required over the whole seam. All one-sided knot limits are included, without
/// claiming exact C1 storage continuity. No curvature/G2 certificate is issued.
pub fn inspect(
    c: &Curve,
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    position: f64,
    normal: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    check(
        normal.is_finite()
            && (0. ..=2.).contains(&normal)
            && max_cells <= 100000
            && max_exact <= 1000000,
        "G1 pcurve query needs bounded finite limits",
    )?;
    for i in 0..2 {
        p[i].validate()?;
        s[i].validate()?;
        let natural = [
            [
                s[i].knots_u[s[i].degree_u],
                s[i].knots_u[s[i].control_points.len()],
            ],
            [
                s[i].knots_v[s[i].degree_v],
                s[i].knots_v[s[i].control_points[0].len()],
            ],
        ];
        check(
            !p[i].periodic
                && !s[i].periodic_u
                && !s[i].periodic_v
                && p[i].control_points[0].len() == 2
                && s[i].control_points[0][0].len() == 3
                && p[i].control_points.len() <= 4096
                && p[i].weights.iter().all(|w| *w > 0.)
                && s[i].weights.iter().flatten().all(|w| *w > 0.)
                && s[i]
                    .control_points
                    .len()
                    .saturating_mul(s[i].control_points[0].len())
                    <= 4096
                && p[i]
                    .control_points
                    .iter()
                    .all(|cp| (0..2).all(|k| natural[k][0] <= cp[k] && cp[k] <= natural[k][1])),
            "G1 needs bounded original nonperiodic charts and contained pcurves",
        )?;
    }
    let positions =
        surface_g2_pcurves::verify_positions(c, p, s, reverse, position, max_cells, max_exact)?;
    let mut cells = positions.cells;
    let mut pending = vec![[0., 1.]];
    let mut upper = [0_f64; 2];
    let mut complete = positions.proven;
    if !positions.proven {
        pending.clear();
    }
    while let Some(q) = pending.pop() {
        if cells == max_cells {
            complete = false;
            break;
        }
        let a = normal_range(p[0], s[0], q, reverse[0], &mut cells, max_cells)?;
        let b = if cells < max_cells {
            normal_range(p[1], s[1], q, reverse[1], &mut cells, max_cells)?
        } else {
            None
        };
        if let (Some(a), Some(b)) = (a, b) {
            let bounds = [distance(a, b, false)?, distance(a, b, true)?];
            if bounds.iter().any(|u| *u <= normal) {
                for k in 0..2 {
                    upper[k] = upper[k].max(bounds[k]);
                }
                continue;
            }
        }
        let mid = q[0] * 0.5 + q[1] * 0.5;
        if cells < max_cells {
            let a = normal_range(p[0], s[0], [mid, mid], reverse[0], &mut cells, max_cells)?;
            let b = if cells < max_cells {
                normal_range(p[1], s[1], [mid, mid], reverse[1], &mut cells, max_cells)?
            } else {
                None
            };
            if let (Some(a), Some(b)) = (a, b) {
                if distance_lower(a, b, false)? > normal && distance_lower(a, b, true)? > normal {
                    return Ok(Report {
                        positions,
                        decision: Decision::CorrespondenceExceedsTolerance,
                        orientation: None,
                        normal_chord_upper: None,
                        regularity_proven: false,
                        violation_parameter: Some(mid),
                        cells,
                    });
                }
            }
        }
        if !(q[0] < mid && mid < q[1]) {
            complete = false;
            break;
        }
        pending.push([mid, q[1]]);
        pending.push([q[0], mid]);
    }
    let branch = if complete {
        (0..2).find(|&k| upper[k] <= normal)
    } else {
        None
    };
    Ok(Report {
        positions,
        decision: if branch.is_some() {
            Decision::WithinTolerance
        } else {
            Decision::Unresolved
        },
        orientation: branch.map(|k| {
            if k == 0 {
                Orientation::Same
            } else {
                Orientation::Opposite
            }
        }),
        normal_chord_upper: branch.map(|k| upper[k]),
        regularity_proven: complete,
        violation_parameter: None,
        cells,
    })
}

pub struct NormalRangeReport {
    pub normal: Option<[[f64; 2]; 3]>,
    pub cells: usize,
}
/// Original first-jet normal bounds on every intersected cell, including all
/// one-sided knot limits. This does not certify C1/C2 storage continuity,
/// injectivity, curvature, material opposition or wall thickness.
pub fn rectangle_normal_bounds(
    s: &Surface,
    query: [[f64; 2]; 2],
    max_cells: usize,
) -> Result<NormalRangeReport> {
    s.validate()?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    check(
        !s.periodic_u
            && !s.periodic_v
            && s.control_points[0][0].len() == 3
            && s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                <= 4096
            && max_cells <= 100000
            && (0..2).all(|k| {
                query[k].iter().all(|x| x.is_finite())
                    && domain[k][0] <= query[k][0]
                    && query[k][0] <= query[k][1]
                    && query[k][1] <= domain[k][1]
            }),
        "Need a bounded original normal rectangle",
    )?;
    let mut cells = 0;
    let normal =
        normal_rectangle(s, query, &mut cells, max_cells)?.map(|n| n.map(|v| [v.lo, v.hi]));
    Ok(NormalRangeReport { normal, cells })
}
