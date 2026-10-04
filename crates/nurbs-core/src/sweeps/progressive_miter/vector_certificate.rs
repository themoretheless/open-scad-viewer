//! Original-coefficient XYZ law jets with a shared component work budget.
//! These law enclosures alone do not certify a frame or sweep geometry.
use super::scalar_certificate::{self, Status};
use crate::{Result, check, curve::Curve};
#[derive(Clone, Debug)]
pub struct ValuesReport {
    pub cells: usize,
    pub value: Option<[[f64; 2]; 3]>,
}
/// Point-safe values with conservative charging of every active scalar span.
pub fn certify_values_traversal(
    law: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
    require_positive: bool,
) -> Result<ValuesReport> {
    law.validate()?;
    check(
        max_cells <= 100000 && law.control_points.iter().all(|p| p.len() == 3),
        "Invalid vector value certificate input",
    )?;
    let mut out = ValuesReport {
        cells: 0,
        value: None,
    };
    let mut values = [[0.; 2]; 3];
    let charge = (law.degree..law.control_points.len())
        .filter(|&i| law.knots[i] < law.knots[i + 1])
        .count();
    for axis in 0..3 {
        if charge > max_cells - out.cells {
            return Ok(out);
        }
        let mut scalar = law.clone();
        scalar.control_points = law
            .control_points
            .iter()
            .map(|p| vec![p[axis], 0., 0.])
            .collect();
        let result = scalar_certificate::value_traversal(&scalar, traversal, charge)?;
        out.cells += charge;
        let Some(v) = result else {
            return Ok(out);
        };
        if require_positive && v[0] <= 0. {
            return Ok(out);
        }
        values[axis] = v;
    }
    out.value = Some(values);
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    pub value: Option<[[f64; 2]; 3]>,
    pub first: Option<[[f64; 2]; 3]>,
    pub second: Option<[[f64; 2]; 3]>,
    pub single_span: bool,
    pub reason: Option<&'static str>,
}

/// Normalize each law's own knot domain to traversal [0,1]. Component
/// projection copies original binary64 values without evaluation or trimming.
/// Derivatives retain the authored knot parameter convention of scalar jets.
pub fn certify_traversal(
    law: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
    require_positive: bool,
) -> Result<Report> {
    law.validate()?;
    check(
        law.control_points.iter().all(|p| p.len() == 3),
        "Vector law requires XYZ controls",
    )?;
    check(max_cells <= 100000, "Vector law work exceeds100000 cells")?;
    let mut out = Report {
        status: Status::Unresolved,
        cells: 0,
        value: None,
        first: None,
        second: None,
        single_span: false,
        reason: Some("component-enclosure-unresolved"),
    };
    let mut values = [[0.; 2]; 3];
    let mut first = values;
    let mut second = values;
    let mut single_span = true;
    for axis in 0..3 {
        let mut scalar = law.clone();
        scalar.control_points = law
            .control_points
            .iter()
            .map(|p| vec![p[axis], 0., 0.])
            .collect();
        let r = scalar_certificate::certify_traversal(&scalar, traversal, max_cells - out.cells)?;
        out.cells += r.cells;
        if r.status != Status::Certified {
            out.reason = r.reason;
            return Ok(out);
        }
        values[axis] = r.value.unwrap();
        if require_positive && values[axis][0] <= 0. {
            out.reason = Some("positive-axis-scale-unproved");
            return Ok(out);
        }
        first[axis] = r.first.unwrap();
        second[axis] = r.second.unwrap();
        single_span &= r.single_span;
    }
    out.status = Status::Certified;
    out.value = Some(values);
    out.first = Some(first);
    out.second = Some(second);
    out.single_span = single_span;
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn law() -> Curve {
        Curve {
            degree: 1,
            knots: vec![2., 2., 3., 4., 4.],
            control_points: vec![vec![1., 2., 3.], vec![2., 3., 4.], vec![3., 4., 5.]],
            weights: vec![1., 0.5, 2.],
            periodic: false,
        }
    }
    #[test]
    fn shared_budget_discards_partial_axis_hulls() {
        let law = law();
        let positive = certify_traversal(&law, [0., 1.], 6, true).unwrap();
        assert_eq!(positive.status, Status::Certified);
        assert_eq!(positive.cells, 6);
        assert!(!positive.single_span);
        for budget in [0, 2, 5] {
            let refused = certify_traversal(&law, [0., 1.], budget, true).unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.cells <= budget);
            assert!(refused.value.is_none() && refused.first.is_none() && refused.second.is_none());
        }
        for i in 0..=20 {
            let e = law.evaluate(2. + i as f64 / 10.).unwrap();
            for k in 0..3 {
                let inside = |range: [f64; 2], x: f64| range[0] <= x && x <= range[1];
                assert!(inside(positive.value.unwrap()[k], e.point[k]));
                if let Some(d1) = &e.d1 {
                    assert!(inside(positive.first.unwrap()[k], d1[k]));
                }
                if let Some(d2) = &e.d2 {
                    assert!(inside(positive.second.unwrap()[k], d2[k]));
                }
            }
        }
    }
    #[test]
    fn center_offsets_can_be_negative_but_axis_scale_cannot() {
        let mut law = law();
        for p in &mut law.control_points {
            p[1] = -p[1];
        }
        assert_eq!(
            certify_traversal(&law, [0., 1.], 6, false).unwrap().status,
            Status::Certified
        );
        let refused = certify_traversal(&law, [0., 1.], 6, true).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert_eq!(refused.reason, Some("positive-axis-scale-unproved"));
        assert!(refused.value.is_none());
        assert!(certify_traversal(&law, [-0.1, 1.], 6, false).is_err());
        assert!(certify_traversal(&law, [0., 1.], 100001, false).is_err());
    }
}
