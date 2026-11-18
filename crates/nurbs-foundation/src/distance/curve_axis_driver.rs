//! Strict monotonicity of one original clamped pcurve coordinate.
//! Cell derivatives are normalized to each cell; only their signs are combined.
use crate::{check, curve::Curve, Result};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    IntervalDerivative,
    OrderedRationalBezier,
}
pub struct Cell {
    pub parameter: [f64; 2],
    pub normalized_derivative: Option<[f64; 2]>,
    pub admitted: bool,
    pub method: Method,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub axis_extent: [f64; 2],
    pub increasing: bool,
    pub monotonic_proven: bool,
}
pub fn certify(curve: &Curve, axis: usize, max_cells: usize) -> Result<Report> {
    curve.validate()?;
    check(
        axis < curve.control_points[0].len()
            && !curve.periodic
            && (1..=100000).contains(&max_cells),
        "Choose an original nonperiodic coordinate and bounded driver work",
    )?;
    let d = curve.domain();
    check(
        curve.knots[..=curve.degree].iter().all(|&x| x == d[0])
            && curve.knots[curve.control_points.len()..]
                .iter()
                .all(|&x| x == d[1]),
        "Original driver endpoints must be clamped",
    )?;
    for &k in &curve.knots {
        if k > d[0] && k < d[1] {
            check(
                curve.knots.iter().filter(|&&x| x == k).count() <= curve.degree,
                "Driver must be continuous at every original knot",
            )?;
        }
    }
    let start = curve.control_points[0][axis];
    let end = curve.control_points.last().unwrap()[axis];
    let increasing = end > start;
    let mut out = Report {
        cells: vec![],
        visited: 0,
        axis_extent: [start.min(end), start.max(end)],
        increasing,
        monotonic_proven: false,
    };
    // Positive rational Bernstein basis. For 0<x<1, the derivative
    // numerator is the sum over i<j of (j-i)*wi*wj*(Pj-Pi)*Bi*Bj
    // divided by x*(1-x). Ordered poles and distinct end coordinates
    // therefore prove strict interior monotonicity, even if an endpoint
    // derivative vanishes. This is not an inverse regularity certificate.
    if curve.control_points.len() == curve.degree + 1
        && start != end
        && curve.control_points.windows(2).all(|p| {
            if increasing {
                p[0][axis] <= p[1][axis]
            } else {
                p[0][axis] >= p[1][axis]
            }
        })
    {
        out.visited = 1;
        out.monotonic_proven = true;
        out.cells.push(Cell {
            parameter: d,
            normalized_derivative: None,
            admitted: true,
            method: Method::OrderedRationalBezier,
        });
        return Ok(out);
    }
    let mut queue = vec![];
    for span in (curve.degree..curve.control_points.len()).rev() {
        let interval = [curve.knots[span], curve.knots[span + 1]];
        if interval[0] < interval[1] {
            queue.push((span, interval, 0usize));
        }
    }
    while let Some((span, parameter, depth)) = queue.pop() {
        if out.visited == max_cells {
            out.cells.push(Cell {
                parameter,
                normalized_derivative: None,
                admitted: false,
                method: Method::IntervalDerivative,
            });
            continue;
        }
        out.visited += 1;
        let derivative = crate::curve_jets::enclose(curve, span, parameter)?[1][axis];
        let admitted = if increasing {
            derivative.lo > 0.
        } else {
            end < start && derivative.hi < 0.
        };
        let mid = parameter[0] * 0.5 + parameter[1] * 0.5;
        if !admitted
            && out.visited < max_cells
            && depth < 32
            && mid > parameter[0]
            && mid < parameter[1]
        {
            queue.push((span, [mid, parameter[1]], depth + 1));
            queue.push((span, [parameter[0], mid], depth + 1));
        } else {
            out.cells.push(Cell {
                parameter,
                normalized_derivative: Some([derivative.lo, derivative.hi]),
                admitted,
                method: Method::IntervalDerivative,
            });
        }
    }
    out.cells
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    out.monotonic_proven = out.cells.iter().all(|c| c.admitted);
    Ok(out)
}
/// Fresh derivative coverage on a restricted interval of the unchanged curve.
/// No evaluated endpoint coordinates or rounded subcurve establish direction.
pub struct RestrictedReport {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub increasing: Option<bool>,
    pub monotonic_proven: bool,
}
pub fn certify_domain(curve:&Curve,axis:usize,domain:[f64;2],max_cells:usize)->Result<RestrictedReport> {
    curve.validate()?;
    check(axis<curve.control_points[0].len(),"Choose an original curve coordinate")?;
    certify_restricted(curve,domain,max_cells,|jet|Ok(jet[axis]))
}
/// Strict signed-plane monotonicity from outward original derivative bounds.
/// A successful derivative bound also proves the authored plane nondegenerate.
pub fn certify_plane_domain(curve:&Curve,plane:[[f64;3];3],domain:[f64;2],max_cells:usize)->Result<RestrictedReport> {
    use crate::interval_eval::Interval as I;
    curve.validate()?;
    check(curve.control_points[0].len()==3 && plane.iter().flatten().all(|x|x.is_finite()),
        "Choose an original 3D curve and finite plane anchors")?;
    let mut edges=[[I::point(0.);3];2];
    for side in 0..2 {for axis in 0..3 {
        edges[side][axis]=I::point(plane[side+1][axis]).sub(I::point(plane[0][axis]))?;
    }}
    let mut normal=[I::point(0.);3];
    for axis in 0..3 {let j=(axis+1)%3;let k=(axis+2)%3;
        normal[axis]=edges[0][j].mul(edges[1][k])?.sub(edges[0][k].mul(edges[1][j])?)?;
    }
    certify_restricted(curve,domain,max_cells,|jet| {
        let mut sum=I::point(0.);
        for axis in 0..3 {sum=sum.add(normal[axis].mul(jet[axis])?)?;}
        Ok(sum)
    })
}
fn certify_restricted(curve:&Curve,domain:[f64;2],max_cells:usize,
    project:impl Fn(&[crate::interval_eval::Interval])->Result<crate::interval_eval::Interval>)->Result<RestrictedReport> {
    let natural=curve.domain();
    check(!curve.periodic && max_cells<=100000
        && domain.iter().all(|x|x.is_finite()) && natural[0]<=domain[0]
        && domain[0]<domain[1] && domain[1]<=natural[1],
        "Choose an original coordinate, contained interval and bounded driver work")?;
    let mut out=RestrictedReport{cells:Vec::new(),visited:0,increasing:None,monotonic_proven:false};
    let mut queue=Vec::new();
    for span in (curve.degree..curve.control_points.len()).rev() {
        let interval=[curve.knots[span].max(domain[0]),curve.knots[span+1].min(domain[1])];
        if interval[0]<interval[1] {queue.push((span,interval,0usize));}
    }
    let mut consistent=true;
    while let Some((span,parameter,depth))=queue.pop() {
        if out.visited==max_cells {
            out.cells.push(Cell{parameter,normalized_derivative:None,admitted:false,method:Method::IntervalDerivative});
            continue;
        }
        out.visited+=1;
        let jet=crate::curve_jets::enclose(curve,span,parameter)?;
        let derivative=project(&jet[1])?;
        let direction=if derivative.lo>0. {Some(true)}else if derivative.hi<0. {Some(false)}else{None};
        if let Some(direction)=direction {
            if let Some(previous)=out.increasing {consistent &= previous==direction;}else{out.increasing=Some(direction);}
        }
        let mid=parameter[0]*0.5+parameter[1]*0.5;
        if direction.is_none() && out.visited<max_cells && depth<32 && parameter[0]<mid && mid<parameter[1] {
            queue.push((span,[mid,parameter[1]],depth+1));queue.push((span,[parameter[0],mid],depth+1));
        }else{out.cells.push(Cell{parameter,normalized_derivative:Some([derivative.lo,derivative.hi]),
            admitted:direction.is_some(),method:Method::IntervalDerivative});}
    }
    out.cells.sort_by(|a,b|a.parameter[0].total_cmp(&b.parameter[0]));
    out.monotonic_proven=consistent && out.increasing.is_some() && !out.cells.is_empty()
        && out.cells.iter().all(|cell|cell.admitted);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oblique_plane_driver_keeps_outward_bounds_and_refuses_tangent_or_degenerate_planes() {
        let curve=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.25,0.25,0.],vec![-0.25,-0.25,0.],vec![0.25,0.25,0.]],
            weights:vec![1.;3],periodic:false};
        let plane=[[0.,0.,0.],[1.,2.,0.],[0.,0.,1.]];
        let r=certify_plane_domain(&curve,plane,[0.1,0.4],64).unwrap();
        assert!(r.monotonic_proven);assert_eq!(r.increasing,Some(false));
        let reversed=[plane[0],plane[2],plane[1]];
        let r=certify_plane_domain(&curve,reversed,[0.1,0.4],64).unwrap();
        assert!(r.monotonic_proven);assert_eq!(r.increasing,Some(true));
        assert!(!certify_plane_domain(&curve,plane,[0.1,0.9],64).unwrap().monotonic_proven);
        assert!(!certify_plane_domain(&curve,[[0.;3];3],[0.1,0.4],16).unwrap().monotonic_proven);
        let tangent=[[0.,0.,0.],[1.,1.,0.],[0.,0.,1.]];
        assert!(!certify_plane_domain(&curve,tangent,[0.1,0.4],16).unwrap().monotonic_proven);
        let stopped=certify_plane_domain(&curve,plane,[0.1,0.4],0).unwrap();
        assert!(!stopped.monotonic_proven);assert_eq!(stopped.visited,0);
        let mut invalid=plane;invalid[0][0]=f64::NAN;
        assert!(certify_plane_domain(&curve,invalid,[0.1,0.4],0).is_err());
    }
    #[test]
    fn oblique_rational_multispan_driver_covers_knots_and_refuses_interior_reversal() {
        let mut curve=Curve{degree:1,knots:vec![2.,2.,4.,8.,8.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![3.,3.,0.]],
            weights:vec![1.,0.5,2.],periodic:false};
        let plane=[[0.,0.,0.],[1.,2.,0.],[0.,0.,1.]];
        let report=certify_plane_domain(&curve,plane,[2.,8.],10000).unwrap();
        assert!(report.monotonic_proven);assert_eq!(report.increasing,Some(true));
        assert_eq!(report.cells.first().unwrap().parameter[0],2.);
        assert_eq!(report.cells.last().unwrap().parameter[1],8.);
        for cells in report.cells.windows(2) {assert_eq!(cells[0].parameter[1],cells[1].parameter[0]);}
        assert!(report.cells.iter().any(|c|c.parameter[1]==4.));
        // Positive rational weights do not make opposite span directions safe.
        curve.control_points[2]=vec![0.,0.,0.];
        assert!(!certify_plane_domain(&curve,plane,[2.,8.],10000).unwrap().monotonic_proven);
        assert!(certify_plane_domain(&curve,plane,[2.,4.],10000).unwrap().monotonic_proven);
        let second=certify_plane_domain(&curve,plane,[4.,8.],10000).unwrap();
        assert!(second.monotonic_proven);assert_eq!(second.increasing,Some(false));
        assert!(!certify_plane_domain(&curve,plane,[2.,8.],1).unwrap().monotonic_proven);
    }
    #[test]
    fn restricted_original_driver_refuses_a_turn_and_keeps_local_direction() {
        let curve=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.25,0.],vec![-0.25,0.],vec![0.25,0.]],weights:vec![1.;3],periodic:false};
        assert!(!certify(&curve,0,64).unwrap().monotonic_proven);
        for (domain,increasing) in [([0.1,0.4],false),([0.6,0.9],true)] {
            let r=certify_domain(&curve,0,domain,64).unwrap();
            assert!(r.monotonic_proven);assert_eq!(r.increasing,Some(increasing));
            assert!(r.cells.iter().all(|c|domain[0]<=c.parameter[0] && c.parameter[1]<=domain[1]));
        }
        assert!(!certify_domain(&curve,0,[0.1,0.9],64).unwrap().monotonic_proven);
        let stopped=certify_domain(&curve,0,[0.1,0.4],0).unwrap();
        assert_eq!(stopped.visited,0);assert!(!stopped.monotonic_proven);assert_eq!(stopped.cells.len(),1);
        for domain in [[-0.1,0.4],[0.6,1.1],[0.4,0.4],[f64::NAN,0.4]] {
            assert!(certify_domain(&curve,0,domain,0).is_err());
        }
        assert!(certify_domain(&curve,2,[0.1,0.4],0).is_err());
        assert!(certify_domain(&curve,0,[0.1,0.4],100001).is_err());
    }

    #[test]
    fn nonunit_reversed_and_interior_turning_coordinates() {
        let mut c = Curve::from_polyline(vec![vec![1., 0.], vec![0., 1.]]).unwrap();
        c.knots = vec![2., 2., 8., 8.];
        let r = certify(&c, 0, 8).unwrap();
        assert!(r.monotonic_proven && !r.increasing);
        assert_eq!(r.axis_extent, [0., 1.]);
        c.degree = 3;
        c.knots = vec![2., 2., 2., 2., 8., 8., 8., 8.];
        c.control_points = vec![vec![0., 0.], vec![2., 0.], vec![-1., 0.], vec![1., 0.]];
        c.weights = vec![1.; 4];
        let r = certify(&c, 0, 31).unwrap();
        assert!(!r.monotonic_proven);
        assert_eq!(r.cells.first().unwrap().parameter[0], 2.);
        assert_eq!(r.cells.last().unwrap().parameter[1], 8.);
        for p in r.cells.windows(2) {
            assert_eq!(p[0].parameter[1], p[1].parameter[0]);
        }
        assert!(r.visited <= 31);
    }
    #[test]
    fn ordered_rational_bezier_allows_zero_endpoint_derivative_without_claiming_regular_inverse() {
        let mut c = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 8., 8., 8.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        for axis in 0..2 {
            let r = certify(&c, axis, 1).unwrap();
            assert!(r.monotonic_proven);
            assert_eq!(r.cells[0].method, Method::OrderedRationalBezier);
            assert!(r.cells[0].normalized_derivative.is_none());
        }
        c.control_points[1][0] = 1.2;
        assert!(!certify(&c, 0, 1).unwrap().monotonic_proven);
        c.control_points = vec![vec![0., 0.], vec![0., 1.], vec![0., 2.]];
        assert!(!certify(&c, 0, 1).unwrap().monotonic_proven);
    }
}
