use brep_core::planar_cap::{
    SectionSheetOptions, build_section_sheets, build_section_sheets_with_support_tolerance,
};
use nurbs_core::curve::Curve;

fn options() -> SectionSheetOptions {
    SectionSheetOptions {
        tolerance: 1e-8,
        max_exact_work: 1_000_000,
        max_verification_cells: 10_000,
    }
}

fn ring(lo: [f64; 2], hi: [f64; 2], z: impl Fn(f64) -> f64) -> Vec<Curve> {
    let [x0, y0] = lo;
    let [x1, y1] = hi;
    let points = [[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]];
    points
        .windows(2)
        .map(|pair| {
            Curve::from_polyline(pair.iter().map(|&[x, y]| vec![x, y, z(x)]).collect()).unwrap()
        })
        .collect()
}

#[test]
fn authors_oblique_cap_sheets_with_holes_and_preserves_input_edge_clocks() {
    let outer = ring([0., 0.], [10., 10.], |x| x);
    let hole = ring([2., 2.], [4., 4.], |x| x);
    let separate = ring([20., 20.], [22., 22.], |x| x);
    let report =
        build_section_sheets(&[&hole, &separate, &outer], [-1., 0., 1.], 0., options()).unwrap();
    assert_eq!(report.face_rings, [vec![1], vec![2, 0]]);
    assert_eq!(report.model.faces.len(), 2);
    assert_eq!(report.model.faces[1].holes.len(), 1);
    assert_eq!(report.model.edges.len(), 12);
    assert!(report.model.shells.iter().all(|shell| !shell.closed));
    assert!(
        report
            .model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .any(|c| c.reversed)
    );
    for edge in &report.model.edges {
        assert!(outer.iter().chain(&hole).chain(&separate).any(|curve| {
            curve.control_points == edge.curve.control_points
                && curve.knots == edge.curve.knots
                && curve.weights == edge.curve.weights
        }));
    }
    report.model.validate().unwrap();
    let limited = build_section_sheets(
        &[&hole, &separate, &outer],
        [-1., 0., 1.],
        0.,
        SectionSheetOptions {
            max_exact_work: report.exact_work - 1,
            ..options()
        },
    );
    assert!(limited.is_err());
}

#[test]
fn refuses_a_support_that_cannot_be_lifted_without_rounding() {
    let outline = ring([0., 0.], [1., 1.], |_| 1. / 3.);
    let report = build_section_sheets(&[&outline], [0., 0., 3.], 1., options());
    assert!(report.is_err());
}

#[test]
fn rounds_an_oblique_nonbinary_support_without_warping_it() {
    use num_rational::BigRational as Q;
    let mut wire = ring([0., 0.], [1., 1.], |_| 0.);
    for point in wire.iter_mut().flat_map(|curve| &mut curve.control_points) {
        point[2] = (point[0] + point[1] + 1.) / 3.;
    }
    let normal = [-1., -1., 3.];
    let report =
        build_section_sheets_with_support_tolerance(&[&wire], normal, 1., options(), 1e-14)
            .unwrap();
    let [error] = report.support_error_upper.as_slice() else {
        panic!("{:?}", report.support_error_upper);
    };
    assert!(error > &Q::from_integer(0.into()));
    assert!(error <= &Q::from_float(1e-14).unwrap());
    let cap = &report.sheet.model.faces[0].surface;
    let z = |u: usize, v: usize| Q::from_float(cap.control_points[u][v][2]).unwrap();
    assert_eq!(z(0, 0) + z(1, 1), z(0, 1) + z(1, 0));
    for point in cap.control_points.iter().flatten() {
        let exact = (Q::from_float(point[0]).unwrap()
            + Q::from_float(point[1]).unwrap()
            + Q::from_integer(1.into()))
            / Q::from_integer(3.into());
        let gap = Q::from_float(point[2]).unwrap() - exact;
        assert!(gap.pow(2) <= error.pow(2));
    }
    report.sheet.model.validate().unwrap();
    assert!(
        build_section_sheets_with_support_tolerance(&[&wire], normal, 1., options(), 0.).is_err()
    );
}

#[test]
fn supports_zero_tolerance_for_an_exactly_storable_plane() {
    use num_rational::BigRational as Q;
    let wire = ring([0., 0.], [1., 1.], |_| 0.5);
    let report =
        build_section_sheets_with_support_tolerance(&[&wire], [0., 0., 2.], 1., options(), 0.)
            .unwrap();
    assert_eq!(report.support_error_upper, [Q::from_integer(0.into())]);
    assert_eq!(report.sheet.model.faces.len(), 1);
}
