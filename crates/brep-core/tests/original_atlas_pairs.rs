use brep_core::profile_certificate as p;
use nurbs_core::{
    curve::Curve,
    surface::{self, Surface},
};
use std::sync::Arc;
fn planar() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 3],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn atlas(surface: &Surface, wrapped: [bool; 2]) -> Arc<p::OwnedOriginalInjectiveChartAtlas> {
    let chart = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let lo = [chart[0][0], chart[1][0]];
    let hi = [chart[0][1], chart[1][1]];
    let v = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo];
    let wire = v
        .windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect();
    let regions = Arc::new(
        p::assemble_original_filled_regions(
            &[wire],
            &[1],
            1e-12,
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let vertices = Arc::new(
        p::own_original_chart_vertices(
            regions.owner().cycles().graph().clone(),
            chart,
            wrapped,
            Default::default(),
        )
        .unwrap(),
    );
    let dictionary =
        Arc::new(p::own_original_seam_dictionary(regions, vertices, Default::default()).unwrap());
    let seams = Arc::new(p::bind_original_seams_to_surface(dictionary, surface).unwrap());
    let topology = Arc::new(p::own_original_periodic_topology(seams).unwrap());
    let coverage =
        Arc::new(p::own_original_trim_chart_coverage(topology, Default::default()).unwrap());
    let regularity = Arc::new(p::own_original_regular_trim_surface(coverage, 4096).unwrap());
    Arc::new(p::own_original_injective_chart_atlas(regularity, Default::default()).unwrap())
}
fn doubled() -> Surface {
    let c = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    surface::extrude(&c, [0., 0., 1.]).unwrap()
}
#[test]
fn complete_adjacent_chart_pair_uses_one_common_original_rectangle_proof() {
    let a = atlas(&planar(), [false, false]);
    assert_eq!(a.charts().len(), 2);
    let c = Arc::new(p::own_original_atlas_pairs(a, Default::default()).unwrap());
    c.reverify_source().unwrap();
    assert_eq!(c.pairs().len(), 1);
    assert_eq!(c.pairs()[0].charts, [0, 1]);
    assert!(matches!(
        c.pairs()[0].proof,
        p::OriginalAtlasPairProof::CommonOriginalRectangle { .. }
    ));
    assert_eq!(c.unresolved_pairs(), 0);
    p::own_original_global_chart_embedding(c)
        .unwrap()
        .reverify_source()
        .unwrap();
}
#[test]
fn complete_pair_census_prevents_double_cover_from_inheriting_local_success() {
    let a = atlas(&doubled(), [true, false]);
    let c = Arc::new(p::own_original_atlas_pairs(a, Default::default()).unwrap());
    c.reverify_source().unwrap();
    assert_eq!(c.pairs().len(), 28);
    assert!(c.unresolved_pairs() > 0);
    assert!(c.pairs().iter().any(|p| matches!(
        p.proof,
        p::OriginalAtlasPairProof::SeparatedAxis { .. }
            | p::OriginalAtlasPairProof::SeparatedControlGap { .. }
    )));
    assert!(p::own_original_global_chart_embedding(c).is_err());
}
#[test]
fn zero_union_work_stays_unresolved_and_small_pair_census_refuses() {
    let a = atlas(&planar(), [false, false]);
    let c = Arc::new(
        p::own_original_atlas_pairs(
            a.clone(),
            p::OriginalAtlasPairLimits {
                max_span_work: 0,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    assert_eq!(c.pairs().len(), 1);
    assert_eq!(c.unresolved_pairs(), 1);
    assert!(p::own_original_global_chart_embedding(c).is_err());
    assert!(
        p::own_original_atlas_pairs(
            a,
            p::OriginalAtlasPairLimits {
                max_pairs: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn public_control_gap_requires_actual_valid_sources_and_domains() {
    let s = planar();
    assert!(
        nurbs_core::surface_distance::verify_rectangle_separation_gap(
            &s,
            &s,
            [[[0., 2.], [0., 1.]], [[0., 0.5], [0., 1.]]]
        )
        .is_err()
    );
    let mut broken = s.clone();
    broken.weights.clear();
    assert!(
        nurbs_core::surface_distance::verify_rectangle_separation_gap(
            &broken,
            &s,
            [[[0., 0.5], [0., 1.]]; 2]
        )
        .is_err()
    );
}
#[test]
fn outward_control_gap_separates_actual_oblique_images_with_overlapping_aabbs() {
    let wall = |offset: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., offset], vec![0., 1., 1. + offset]],
            vec![vec![1., 0., 1. + offset], vec![1., 1., 2. + offset]],
        ],
        weights: vec![vec![1., 1.], vec![0.5, 0.5]],
        periodic_u: false,
        periodic_v: false,
    };
    let a = wall(0.);
    let b = wall(0.125);
    let d = [[0., 1.], [0., 1.]];
    let aa = nurbs_core::surface_distance::rectangle_bounds(&a, d).unwrap();
    let bb = nurbs_core::surface_distance::rectangle_bounds(&b, d).unwrap();
    assert!((0..3).all(|k| aa[k][0] <= bb[k][1] && bb[k][0] <= aa[k][1]));
    assert!(
        nurbs_core::surface_distance::verify_rectangle_separation_gap(&a, &b, [d, d])
            .unwrap()
            .unwrap()
            > 0.
    );
    assert_eq!(
        nurbs_core::surface_distance::verify_rectangle_separation_gap(&a, &a, [d, d]).unwrap(),
        Some(0.)
    );
}
#[test]
fn complete_rational_torus_pair_census_proves_periodic_quotient_embedding() {
    let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let a = atlas(&surface, [true, true]);
    let n = a.charts().len();
    let c = Arc::new(
        p::own_original_atlas_pairs(
            a,
            p::OriginalAtlasPairLimits {
                max_span_work: 1000000,
                refinement_depth: 4,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    println!(
        "torus charts={n}, pairs={}, unresolved={}, work={}, glued_roots={}, subdivisions={}",
        c.pairs().len(),
        c.unresolved_pairs(),
        c.span_work(),
        c.pairs()
            .iter()
            .filter(|p| matches!(
                p.proof,
                p::OriginalAtlasPairProof::SeamGluedRectangle { .. }
            ))
            .count(),
        c.subdivision_cells()
    );
    assert_eq!(c.pairs().len(), n * (n - 1) / 2);
    assert!(c.pairs().iter().any(|p| matches!(
        p.proof,
        p::OriginalAtlasPairProof::SeamGluedRectangle { .. }
    )));
    for pair in c.pairs().iter().filter(|p| !p.proof.is_proven()).take(16) {
        println!(
            "unresolved {:?}: {:?} / {:?}",
            pair.charts,
            c.atlas().charts()[pair.charts[0]].domain,
            c.atlas().charts()[pair.charts[1]].domain
        );
    }
    assert!(c.subdivision_cells() > 0);
    assert!(
        c.pairs()
            .iter()
            .any(|p| matches!(p.proof, p::OriginalAtlasPairProof::Subdivided { .. }))
    );
    assert_eq!(c.unresolved_pairs(), 0);
    let embedding = Arc::new(p::own_original_global_chart_embedding(c).unwrap());
    let physical = p::own_original_physical_faces(embedding).unwrap();
    assert_eq!(physical.faces().len(), 1);
    assert!(physical.faces()[0].boundary_cycles.is_empty());
    assert_eq!(physical.faces()[0].euler_characteristic, 0);
    assert_eq!(physical.vertices().len(), 1);
    assert_eq!(physical.edges().len(), 2);
    assert!(
        physical
            .edges()
            .iter()
            .all(|e| e.vertices == [0, 0] && e.uses.len() == 2 && e.traces.len() == 2)
    );
}
#[test]
fn complete_piecewise_regular_cylinder_glues_crease_shores_without_claiming_g1() {
    let curve = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let surface = surface::extrude(&curve, [0., 0., 1.]).unwrap();
    let a = atlas(&surface, [true, false]);
    assert!(!a.regularity().record().interior_basis_c1);
    let c = Arc::new(p::own_original_atlas_pairs(a, Default::default()).unwrap());
    assert_eq!(c.pairs().len(), 6);
    assert_eq!(c.unresolved_pairs(), 0);
    assert!(c.pairs().iter().any(|p| matches!(
        p.proof,
        p::OriginalAtlasPairProof::SeamGluedRectangle { .. }
    )));
    p::own_original_global_chart_embedding(c).unwrap();
}
