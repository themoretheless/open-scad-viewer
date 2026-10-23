//! XYZ original-law jets through third order, with one component work budget.
use super::*;
#[derive(Clone, Debug)]
pub struct ThirdReport {
    pub base: Report,
    pub third: Option<[[f64; 2]; 3]>,
}
pub fn certify_third_traversal(
    law: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ThirdReport> {
    law.validate()?;
    check(
        max_cells <= 100000 && law.control_points.iter().all(|p| p.len() == 3),
        "Invalid XYZ third-jet input",
    )?;
    let mut out = ThirdReport {
        base: Report {
            status: Status::Unresolved,
            cells: 0,
            value: None,
            first: None,
            second: None,
            single_span: false,
            reason: Some("third-component-enclosure-unresolved"),
        },
        third: None,
    };
    let mut value = [[0.; 2]; 3];
    let mut first = value;
    let mut second = value;
    let mut third = value;
    let mut single_span = true;
    for axis in 0..3 {
        let mut scalar = law.clone();
        scalar.control_points = law
            .control_points
            .iter()
            .map(|p| vec![p[axis], 0., 0.])
            .collect();
        let r = scalar_certificate::certify_third_traversal(
            &scalar,
            traversal,
            max_cells - out.base.cells,
        )?;
        out.base.cells += r.base.cells;
        if r.base.status != Status::Certified {
            out.base.reason = r.base.reason;
            return Ok(out);
        }
        value[axis] = r.base.value.unwrap();
        first[axis] = r.base.first.unwrap();
        second[axis] = r.base.second.unwrap();
        third[axis] = r.third.unwrap();
        single_span &= r.base.single_span;
    }
    out.base.status = Status::Certified;
    out.base.value = Some(value);
    out.base.first = Some(first);
    out.base.second = Some(second);
    out.base.single_span = single_span;
    out.base.reason = None;
    out.third = Some(third);
    Ok(out)
}
#[test]
fn rational_xyz_third_jet_shares_budget_and_discards_partial_components() {
    let law = Curve {
        degree: 1,
        knots: vec![2., 2., 5., 5.],
        control_points: vec![vec![0.; 3], vec![1., 2., 10.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let report = certify_third_traversal(&law, [0.25, 0.5], 100).unwrap();
    assert_eq!(report.base.status, Status::Certified);
    assert!(report.base.single_span);
    assert_eq!(report.base.cells, 6);
    for t in [0.25_f64, 0.375, 0.5] {
        for (axis, amplitude) in [1., 2., 10.].into_iter().enumerate() {
            for (range, expected) in [
                (
                    report.base.value.unwrap()[axis],
                    amplitude * 2. * t / (1. + t),
                ),
                (
                    report.base.first.unwrap()[axis],
                    amplitude * 2. / (3. * (1. + t).powi(2)),
                ),
                (
                    report.base.second.unwrap()[axis],
                    -amplitude * 4. / (9. * (1. + t).powi(3)),
                ),
                (
                    report.third.unwrap()[axis],
                    amplitude * 12. / (27. * (1. + t).powi(4)),
                ),
            ] {
                assert!(range[0] <= expected && expected <= range[1]);
            }
        }
    }
    let refused = certify_third_traversal(&law, [0.25, 0.5], 5).unwrap();
    assert_eq!(refused.base.status, Status::Unresolved);
    assert!(
        refused.base.value.is_none() && refused.base.first.is_none() && refused.third.is_none()
    );
    assert!(refused.base.cells <= 5);
}
