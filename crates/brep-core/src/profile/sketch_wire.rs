//! Planar 2D rational NURBS wire constructors shared by sketch extrusion and
//! planar arrangement / offset operations.
use crate::{Result, invalid};
use nurbs_core::curve::Curve;

pub fn bezier_wire(path: planar_geometry::path::BezierPath) -> Result<Vec<Curve>> {
    if !path.closed {
        return Err(invalid("Bézier extrusion requires a closed path"));
    }
    let mut start = path.start;
    let mut wire = Vec::new();
    for segment in path.segments {
        let controls = match segment {
            planar_geometry::path::PathSegment::Line { to } => vec![start.to_vec(), to.to_vec()],
            planar_geometry::path::PathSegment::Cubic { c1, c2, to } => {
                vec![start.to_vec(), c1.to_vec(), c2.to_vec(), to.to_vec()]
            }
        };
        start = segment.end();
        let degree = controls.len() - 1;
        wire.push(Curve {
            degree,
            knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            weights: vec![1.; controls.len()],
            control_points: controls,
            periodic: false,
        });
    }
    if start != path.start {
        return Err(invalid("Bézier closing span must be explicit"));
    }
    Ok(wire)
}

pub fn polygon_wire(mut ring: Vec<[f64; 2]>) -> Result<Vec<Curve>> {
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 || ring.len() > 512 {
        return Err(invalid("Profile ring requires 3..512 points"));
    }
    (0..ring.len())
        .map(|i| {
            Curve::from_polyline(vec![
                ring[i].to_vec(),
                ring[(i + 1) % ring.len()].to_vec(),
            ])
        })
        .collect()
}

pub fn circle_wire(radius: f64) -> Result<Vec<Curve>> {
    if !radius.is_finite() || radius <= 0. || radius > 1e6 {
        return Err(invalid("Invalid profile circle radius"));
    }
    let cardinal = [[radius, 0.], [0., radius], [-radius, 0.], [0., -radius]];
    (0..4)
        .map(|i| {
            let a = cardinal[i];
            let b = cardinal[(i + 1) % 4];
            let curve = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![a.to_vec(), vec![a[0] + b[0], a[1] + b[1]], b.to_vec()],
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            };
            curve.validate()?;
            Ok(curve)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_wires_retain_analytic_geometry_and_closure() {
        let wire = circle_wire(3.).unwrap();
        crate::planar_trim::validate(std::slice::from_ref(&wire), 1e-7).unwrap();
        for arc in wire {
            for i in 0..=16 {
                let p = arc.evaluate(i as f64 / 16.).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - 9.).abs() < 1e-12);
            }
        }
        let ring = polygon_wire(vec![[0., 0.], [2., 0.], [2., 1.], [0., 1.], [0., 0.]]).unwrap();
        assert_eq!(ring.len(), 4);
        assert!((crate::planar_trim::signed_area(&ring, 1e-7).unwrap() - 2.).abs() < 1e-12);
        assert!(circle_wire(0.).is_err());
    }
}
