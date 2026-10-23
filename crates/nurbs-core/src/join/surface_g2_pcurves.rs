//! Whole owned-curve G2 evidence on original face-local pcurves, including trims.
use crate::{
    Result, check,
    curve::Curve,
    curve_surface_agreement::{self, Status},
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
    surface_g2::{self, BranchBounds, Orientation},
    surface_join::Decision,
};
#[path = "surface_g2_pcurves/trace_mean_value.rs"]
mod trace_mean_value;
#[derive(Clone)]
pub struct PositionEvidence {
    pub exact: Option<cad_predicates::BezierIdentityDecision>,
    pub interval: Option<curve_surface_agreement::Report>,
}
#[derive(Clone)]
pub struct Positions {
    pub proven: bool,
    pub upper: Option<f64>,
    pub cells: usize,
    pub exact_work: u64,
    pub sides: [PositionEvidence; 2],
}
pub struct Report {
    pub positions: Positions,
    pub geometric: surface_g2::Report,
}
pub fn verify_positions(
    c: &Curve,
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    tol: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Positions> {
    c.validate()?;
    for k in 0..2 {
        p[k].validate()?;
        s[k].validate()?;
    }
    check(
        tol.is_finite() && tol >= 0. && max_cells <= 100000 && max_exact <= 1000000,
        "Owned seam position budgets/tolerance are invalid",
    )?;
    check(
        c.control_points[0].len() == 3
            && (0..2)
                .all(|k| p[k].control_points[0].len() == 2 && s[k].control_points[0][0].len() == 3),
        "Owned seam needs a 3D curve, 2D pcurves and 3D surfaces",
    )?;
    let mut out = Positions {
        proven: false,
        upper: None,
        cells: 0,
        exact_work: 0,
        sides: std::array::from_fn(|_| PositionEvidence {
            exact: None,
            interval: None,
        }),
    };
    let mut bounds = [None; 2];
    let half = (tol * 0.5).next_down().max(0.);
    for k in 0..2 {
        if out.exact_work < max_exact {
            let proof = curve_surface_agreement::verify_exact(
                c,
                p[k],
                s[k],
                reverse[k],
                max_exact - out.exact_work,
            )?;
            if let Some(r) = &proof {
                out.exact_work += r.work_used;
                if r.outcome == cad_predicates::BezierIdentity::Equal {
                    bounds[k] = Some(0.);
                }
            }
            out.sides[k].exact = proof;
        }
        if bounds[k].is_none() && half > 0. && out.cells < max_cells {
            let proof = curve_surface_agreement::verify(
                c,
                p[k],
                s[k],
                reverse[k],
                half,
                max_cells - out.cells,
            )?;
            out.cells += proof.cells;
            if proof.status == Status::WithinTolerance {
                bounds[k] = Some(half);
            }
            out.sides[k].interval = Some(proof);
        }
    }
    if let [Some(a), Some(b)] = bounds {
        out.upper = Some(if a == 0. && b == 0. {
            0.
        } else {
            (a + b).next_up()
        });
        out.proven = out.upper.is_some_and(|u| u <= tol);
    }
    Ok(out)
}
fn uv_box(
    p: &Curve,
    t: [f64; 2],
    reverse: bool,
    work: &mut usize,
    max: usize,
) -> Result<Option<[[f64; 2]; 2]>> {
    let mut fraction = I::new(t[0], t[1])?;
    if reverse {
        fraction = I::point(1.).sub(fraction)?.intersect(0., 1.)?;
    }
    let d = p.domain();
    let natural = I::point(d[0])
        .add(I::point(d[1]).sub(I::point(d[0]))?.mul(fraction)?)?
        .intersect(d[0], d[1])?;
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    let mut found = false;
    for span in p.degree..p.control_points.len() {
        let lo = natural.lo.max(p.knots[span]);
        let hi = natural.hi.min(p.knots[span + 1]);
        if p.knots[span] == p.knots[span + 1] || lo > hi {
            continue;
        }
        if *work == max {
            return Ok(None);
        }
        *work += 1;
        let box_ = crate::curve_distance::enclosure(p, span, I::new(lo, hi)?)?;
        found = true;
        for k in 0..2 {
            bounds[k][0] = bounds[k][0].min(box_[k].lo);
            bounds[k][1] = bounds[k][1].max(box_[k].hi);
        }
    }
    if !found {
        return Ok(None);
    }
    // Positive rational convex hull also preserves exact constant coordinates,
    // especially a trim on the original chart's natural boundary.
    for k in 0..2 {
        let (lo, hi) = p
            .control_points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v[k]), hi.max(v[k]))
            });
        let clipped = I::new(bounds[k][0], bounds[k][1])?.intersect(lo, hi)?;
        bounds[k] = [clipped.lo, clipped.hi];
    }
    Ok(Some(bounds))
}
type Distances = [[[f64; 2]; 2]; 2];
fn same_original_trace(p: [&Curve; 2], s: [&Surface; 2], reverse: [bool; 2]) -> bool {
    if s[0] != s[1] {
        return false;
    }
    for (knots, degree, n) in [
        (&s[0].knots_u, s[0].degree_u, s[0].control_points.len()),
        (&s[0].knots_v, s[0].degree_v, s[0].control_points[0].len()),
    ] {
        let max_mult = degree.saturating_sub(2);
        let mut i = 0;
        while i < knots.len() {
            let k = knots[i];
            let mut j = i + 1;
            while j < knots.len() && knots[j] == k {
                j += 1;
            }
            if knots[degree] < k && k < knots[n] && (j - i) > max_mult {
                return false;
            }
            i = j;
        }
    }
    if reverse[0] == reverse[1] {
        return p[0] == p[1];
    }
    let (a, b) = (p[0], p[1]);
    if a.degree != b.degree
        || a.periodic != b.periodic
        || a.control_points.len() != b.control_points.len()
        || a.knots.len() != b.knots.len()
        || !a.control_points.iter().rev().eq(b.control_points.iter())
        || !a.weights.iter().rev().eq(b.weights.iter())
    {
        return false;
    }
    // A rounded reversed knot vector cannot grant identity. Every reflected
    // knot must be exactly representable and equal to the actual other knot.
    let d = a.domain();
    let Some(sum) = crate::exact_curve_segments::exact_add(d[0], d[1]) else {
        return false;
    };
    a.knots
        .iter()
        .rev()
        .zip(&b.knots)
        .all(|(&x, &y)| crate::exact_curve_segments::exact_add(sum, -x) == Some(y))
}
fn pair(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut usize,
    max: usize,
) -> Result<Option<Distances>> {
    let mut normals = [[I::point(0.); 3]; 2];
    let mut tensors = [[I::point(0.); 9]; 2];
    for k in 0..2 {
        let Some(uv) = uv_box(p[k], t, reverse[k], work, max)? else {
            return Ok(None);
        };
        let report = crate::surface_shape_operator::rectangle_bounds(s[k], uv, max - *work)?;
        *work += report.cells;
        let (Some(n), Some(b)) = (report.shape.normal, report.shape.tensor) else {
            return Ok(None);
        };
        for i in 0..3 {
            normals[k][i] = I::new(n[i][0], n[i][1])?;
        }
        for i in 0..3 {
            for j in 0..3 {
                tensors[k][3 * i + j] = I::new(b[i][j][0], b[i][j][1])?;
            }
        }
    }
    let mut distances = [[[0.; 2]; 2]; 2];
    for sign in 0..2 {
        let mut y_n = [I::point(0.); 3];
        for i in 0..3 {
            let x = normals[1][i];
            y_n[i] = if sign == 0 {
                x
            } else {
                I::new(-x.hi, -x.lo)?
            };
        }
        let (lo_n, hi_n) = box_distance(&normals[0], &y_n)?;
        distances[sign][0] = [lo_n, hi_n.min(2.)];

        let mut y_b = [I::point(0.); 9];
        for i in 0..9 {
            let x = tensors[1][i];
            y_b[i] = if sign == 0 {
                x
            } else {
                I::new(-x.hi, -x.lo)?
            };
        }
        let (lo_b, hi_b) = box_distance(&tensors[0], &y_b)?;
        distances[sign][1] = [lo_b, hi_b];
    }
    // Whole original chart/trace identity correlates the shape fields exactly.
    // Both original regularity enclosures above remain mandatory, and position
    // agreement with the actual common 3D curve is checked separately.
    if same_original_trace(p, s, reverse) {
        distances[0] = [[0.; 2]; 2];
    }
    Ok(Some(distances))
}
struct Cell {
    t: [f64; 2],
    upper: Option<[[f64; 2]; 2]>,
    lower: [[f64; 2]; 2],
}
fn cell(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    normal: f64,
    curvature: f64,
    work: &mut usize,
    max: usize,
) -> Result<Cell> {
    // A finite mean-value enclosure supplies whole-cell regularity/upper
    // bounds and midpoint lower witnesses. Reuse this group on short traces
    // instead of querying an independent rectangle pair and midpoint again.
    // Missing mean-value evidence still falls back to the rectangle operator.
    let early_mean_value=t[1]-t[0]<=1./16. && !same_original_trace(p,s,reverse) && *work<max;
    if early_mean_value {
        if let Some((upper,lower))=trace_mean_value::bounds(p,s,reverse,t,work,max)? {
            return Ok(Cell {t,upper:Some(upper),lower});
        }
    }
    let full = pair(p, s, reverse, t, work, max)?;
    let mut upper = full.map(|d| d.map(|b| b.map(|x| x[1])));
    if !early_mean_value && !upper.is_some_and(|u| u.iter().any(|b| b[0] <= normal && b[1] <= curvature)) && *work < max {
        if let Some(bound) = trace_mean_value::upper(p, s, reverse, t, work, max)? {
            upper = Some(match upper {
                Some(old) => std::array::from_fn(|k| std::array::from_fn(|j| old[k][j].min(bound[k][j]))),
                None => bound,
            });
        }
    }
    let mut lower = [[0.; 2]; 2];
    if !upper.is_some_and(|u| u.iter().any(|b| b[0] <= normal && b[1] <= curvature)) && *work < max
    {
        let middle = t[0] * 0.5 + t[1] * 0.5;
        if let Some(d) = pair(p, s, reverse, [middle, middle], work, max)? {
            lower = d.map(|b| b.map(|x| x[0]));
        }
    }
    Ok(Cell { t, upper, lower })
}
/// Position agreement is checked against the actual common curve, not a supplied
/// claim. Shape boxes evaluate original rational jets over complete pcurve image
/// enclosures. Only whole-interval upper bounds grant G2; point boxes can prove
/// violations. One coherent sign applies to normal AND signed curvature tensor
/// over the entire seam. No exact strip, corner, thickness or solid claim follows.
pub fn inspect(
    c: &Curve,
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    position: f64,
    normal: f64,
    curvature: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    check(
        normal.is_finite()
            && (0. ..=2.).contains(&normal)
            && curvature.is_finite()
            && curvature >= 0.,
        "Owned seam shape tolerances are invalid",
    )?;
    let positions = verify_positions(c, p, s, reverse, position, max_cells, max_exact)?;
    let mut work = positions.cells;
    let make = |decision, work, upper: Option<[[f64; 2]; 2]>, lower: [[f64; 2]; 2], orientation| {
        surface_g2::Report {
            position_bounds: [0., positions.upper.unwrap_or(f64::INFINITY)],
            branches: std::array::from_fn(|k| BranchBounds {
                normal_lower: lower[k][0],
                tensor_lower: lower[k][1],
                normal_upper: upper.map(|u| u[k][0]),
                tensor_upper: upper.map(|u| u[k][1]),
            }),
            orientation,
            regularity_proven: upper.is_some(),
            decision,
            cells: work,
        }
    };
    if !positions.proven || work == max_cells || s.iter().any(|s| s.periodic_u || s.periodic_v) {
        let geometric = make(Decision::Unresolved, work, None, [[0.; 2]; 2], None);
        return Ok(Report {
            positions,
            geometric,
        });
    }
    let mut cells = vec![cell(
        p,
        s,
        reverse,
        [0., 1.],
        normal,
        curvature,
        &mut work,
        max_cells,
    )?];
    // A midpoint counterexample remains valid for the original whole seam
    // after its containing cell is refined or restored on work exhaustion.
    let mut witnesses=[[0_f64;2];2];
    loop {
        let upper = cells.iter().try_fold([[0_f64; 2]; 2], |mut u, c| {
            let x = c.upper?;
            for k in 0..2 {
                for j in 0..2 {
                    u[k][j] = u[k][j].max(x[k][j]);
                }
            }
            Some(u)
        });
        let mut lower = witnesses;
        for c in &cells {
            for k in 0..2 {
                for j in 0..2 {
                    lower[k][j] = lower[k][j].max(c.lower[k][j]);
                }
            }
        }
        witnesses=lower;
        let orientation = upper
            .and_then(|u| (0..2).find(|&k| u[k][0] <= normal && u[k][1] <= curvature))
            .map(|k| {
                if k == 0 {
                    Orientation::Same
                } else {
                    Orientation::Opposite
                }
            });
        let failed = lower.map(|b| b[0] > normal || b[1] > curvature);
        let decision = if failed[0] && failed[1] {
            Some(Decision::CorrespondenceExceedsTolerance)
        } else if orientation.is_some() {
            Some(Decision::WithinTolerance)
        } else if work == max_cells {
            Some(Decision::Unresolved)
        } else {
            None
        };
        if let Some(d) = decision {
            let geometric = make(
                d,
                work,
                upper,
                lower,
                if d == Decision::WithinTolerance {
                    orientation
                } else {
                    None
                },
            );
            return Ok(Report {
                positions,
                geometric,
            });
        }
        // Spend work on cells that still prevent a coherent orientation from
        // qualifying. Previously the longest already-qualified cells were
        // subdivided again while a short difficult interval remained unresolved.
        // This changes scheduling only: the final all-cell maximum and original
        // position/regularity evidence above remain the admission conditions.
        let ratio = |bound: f64, tolerance: f64| {
            if bound == 0. { 0. } else if tolerance == 0. { f64::INFINITY } else { bound / tolerance }
        };
        let target = upper.and_then(|u| (0..2).filter(|&k| !failed[k]).min_by(|&a, &b| {
            let score = |k: usize| ratio(u[k][0], normal).max(ratio(u[k][1], curvature));
            score(a).total_cmp(&score(b))
        }));
        let at = cells
            .iter()
            .enumerate()
            .filter(|(_, c)| match (c.upper, target) {
                (None, _) => true,
                (Some(u), Some(k)) => u[k][0] > normal || u[k][1] > curvature,
                (Some(_), None) => false,
            })
            .max_by(|(_, a), (_, b)| (a.t[1] - a.t[0]).total_cmp(&(b.t[1] - b.t[0])))
            .unwrap()
            .0;
        let old = cells.swap_remove(at);
        let mid = old.t[0] * 0.5 + old.t[1] * 0.5;
        if !(old.t[0] < mid && mid < old.t[1]) {
            let geometric = make(Decision::Unresolved, work, upper, lower, None);
            return Ok(Report {
                positions,
                geometric,
            });
        }
        let left=cell(
            p,
            s,
            reverse,
            [old.t[0], mid],
            normal,
            curvature,
            &mut work,
            max_cells,
        )?;
        for k in 0..2 {for j in 0..2 {witnesses[k][j]=witnesses[k][j].max(left.lower[k][j]);}}
        if work==max_cells && left.upper.is_none() {cells.push(old);continue;}
        let right=cell(
            p,
            s,
            reverse,
            [mid, old.t[1]],
            normal,
            curvature,
            &mut work,
            max_cells,
        )?;
        for k in 0..2 {for j in 0..2 {witnesses[k][j]=witnesses[k][j].max(right.lower[k][j]);}}
        // Do not erase a valid whole-parent enclosure with an uninspected
        // child merely because the final query consumed the remaining work.
        if work==max_cells && right.upper.is_none() {cells.push(old);continue;}
        cells.push(left);cells.push(right);
    }
}

