//! Boundary-position bounds under an explicit affine parameter correspondence.
use crate::distance_bounds::box_distance;
use crate::{Result, check, surface::Surface};
pub use crate::analysis::surface_boundary::{Boundary, Decision};
pub(crate) use crate::analysis::surface_boundary::boundary_box;
#[derive(Clone, Debug)]
pub struct Report {
    pub bounds: [f64; 2],
    pub decision: Decision,
    pub cells: usize,
}
/// Sufficient identity of the represented, clamped boundary curves under the
/// same normalized parameter. No rounded knot normalization or sampled equality.
pub(crate) fn identical_boundary(a:&Surface,ea:Boundary,b:&Surface,eb:Boundary)->bool {
    fn extract(s:&Surface,e:Boundary)->Option<(usize,&[f64],bool,Vec<&Vec<f64>>,Vec<f64>)> {
        let (fixed,max)=match e {Boundary::UMin=>(0,false),Boundary::UMax=>(0,true),Boundary::VMin=>(1,false),Boundary::VMax=>(1,true)};
        let (p,k,n,periodic)=if fixed==0 {(s.degree_u,&s.knots_u,s.control_points.len(),s.periodic_u)}else{(s.degree_v,&s.knots_v,s.control_points[0].len(),s.periodic_v)};
        let endpoint=k[if max {n}else{p}];
        let end=if max {&k[n..]}else{&k[..=p]};
        if periodic||end.len()!=p+1||!end.iter().all(|&v|v==endpoint){return None;}
        let i=if max {n-1}else{0};
        Some(if fixed==0 {(s.degree_v,&s.knots_v,s.periodic_v,s.control_points[i].iter().collect(),s.weights[i].clone())}
          else{(s.degree_u,&s.knots_u,s.periodic_u,s.control_points.iter().map(|row|&row[i]).collect(),s.weights.iter().map(|row|row[i]).collect())})
    }
    match(extract(a,ea),extract(b,eb)) {(Some(a),Some(b))=>a==b,_=>false}
}

struct Cell {
    t: [f64; 2],
    bounds: [f64; 2],
}
fn cell(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    t: [f64; 2],
) -> Result<Cell> {
    let upper = box_distance(
        &boundary_box(a, ea, t, false)?,
        &boundary_box(b, eb, t, reverse)?,
    )?
    .1;
    let mut lower = 0_f64;
    for u in [t[0], t[0] * 0.5 + t[1] * 0.5, t[1]] {
        lower = lower.max(
            box_distance(
                &boundary_box(a, ea, [u, u], false)?,
                &boundary_box(b, eb, [u, u], reverse)?,
            )?
            .0,
        );
    }
    Ok(Cell {
        t,
        bounds: [lower, upper],
    })
}
/// A sufficient G0 test for two chosen boundaries with normalized affine
/// parameter correspondence. Rejection concerns that correspondence, not every
/// possible reparameterization of geometrically coincident boundary images.
pub fn inspect_g0(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        tolerance.is_finite() && tolerance >= 0.,
        "G0 tolerance must be finite and nonnegative",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "G0 needs 1..100000 cells",
    )?;
    let mut cells = vec![cell(a, ea, b, eb, reverse, [0., 1.])?];
    let mut work = 1;
    let mut lower = cells[0].bounds[0];
    let decision;
    loop {
        let upper = cells.iter().map(|x| x.bounds[1]).fold(lower, f64::max);
        if upper <= tolerance {
            decision = Decision::WithinTolerance;
            break;
        }
        if lower > tolerance {
            decision = Decision::CorrespondenceExceedsTolerance;
            break;
        }
        if work + 2 > max_cells {
            decision = Decision::Unresolved;
            break;
        }
        let index = cells
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.bounds[1].total_cmp(&b.1.bounds[1]))
            .unwrap()
            .0;
        let t = cells[index].t;
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if !(t[0] < mid && mid < t[1]) {
            decision = Decision::Unresolved;
            break;
        }
        cells.swap_remove(index);
        for next in [[t[0], mid], [mid, t[1]]] {
            let x = cell(a, ea, b, eb, reverse, next)?;
            lower = lower.max(x.bounds[0]);
            cells.push(x);
            work += 1;
        }
    }
    Ok(Report {
        bounds: [
            lower,
            cells.iter().map(|x| x.bounds[1]).fold(lower, f64::max),
        ],
        decision,
        cells: work,
    })
}
