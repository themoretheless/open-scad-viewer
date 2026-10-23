use brep_core::profile_certificate as p;
use num_rational::BigRational as Q;
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as trim, Piece, Report},
};
fn area(rings: p::ExactRegionRings) -> Q {
    rings
        .into_iter()
        .map(|r| {
            let r = r
                .into_iter()
                .map(|p| p.map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap())))
                .collect::<Vec<_>>();
            (0..r.len())
                .map(|i| &r[i][0] * &r[(i + 1) % r.len()][1] - &r[i][1] * &r[(i + 1) % r.len()][0])
                .sum::<Q>()
                / Q::from_integer(2.into())
        })
        .sum()
}
fn limits() -> trim::ChartAssemblyLimits {
    trim::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 64,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    }
}
#[test]
fn actual_periodic_source_crossings_are_regularized_before_nonzero_selection() {
    for reverse in [false, true] {
        let a = Curve::from_polyline(vec![
            vec![0.125, 0.125],
            vec![0.625, 0.125],
            vec![0.625, 0.625],
            vec![0.125, 0.625],
            vec![0.125, 0.125],
        ])
        .unwrap();
        let b = Curve::from_polyline(vec![
            vec![0.375, 0.375],
            vec![0.875, 0.375],
            vec![0.875, 0.875],
            vec![0.375, 0.875],
            vec![0.375, 0.375],
        ])
        .unwrap();
        let b = if reverse { b.reverse().unwrap() } else { b };
        let sources = vec![a, b];
        let reports = sources
            .iter()
            .map(|c| Report {
                pieces: vec![Piece {
                    curve: c.clone(),
                    source_domain: c.domain(),
                    translation: [0.; 2],
                    tile: [0; 2],
                }],
                candidate_hulls_contained: false,
            })
            .collect::<Vec<_>>();
        let owner = p::regularize_linear_periodic_contacts(
            &sources,
            &reports,
            &[[0; 2]; 2],
            [[0., 1.]; 2],
            [0.25, 0.25],
            limits(),
            1_000_000,
        )
        .unwrap();
        owner.reverify_source().unwrap();
        let expected = if reverse {
            Q::new(3.into(), 8.into())
        } else {
            Q::new(7.into(), 16.into())
        };
        assert_eq!(area(owner.regions().exact_regions()), expected);
        assert_eq!(owner.regions().groups().len(), if reverse { 2 } else { 1 });
    }
}
#[test]
fn periodic_contact_regions_and_image_owners_retain_corrected_corner_fill() {
    let source = Curve::from_polyline(vec![
        vec![1.25, 1.],
        vec![1., 1.25],
        vec![0.75, 1.],
        vec![1., 0.75],
        vec![1.25, 1.],
    ])
    .unwrap();
    let report = trim::split_candidates(
        &source,
        [[0., 1.]; 2],
        &[1., 2., 3.],
        &[[1, 1], [0, 1], [0, 0], [1, 0]],
        16,
    )
    .unwrap();
    let owner = p::regularize_linear_periodic_contacts(
        &[source.clone()],
        &[report.clone()],
        &[[0; 2]],
        [[0., 1.]; 2],
        [0.5, 0.5],
        limits(),
        1_000_000,
    )
    .unwrap();
    owner.reverify_source().unwrap();
    assert_eq!(owner.original().calibration().unwrap().offset, Some(1));
    assert_eq!(owner.regions().groups().len(), 4);
    assert_eq!(
        area(owner.regions().exact_regions()),
        Q::new(1.into(), 8.into())
    );
    let mapped = p::regularize_multispan_periodic_winding(
        &[source.clone()],
        &[report.clone()],
        &[[0; 2]],
        [[0., 1.]; 2],
        [0.5, 0.5],
        limits(),
        &[],
        1e-7,
        1_000_000,
    )
    .unwrap();
    mapped.reverify_source().unwrap();
    assert_eq!(mapped.regions().components().len(), 4);
    assert_eq!(mapped.graph(), mapped.original().graph().unwrap());
    let clock = p::regularize_clock_mapped_periodic_winding(
        &[source],
        &[report],
        &[[0; 2]],
        [[0., 1.]; 2],
        [0.5, 0.5],
        limits(),
        &[],
        1e-7,
        1_000_000,
    )
    .unwrap();
    clock.reverify_source().unwrap();
    assert_eq!(clock.regions().components().len(), 4);
}
