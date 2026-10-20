use brep_core::profile_certificate::regularize_exact_periodic_contacts as regularize;
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
};
fn fixture() -> (Vec<Curve>, Vec<Report>, p::ChartAssemblyLimits) {
    let src = [-1., 1.]
        .into_iter()
        .map(|sign| Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0.5, 0.5],
                vec![0.5 + sign * 0.25, 0.5],
                vec![0.5, 0.5 + sign * 0.25],
                vec![0.5, 0.5],
            ],
            weights: vec![1.; 4],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let parts = src
        .iter()
        .map(|c| Report {
            pieces: vec![Piece {
                source_domain: c.domain(),
                curve: c.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        })
        .collect::<Vec<_>>();
    let limits = p::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 32,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    };
    (src, parts, limits)
}
#[test]
fn exact_nonlinear_original_contact_functions_reach_owned_nonzero_regions() {
    let (src, parts, limits) = fixture();
    let owned = regularize(
        &src,
        &parts,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits,
        1e-7,
        1_000_000,
    )
    .unwrap();
    owned.reverify_source().unwrap();
    assert_eq!(
        owned
            .regions()
            .iter()
            .map(|r| r.components().len())
            .sum::<usize>(),
        2
    );
    assert_eq!(owned.chart().contact_ownership().unwrap().contacts.len(), 1);
    let mut damaged = parts;
    damaged[0].pieces[0].curve.control_points[1][0] = 0.25_f64.next_up();
    assert!(
        regularize(
            &src,
            &damaged,
            &[[0, 0]; 2],
            [[0., 1.]; 2],
            [0.53, 0.53],
            limits,
            1e-7,
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn rational_contact_multiplicity_and_reversal_preserve_nonzero_components() {
    let (mut src, _, limits) = fixture();
    for c in &mut src {
        c.weights = vec![1., 2., 3., 1.];
    }
    // Negative fill is independently normalized; repeated original curves
    // retain multiplicity in the dictionary and never duplicate contact rays.
    for c in &mut src {
        c.control_points.reverse();
        c.weights.reverse();
    }
    src.extend(src.clone());
    let parts = src
        .iter()
        .map(|c| Report {
            pieces: vec![Piece {
                source_domain: c.domain(),
                curve: c.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        })
        .collect::<Vec<_>>();
    let owned = regularize(
        &src,
        &parts,
        &vec![[0, 0]; src.len()],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits,
        1e-7,
        1_000_000,
    )
    .unwrap();
    owned.reverify_source().unwrap();
    assert_eq!(owned.regions().len(), 2);
    assert!(
        owned
            .regions()
            .iter()
            .all(|r| r.coefficients() == [2] && r.components().len() == 1)
    );
}
#[test]
fn overlapping_interiors_cannot_inherit_endpoint_only_admission() {
    let (mut src, _, limits) = fixture();
    // These loops leave their shared endpoint along distinct but interlaced
    // directions; their interiors overlap. Exact source identity is insufficient.
    src[1].control_points[1] = vec![0.25, 0.625];
    src[1].control_points[2] = vec![0.625, 0.25];
    let parts = src
        .iter()
        .map(|c| Report {
            pieces: vec![Piece {
                source_domain: c.domain(),
                curve: c.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        })
        .collect::<Vec<_>>();
    assert!(
        regularize(
            &src,
            &parts,
            &[[0, 0]; 2],
            [[0., 1.]; 2],
            [0.53, 0.53],
            limits,
            1e-7,
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn complete_opposite_original_functions_cancel_without_fabricated_contacts() {
    let (source, _, limits) = fixture();
    let mut reverse = source[0].clone();
    reverse.control_points.reverse();
    let src = vec![source[0].clone(), reverse];
    let parts = src
        .iter()
        .map(|c| Report {
            pieces: vec![Piece {
                source_domain: c.domain(),
                curve: c.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        })
        .collect::<Vec<_>>();
    let owned = regularize(
        &src,
        &parts,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits,
        1e-7,
        1_000_000,
    )
    .unwrap();
    owned.reverify_source().unwrap();
    assert!(owned.regions().is_empty());
    assert!(owned.chart().graph().unwrap().edges.is_empty());
    assert!(owned.chart().contact_ownership().is_none());
}