#[cfg(test)]
mod trace_identity_tests {
    use super::*;
    #[test]
    fn chart_trace_identity_requires_exact_knots_and_fresh_regularity_and_positions() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let pc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.25, 0.5], vec![0.5, 0.25], vec![0.75, 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let back = pc.reverse().unwrap();
        assert!(same_original_trace([&pc, &back], [&s, &s], [false, true]));
        assert!(!same_original_trace([&pc, &back], [&s, &s], [false, false]));
        let mut changed = back.clone();
        changed.control_points[1][0] += 1e-12;
        assert!(!same_original_trace(
            [&pc, &changed],
            [&s, &s],
            [false, true]
        ));
        let mut changed = back.clone();
        changed.knots[0] = 1e-16;
        assert!(!same_original_trace(
            [&pc, &changed],
            [&s, &s],
            [false, true]
        ));
        let spline = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.1, 1., 1., 1.],
            control_points: vec![
                vec![0.25, 0.5],
                vec![0.4, 0.25],
                vec![0.6, 0.25],
                vec![0.75, 0.5],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let rounded_back = spline.reverse().unwrap();
        spline.validate().unwrap();
        rounded_back.validate().unwrap();
        assert!(!same_original_trace(
            [&spline, &rounded_back],
            [&s, &s],
            [false, true]
        ));
        let curve = Curve {
            control_points: pc
                .control_points
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            ..pc.clone()
        };
        let proof = inspect(
            &curve,
            [&pc, &back],
            [&s, &s],
            [false, true],
            1e-7,
            1e-5,
            1e-4,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(proof.geometric.decision, Decision::WithinTolerance);
        let mut wrong = curve.clone();
        wrong.control_points[1][2] = 0.01;
        let proof = inspect(
            &wrong,
            [&pc, &back],
            [&s, &s],
            [false, true],
            1e-7,
            1e-5,
            1e-4,
            100000,
            1000000,
        )
        .unwrap();
        assert!(!proof.positions.proven);
        assert_ne!(proof.geometric.decision, Decision::WithinTolerance);
        let mut singular = s.clone();
        singular.control_points[1] = singular.control_points[0].clone();
        let mut work = 0;
        assert!(
            pair(
                [&pc, &back],
                [&singular, &singular],
                [false, true],
                [0., 1.],
                &mut work,
                100000
            )
            .unwrap()
            .is_none()
        );
    }
}

