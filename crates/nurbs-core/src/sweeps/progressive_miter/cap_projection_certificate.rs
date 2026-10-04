//! Conditional filled-cap projection prerequisite. Callers must provide original
//! ideal/retained plane normal enclosures; this does not prove cap ownership.
use crate::{Result, distance_bounds::Interval as I};
#[derive(Clone, Debug)]
pub struct Report {
    pub projection_regular: bool,
    pub normal_dot: Option<[f64; 2]>,
    pub absolute_dot_lower: Option<f64>,
    /// Reflection is permitted only if all boundary orientations are reversed.
    pub reverses_orientation: Option<bool>,
    pub cells: usize,
}
pub fn certify(ideal: [[f64; 2]; 3], retained: [[f64; 2]; 3], max_cells: usize) -> Result<Report> {
    let mut a = [I::point(0.); 3];
    let mut b = a;
    for k in 0..3 {
        a[k] = I::new(ideal[k][0], ideal[k][1])?;
        b[k] = I::new(retained[k][0], retained[k][1])?;
    }
    let mut report = Report {
        projection_regular: false,
        normal_dot: None,
        absolute_dot_lower: None,
        reverses_orientation: None,
        cells: 0,
    };
    if max_cells == 0 {
        return Ok(report);
    }
    report.cells = 1;
    let mut dot = I::point(0.);
    for k in 0..3 {
        dot = dot.add(a[k].mul(b[k])?)?;
    }
    report.normal_dot = Some([dot.lo, dot.hi]);
    let lower = if dot.lo > 0. {
        Some(dot.lo)
    } else if dot.hi < 0. {
        Some(-dot.hi)
    } else {
        None
    };
    if let Some(lower) = lower {
        // Nonzero dot implies both normals are nonzero. The restriction of
        // orthogonal projection between their planes has nonzero determinant.
        report.projection_regular = true;
        report.absolute_dot_lower = Some(lower);
        report.reverses_orientation = Some(dot.hi < 0.);
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn n(x: f64, y: f64, z: f64) -> [[f64; 2]; 3] {
        [[x, x], [y, y], [z, z]]
    }
    #[test]
    fn parallel_oblique_and_reflected_planes() {
        for (a, b, reflection) in [
            (n(0., 0., 1.), n(0., 0., 2.), false),
            (n(1., 2., 3.), n(0., 0., 1.), false),
            (n(1., 2., 3.), n(0., 0., -1.), true),
        ] {
            let r = certify(a, b, 1).unwrap();
            assert!(r.projection_regular);
            assert!(r.absolute_dot_lower.unwrap() > 0.);
            assert_eq!(r.reverses_orientation, Some(reflection));
        }
    }
    #[test]
    fn perpendicular_uncertain_and_exhausted_refuse() {
        for (a, b, cells) in [
            (n(1., 0., 0.), n(0., 1., 0.), 1),
            ([[-1., 1.], [0., 0.], [0., 0.]], n(1., 0., 0.), 1),
            (n(0., 0., 1.), n(0., 0., 1.), 0),
            (n(0., 0., 0.), n(0., 0., 1.), 1),
        ] {
            let r = certify(a, b, cells).unwrap();
            assert!(!r.projection_regular);
            assert!(r.absolute_dot_lower.is_none());
            assert!(r.reverses_orientation.is_none());
        }
    }
    #[test]
    fn scaled_near_perpendicular_certificates_enclose_analytic_dot() {
        let a = n(1., 0., 1e-12);
        let b = n(0., 0., 7.);
        let r = certify(a, b, 1).unwrap();
        let dot = 7e-12;
        let bounds = r.normal_dot.unwrap();
        assert!(bounds[0] <= dot && dot <= bounds[1]);
        assert!(r.projection_regular);
        assert!(certify([[1., 0.], [0., 0.], [0., 0.]], b, 1).is_err());
    }
}
