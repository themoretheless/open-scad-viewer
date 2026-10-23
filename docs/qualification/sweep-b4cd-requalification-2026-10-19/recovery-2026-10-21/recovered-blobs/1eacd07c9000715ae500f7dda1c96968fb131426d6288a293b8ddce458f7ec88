//! XYZ original-law jets through fourth order, with one component work budget.
use super::*;
#[derive(Clone, Debug)]
pub struct FourthReport {
    pub base: Report,
    pub third: Option<[[f64; 2]; 3]>,
    pub fourth: Option<[[f64; 2]; 3]>,
}
pub fn certify_fourth_traversal(
    law: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<FourthReport> {
    law.validate()?;
    check(
        max_cells <= 100000 && law.control_points.iter().all(|p| p.len() == 3),
        "Invalid XYZ fourth-jet input",
    )?;
    let mut out = FourthReport {
        base: Report {
            status: Status::Unresolved,
            cells: 0,
            value: None,
            first: None,
            second: None,
            single_span: false,
            reason: Some("fourth-component-enclosure-unresolved"),
        },
        third: None,
        fourth: None,
    };
    let mut value = [[0.; 2]; 3];
    let mut first = value;
    let mut second = value;
    let mut third = value;
    let mut fourth = value;
    let mut single_span = true;
    for axis in 0..3 {
        let mut scalar = law.clone();
        scalar.control_points = law
            .control_points
            .iter()
            .map(|p| vec![p[axis], 0., 0.])
            .collect();
        let r = scalar_certificate::certify_fourth_traversal(
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
        fourth[axis] = r.fourth.unwrap();
        single_span &= r.base.single_span;
    }
    out.base.status = Status::Certified;
    out.base.value = Some(value);
    out.base.first = Some(first);
    out.base.second = Some(second);
    out.base.single_span = single_span;
    out.base.reason = None;
    out.third = Some(third);
    out.fourth = Some(fourth);
    Ok(out)
}

#[test]
fn rational_xyz_fourth_jet_refuses_partial_components(){
    let law=Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![0.;3],vec![1.,2.,10.]],weights:vec![1.,2.],periodic:false};
    let r=certify_fourth_traversal(&law,[0.25,0.5],100).unwrap();
    assert_eq!(r.base.status,Status::Certified);assert_eq!(r.base.cells,9);
    for t in [0.25_f64,0.375,0.5] {for (k,a) in [1.,2.,10.].into_iter().enumerate(){
        let expected=-48.*a/(3_f64.powi(4)*(1.+t).powi(5));let bound=r.fourth.unwrap()[k];
        assert!(bound[0]<=expected&&expected<=bound[1]);
    }}
    let short=certify_fourth_traversal(&law,[0.25,0.5],r.base.cells-1).unwrap();
    assert_eq!(short.base.status,Status::Unresolved);
    assert!(short.base.value.is_none()&&short.third.is_none()&&short.fourth.is_none());
}