/// Native ownership of positions proved on literal original coefficients.
/// Public report fields cannot construct this token. This is positional
/// admission only: a new G2 decision still requires fresh geometric inspection.
#[derive(Clone)]
pub struct OwnedPositions {
    curve: Curve,
    pcurves: [Curve; 2],
    surfaces: [Surface; 2],
    reversed: [bool; 2],
    tolerance: f64,
}
impl OwnedPositions {
    /// Only the actual stored definition/orientation may reuse admission.
    /// Coefficient equality establishes identity of the inspected input, not
    /// exact equality between different curve and surface representations.
    pub fn matches(&self,c:&Curve,p:&Curve,s:&Surface,reversed:bool,tolerance:f64)->bool {
        tolerance.is_finite() && self.tolerance<=tolerance && &self.curve==c
            && (0..2).any(|k|&self.pcurves[k]==p && &self.surfaces[k]==s && self.reversed[k]==reversed)
    }
    pub fn reverify(&self)->Result<()> {
        let proof=verify_positions(&self.curve,[&self.pcurves[0],&self.pcurves[1]],
            [&self.surfaces[0],&self.surfaces[1]],self.reversed,self.tolerance,100000,1000000)?;
        check(proof.proven,"Original owned seam positions remain unproven")
    }
}
pub struct OwnedInspection {
    pub report: Report,
    pub positions: Option<OwnedPositions>,
}
/// Inspect actual coefficients once; retain private positional snapshots only
/// after fresh full-curve admission. The G2 report is computed by inspect with
/// the same caller limits, without trusting a stored report or Boolean flag.
pub fn inspect_owned(
    c:&Curve,p:[&Curve;2],s:[&Surface;2],reversed:[bool;2],position:f64,normal:f64,curvature:f64,
    max_cells:usize,max_exact:u64,
)->Result<OwnedInspection> {
    let report=inspect(c,p,s,reversed,position,normal,curvature,max_cells,max_exact)?;
    let positions=report.positions.proven.then(||OwnedPositions {
        curve:c.clone(),pcurves:p.map(Clone::clone),surfaces:s.map(Clone::clone),reversed,tolerance:position,
    });
    Ok(OwnedInspection {report,positions})
}

#[path="surface_g2_pcurves/collar_owned.rs"]
mod collar_owned;
pub use collar_owned::{OwnedCollarInspection,inspect_owned_collar};
