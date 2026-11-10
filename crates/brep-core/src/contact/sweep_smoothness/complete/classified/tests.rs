use super::*;
fn limits() -> Limits {
    Limits {
        incidence_steps: 1000000,
        exact_work: 16000000,
        agreement_work: 1000000,
        regularity_cells: 100000,
    }
}
#[test]
fn explicit_c0_needs_every_edge_position_link_and_face_rank_with_one_census_budget() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let before = model.clone();
    let classes = vec![Class::C0; model.edges.len()];
    let full = inspect(&model, &classes, limits()).unwrap();
    assert!(full.certified && full.vertex_links_complete);
    assert!(full.proof.is_none() && full.smooth_edges.is_empty());
    assert_eq!(full.c0_edges.len(), model.edges.len());
    let tight = Limits {
        incidence_steps: full.steps,
        agreement_work: full.agreement.as_ref().unwrap().work,
        regularity_cells: full.regularity.as_ref().unwrap().cells,
        ..limits()
    };
    assert!(inspect(&model, &classes, tight).unwrap().certified);
    for d in 0..3 {
        let mut short = tight;
        match d {
            0 => short.incidence_steps -= 1,
            1 => short.agreement_work -= 1,
            _ => short.regularity_cells -= 1,
        }
        let r = inspect(&model, &classes, short).unwrap();
        assert!(!r.certified && r.steps <= short.incidence_steps);
    }
    assert!(inspect(&model, &classes[..classes.len() - 1], limits()).is_err());
    assert_eq!(model, before);
}
#[test]
fn explicit_classes_do_not_downgrade_g2_or_skip_world_geometry_for_c0() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let g2 = inspect(&model, &vec![Class::G2; model.edges.len()], limits()).unwrap();
    assert!(!g2.certified && g2.proof.as_ref().is_some_and(|p| !p.certified));
    let mut broken = model.clone();
    broken.edges[0].curve.control_points[0][0] += 1. / 1024.;
    assert!(
        match inspect(&broken, &vec![Class::C0; broken.edges.len()], limits()) {
            Ok(r) => !r.certified,
            Err(_) => true,
        }
    );
}
