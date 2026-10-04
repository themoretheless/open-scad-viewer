//! Whole-chart injectivity under one fixed exact integer linear projection.
//! Sampling may propose a projection elsewhere, but never certifies this result.
use crate::{
    Result, check, distance_bounds::Interval as I, surface::Surface, surface_measure::jets,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub certified: bool,
    pub cells: usize,
    pub projection: [[i8; 3]; 2],
    pub reason: Option<&'static str>,
}
pub fn inspect(s: &Surface, projection: [[i8; 3]; 2], max_cells: usize) -> Result<Report> {
    inspect_with_depth(s, projection, max_cells, 12)
}
fn inspect_with_depth(s: &Surface, projection: [[i8; 3]; 2], max_cells: usize, max_depth: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && projection.iter().flatten().all(|v| (-16..=16).contains(v)),
        "Invalid linear projection or cell budget",
    )?;
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    check(independent, "Linear projection rows must be independent")?;
    let mut out = Report {
        certified: false,
        cells: 0,
        projection,
        reason: Some("linear-monotone-projection-unproved"),
    };
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || (0..2).any(|k| {
            knots[k]
                .iter()
                .filter(|v| **v > knots[k][degrees[k]] && **v < knots[k][counts[k]])
                .any(|v| knots[k].iter().filter(|x| *x == v).count() > degrees[k])
        })
    {
        out.reason = Some("continuous-rectangle-domain-unproved");
        return Ok(out);
    }
    let mut pending = Vec::new();
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] < knots[0][u + 1] && knots[1][v] < knots[1][v + 1] {
                if pending.len() == max_cells {
                    out.reason = Some("cell-budget-exhausted");
                    return Ok(out);
                }
                pending.push((
                    [u, v],
                    [
                        [knots[0][u], knots[0][u + 1]],
                        [knots[1][v], knots[1][v + 1]],
                    ],
                    0usize,
                ));
            }
        }
    }
    while let Some((span, domain, depth)) = pending.pop() {
        if out.cells == max_cells {
            out.reason = Some("cell-budget-exhausted");
            return Ok(out);
        }
        out.cells += 1;
        let result = (|| -> Result<i8> {
            let j = jets::calculate(s, span, domain, None)?;
            let mut projected = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
                    for k in 0..3 {
                        if projection[row][k] == 0 {
                            continue;
                        }
                        let derivative = if axis == 0 { j[1][0][k] } else { j[0][1][k] };
                        projected[row][axis] = projected[row][axis].add(
                            derivative
                                .div(width)?
                                .mul(I::point(projection[row][k] as f64))?,
                        )?;
                    }
                }
            }
            let [[uu, uv], [vu, vv]] = projected;
            if uu.hi <= 0. || vv.hi <= 0. {
                return Ok(-1);
            }
            let off = uv.add(vu)?.mul(I::point(0.5))?;
            let radius = I::point(off.lo.abs().max(off.hi.abs()));
            Ok(i8::from(
                uu.lo > 0. && vv.lo > 0. && uu.mul(vv)?.sub(radius.mul(radius)?)?.lo > 0.,
            ))
        })();
        match result {
            Ok(1) => (),
            Ok(0) if depth < max_depth => {
                let axis = depth % 2;
                let middle = domain[axis][0] * 0.5 + domain[axis][1] * 0.5;
                if !(domain[axis][0] < middle && middle < domain[axis][1]) {
                    out.reason = Some("parameter-subdivision-unresolved");
                    return Ok(out);
                }
                let mut left = domain;
                let mut right = domain;
                left[axis][1] = middle;
                right[axis][0] = middle;
                pending.push((span, right, depth + 1));
                pending.push((span, left, depth + 1));
            }
            Ok(0) => {
                out.reason = Some("subdivision-depth-exhausted");
                return Ok(out);
            }
            Err(_) => {
                out.reason = Some("numerical-enclosure-unresolved");
                return Ok(out);
            }
            _ => return Ok(out),
        }
    }
    out.certified = true;
    out.reason = None;
    Ok(out)
}
/// Exact comparisons of retained corner controls only propose a projection.
/// The full outward proof remains mandatory; no corner sample certifies a chart.
pub fn inspect_candidate(s: &Surface, max_cells: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000 && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Invalid linear injectivity budget/dimension",
    )?;
    let a = &s.control_points[0][0];
    let ends = [
        &s.control_points[s.control_points.len() - 1][0],
        &s.control_points[0][s.control_points[0].len() - 1],
    ];
    let mut projection:[[i8;3];2] = std::array::from_fn(|row| {
        std::array::from_fn(|k| {
            if ends[row][k] > a[k] {
                1
            } else if ends[row][k] < a[k] {
                -1
            } else {
                0
            }
        })
    });
    // An axis-aligned station chord can dominate the symmetric Jacobian if
    // included in both rows. Propose the transverse row without that axis.
    // This changes only the candidate; the same complete proof is required.
    let station_axes=(0..3).filter(|&k|projection[1][k]!=0).collect::<Vec<_>>();
    if station_axes.len()==1 {
        projection[0][station_axes[0]]=0;
    }
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    let first = if !independent {
        Report {
            certified: false,
            cells: 0,
            projection,
            reason: Some("linear-projection-proposal-unproved"),
        }
    } else { inspect(s, projection, max_cells)? };
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    // A rounded transverse direction only proposes another fixed integer map.
    // Every cell is proved again, with the remaining shared work budget.
    let chord: [f64; 3] = std::array::from_fn(|k| ends[1][k] - a[k]);
    let length2: f64 = chord.iter().map(|x| x*x).sum();
    let dot: f64 = (0..3).map(|k| projection[0][k] as f64 * chord[k]).sum();
    let transverse: [f64; 3] = std::array::from_fn(|k|
        projection[0][k] as f64 - dot / length2 * chord[k]);
    let scale = transverse.iter().map(|x| x.abs()).fold(0., f64::max);
    if !length2.is_finite() || length2 <= 0. || !scale.is_finite() || scale <= 0. {
        return Ok(first);
    }
    let station_scale = chord.iter().map(|x| x.abs()).fold(0., f64::max);
    let alternative = [transverse.map(|x| (2.*x/scale).round() as i8),
        chord.map(|x| (2.*x/station_scale).round() as i8)];
    let independent = (0..3).any(|k| alternative[0][k] as i16 * alternative[1][(k+1)%3] as i16
        != alternative[1][k] as i16 * alternative[0][(k+1)%3] as i16);
    if !independent || alternative == projection { return Ok(first); }
    let mut second = inspect(s, alternative, max_cells-first.cells)?;
    second.cells += first.cells;
    if second.certified || second.cells == max_cells { return Ok(second); }
    // Midpoint tangents propose dual directions. The sampled jet supplies no
    // evidence: only the fixed integer map's whole-domain interval proof counts.
    let u = 0.5*s.knots_u[s.degree_u] + 0.5*s.knots_u[s.control_points.len()];
    let v = 0.5*s.knots_v[s.degree_v] + 0.5*s.knots_v[s.control_points[0].len()];
    let Some((du,dv)) = s.evaluate(u,v)?.first_derivatives() else { return Ok(second); };
    let dot = |a:[f64;3],b:[f64;3]| (0..3).map(|k|a[k]*b[k]).sum::<f64>();
    let uu=dot(du,du); let uv=dot(du,dv); let vv=dot(dv,dv);
    let dual:[ [f64;3];2 ] = [std::array::from_fn(|k|vv*du[k]-uv*dv[k]),
        std::array::from_fn(|k|uu*dv[k]-uv*du[k])];
    let mut candidate=[[0_i8;3];2];
    for row in 0..2 {
        let scale=dual[row].iter().map(|x|x.abs()).fold(0.,f64::max);
        if !scale.is_finite() || scale <= 0. { return Ok(second); }
        candidate[row]=dual[row].map(|x|(16.*x/scale).round() as i8);
    }
    let independent=(0..3).any(|k|candidate[0][k] as i16*candidate[1][(k+1)%3] as i16
        != candidate[1][k] as i16*candidate[0][(k+1)%3] as i16);
    if !independent || candidate==alternative || candidate==projection { return Ok(second); }
    let mut third=inspect_with_depth(s,candidate,max_cells-second.cells,14)?;
    third.cells+=second.cells;
    Ok(third)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transverse_proposal_does_not_mix_long_station_motion_into_profile_row() {
        let mut s=quarter();
        for row in &mut s.control_points {
            for (v,p) in row.iter_mut().enumerate() { p[2]=100.*v as f64+p[0]; }
        }
        assert!(!inspect(&s,[[-1,1,-1],[0,0,1]],1000).unwrap().certified);
        let r=inspect_candidate(&s,1000).unwrap();
        assert!(r.certified,"{r:?}");
        assert_eq!(r.projection,[[-1,1,0],[0,0,1]]);
        assert!(!inspect_candidate(&s,0).unwrap().certified);
    }
    #[test]
    fn spatial_station_uses_a_fixed_transverse_integer_projection() {
        let mut s = quarter();
        for row in &mut s.control_points {
            for (v,p) in row.iter_mut().enumerate() {
                let y=p[1];
                p[1]=y+10.*v as f64;
                p[2]=-2.*y+5.*v as f64;
            }
        }
        let r=inspect_candidate(&s,10000).unwrap();
        assert!(r.certified,"{r:?}");
        assert!(r.cells<=10000);
        assert!(!inspect_candidate(&s,0).unwrap().certified);
    }
    fn quarter() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 0., 5.]],
                vec![vec![1., 1., 0.], vec![1., 1., 5.]],
                vec![vec![0., 1., 0.], vec![0., 1., 5.]],
            ],
            weights: vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_quarter_wall_has_one_global_oblique_projection() {
        let s = quarter();
        let p = [[-1, 1, 0], [0, 0, 1]];
        let r = inspect(&s, p, 1000).unwrap();
        assert!(inspect_candidate(&s, 1000).unwrap().certified);
        assert!(r.certified, "{r:?}");
        assert!(r.cells <= 1000);
        assert!(!inspect(&s, p, 0).unwrap().certified);
        assert!(!inspect(&s, [[1, 0, 0], [0, 0, 1]], 100).unwrap().certified);
        assert!(inspect(&s, [[1, 0, 0], [1, 0, 0]], 100).is_err());
    }
    #[test]
    fn folded_chart_cannot_change_projection_between_spans() {
        let mut s = quarter();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points[2] = s.control_points[0].clone();
        s.weights = vec![vec![1.; 2]; 3];
        assert!(!inspect_candidate(&s,1000).unwrap().certified);
        assert!(
            !inspect(&s, [[-1, 1, 0], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
    }
}
