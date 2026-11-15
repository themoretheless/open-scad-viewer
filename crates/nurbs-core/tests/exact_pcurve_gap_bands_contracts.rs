use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurveGapBand, PcurveGapBoundary, PcurvePullbackRefusal, isolate_pcurve_seam_gap,
        },
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face(z: [f64; 3]) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 4., 4., 4.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                let u = 2. * i as f64;
                vec![vec![0., u, z[i]], vec![1., u, z[i]]]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn trace() -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    }
}

#[test]
fn isolates_two_nonbinary_bounds_around_an_interior_violation() {
    let a = face([0.; 3]);
    let b = face([0., 0.25, 0.]);
    let uv = trace();
    let width = 1e-6;
    let result = isolate_pcurve_seam_gap((&a, &uv), (&b, &uv), 0.0625, width, WORK).unwrap();
    let bands = result
        .bands
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(bands.len(), 1);
    let band = bands[0];
    for (bound, root) in [
        (band.start, 5. - 2_f64.sqrt()),
        (band.end, 5. + 2_f64.sqrt()),
    ] {
        let [lo, hi] = bound.bracket;
        assert!(lo < root && root < hi);
        assert!(hi - lo <= width);
        assert!(!bound.included);
    }
}

#[test]
fn leaves_a_single_equality_tangency_out_of_two_violation_bands() {
    let a = face([0.; 3]);
    let b = face([0.375, -0.125, 0.375]);
    let uv = trace();
    let result = isolate_pcurve_seam_gap((&a, &uv), (&b, &uv), 0.125, 1e-9, WORK).unwrap();
    let bands = result
        .bands
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    let boundary = |t, included| PcurveGapBoundary {
        bracket: [t; 2],
        included,
    };
    assert_eq!(
        bands,
        [
            PcurveGapBand {
                start: boundary(3., true),
                end: boundary(5., false)
            },
            PcurveGapBand {
                start: boundary(5., false),
                end: boundary(7., true)
            },
        ]
    );
}

#[test]
fn equality_yields_no_bands_and_unresolvable_contacts_refuse_atomically() {
    let a = face([0.; 3]);
    let equal = face([0.25; 3]);
    let b = face([0., 0.25, 0.]);
    let uv = trace();
    let result = isolate_pcurve_seam_gap((&a, &uv), (&equal, &uv), 0.25, 1e-9, WORK).unwrap();
    assert_eq!(result.bands, Some(vec![]));
    let limited = isolate_pcurve_seam_gap((&a, &uv), (&b, &uv), 0.0625, 1e-20, WORK).unwrap();
    assert!(limited.bands.is_none());
    assert_eq!(
        limited.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable))
    );
    let exhausted = isolate_pcurve_seam_gap((&a, &uv), (&b, &uv), 0.0625, 1e-9, 1).unwrap();
    assert!(exhausted.bands.is_none());
    assert_eq!(
        exhausted.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
}
