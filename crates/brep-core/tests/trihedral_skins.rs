use brep_core::region_replacement::{self as region, EdgeRadius, StripLimits};
fn request(m: &brep_core::Model) -> Vec<EdgeRadius> {
    m.edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect()
}
fn limits() -> StripLimits {
    StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    }
}
#[test]
fn independent_radii_create_root_trimmed_g2_strip_faces() {
    let m = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = m.clone();
    let request = request(&m);
    let out = region::build_trihedral_strips(&m, 0, 0, &request, 0.1, 2., limits()).unwrap();
    assert_eq!(m, before);
    assert_eq!(out.strips.len(), 3);
    let rim = region::build_trihedral_rim(&out, limits().tolerances).unwrap();
    let collars=region::prepare_trihedral_collars(&m,&out,limits().tolerances,1e-8,0.001,100000,20000000,1000000).unwrap();
    assert_eq!(collars.boundaries.len(),6);
    assert!(collars.cells<=100000 && collars.coefficient_work<=20000000);
    assert_eq!(rim.connectors.len(), 3);
    assert_eq!(rim.ordered.len(), 6);
    let simplicity = region::inspect_spatial_rim(&rim.ordered, 1e-7, 30000).unwrap();
    assert!(simplicity.proven_simple,"injective {:?}, pairs {:?}",simplicity.injective,simplicity.pairs);
    for i in 0..6 {
        assert_eq!(rim.ordered[i].control_points.last(),rim.ordered[(i+1)%6].control_points.first());
    }
    let supports=region::inspect_rim_supports(&m,&rim,1e-8,100000).unwrap();
    assert_eq!(supports.pcurves.len(),3);
    assert!(supports.cells<=100000);
    assert!(region::inspect_rim_supports(&m,&rim,1e-8,6).is_err());
    let mut ports = std::collections::BTreeSet::new();
    for connector in &rim.connectors {
        for (end, &(strip, side)) in connector.ports.iter().enumerate() {
            assert!(ports.insert((strip, side)));
            let section = &rim.cross_sections[strip];
            let point = &section.control_points[if side == 0 {0} else {section.control_points.len()-1}];
            assert_eq!(&connector.curve.control_points[if end == 0 {0} else {5}], point);
        }
    }
    assert_eq!(ports.len(), 6);
    assert_eq!(out.separation.len(), 3);
    for pair in &out.separation {
        assert!(pair.strips[0] < pair.strips[1]);
        assert!(pair.distance.lower_bound_mm > limits().tolerances[0]);
        if let Some(upper) = pair.distance.upper_bound_mm {
            assert!(pair.distance.lower_bound_mm <= upper);
        }
    }
    assert!(out.cells <= 300000 && out.exact_work <= 1000000 && out.coefficient_work <= 2000000);
    for (s, r) in out.strips.iter().zip(&request) {
        assert_eq!(s.source_edge, r.edge);
        assert_eq!(s.radius_mm, r.radius_mm);
        assert_eq!(s.patch.model.faces.len(), 1);
        assert_eq!(s.patch.model.edges.len(), 4);
        assert_eq!(s.patch.model.vertices.len(), 4);
        assert!(s.patch.model.bodies.is_empty() && !s.patch.model.shells[0].closed);
        assert!(s.injectivity.certified);
        assert_eq!(s.g2.len(), 2);
        for p in &s.g2 {
            assert!(
                p.positions.is_some()
                    && p.report.positions.proven
                    && p.report.geometric.regularity_proven
            );
            assert_eq!(
                p.report.geometric.decision,
                nurbs_core::surface_join::Decision::WithinTolerance
            );
        }
        for c in &s.contacts {
            assert!(
                s.original_law_domain[0] < c.curve.domain()[0]
                    && c.curve.domain()[1] < s.original_law_domain[1]
            );
        }
        assert_eq!(
            s.radius.decision,
            brep_core::fillet_radius::Decision::WithinTolerance
        );
        let wrong = brep_core::fillet_radius::inspect_contacts_in_domain(
            s.support_faces.map(|f| &m.faces[f].surface),
            &s.patch.model.faces[0].surface,
            [&s.contacts[0], &s.contacts[1]],
            &nurbs_core::offset::fillet_surfaces::RadiusLaw::Constant(r.radius_mm * 1.1),
            s.original_law_domain,
            s.sides,
            1e-7,
            100000,
            20000,
        )
        .unwrap();
        assert_eq!(
            wrong.decision,
            brep_core::fillet_radius::Decision::ExceedsTolerance
        );
    }
    if let Some(directory) = std::env::var_os("OSV_TRIHEDRAL_SKIN_EVIDENCE") {
        let d = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&d).unwrap();
        for (i, s) in out.strips.iter().enumerate() {
            std::fs::write(d.join(format!("skin-{i}.json")),value_codec::to_string(&value_codec::json!({"source":m,"model":s.patch.model,"sourceEdge":s.source_edge,"supports":s.support_faces,"radiusMm":s.radius_mm,"lawDomain":s.original_law_domain,"targetStartMm":out.plan.edges[i].distance_mm,"spanMm":2.,"sides":s.sides,"contactEdges":[s.patch.edges[3],s.patch.edges[1]],"endOrder":s.end_order,"g2Proven":true,"rollingCenterRadiusProven":true,"circularSectionRadiusProven":false,"closedBody":false})).unwrap()).unwrap();
            std::fs::write(
                d.join(format!("skin-{i}.step")),
                brep_core::export_step_v9(&s.patch.model).unwrap().0,
            )
            .unwrap();
        }
        std::fs::write(d.join("work.json"),value_codec::to_string(&value_codec::json!({"cells":out.cells,"exactWork":out.exact_work,"coefficientWork":out.coefficient_work,"separation":out.separation.iter().map(|p|value_codec::json!({"strips":p.strips,"lowerMm":p.distance.lower_bound_mm,"upperMm":p.distance.upper_bound_mm,"converged":p.distance.converged,"cells":p.distance.cells,"domainCells":p.distance.domain_cells})).collect::<Vec<_>>()})).unwrap()).unwrap();
    }
}
#[test]
fn invalid_span_or_work_returns_no_partial_geometry() {
    let m = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = m.clone();
    let r = request(&m);
    assert!(region::build_trihedral_strips(&m, 0, 0, &r, 0.1, 20., limits()).is_err());
    let mut low = limits();
    low.max_cells = 1;
    assert!(region::build_trihedral_strips(&m, 0, 0, &r, 0.1, 2., low).is_err());
    assert_eq!(m, before);
}

#[test]
fn modified_original_surface_cannot_reuse_corner_boundary_proofs() {
    let m=brep_core::cuboid([0.;3],[10.;3]).unwrap();
    let mut strips=region::build_trihedral_strips(&m,0,0,&request(&m),0.1,2.,limits()).unwrap();
    for p in strips.strips[0].patch.model.faces[0].surface.control_points.iter_mut().flatten() {
        p[2]+=0.01;
    }
    assert!(region::prepare_trihedral_collars(&m,&strips,limits().tolerances,1e-8,0.001,100000,20000000,1000000).is_err());
}
