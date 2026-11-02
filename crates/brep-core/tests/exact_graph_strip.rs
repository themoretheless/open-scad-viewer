#[path = "../examples/support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::exact_graph_strip as strip;
use nurbs_core::{
    curve::Curve,
    original_graph_transition::{self as graph, Budget, Patch},
};
fn panels()->Vec<Patch>{panels_profile(false)}
fn panels_profile(quadratic_core:bool) -> Vec<Patch> {
    let (source, request, _, _) = fixture::fixture();
    let plan =
        brep_core::region_replacement::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let supports = plan.edges[0].supports.map(|i| &source.faces[i].surface);
    let mut budget = Budget::new(1_000_000).unwrap();
    (0..2)
        .map(|i| {
            let width = Curve {
                degree: 1,
                knots: vec![i as f64, i as f64, i as f64 + 1., i as f64 + 1.],
                control_points: vec![vec![0.25, 0., 0.]; 2],
                weights: vec![1.; 2],
                periodic: false,
            };
            if quadratic_core{graph::build_quadratic_core_g2(supports,&width,&mut budget).unwrap()}else{graph::build(supports, &width, &mut budget).unwrap()}
        })
        .collect()
}
#[test]
fn exact_transition_faces_share_canonical_edges_without_coordinate_rounding() {
    let mut budget = Budget::new(1_000_000).unwrap();
    let owned = strip::assemble(panels(), 1e-7, &mut budget).unwrap();
    let m = owned.topology();
    assert_eq!((m.vertices.len(), m.edges.len(), m.faces.len()), (6, 7, 2));
    assert!(!m.shells[0].closed && m.bodies.is_empty());
    assert_eq!(owned.geometry().boundary_uses, 8);
    assert_eq!(owned.geometry().open_edges.len(), 6);
    assert!(
        owned.joins()[0].within
            && owned
                .fields()
                .iter()
                .all(|r| r.absolute_curvature_upper.unwrap() <= 10.)
    );
    assert_eq!(m.loops[0].coedges[1].edge, m.loops[1].coedges[3].edge);
    assert_ne!(
        m.loops[0].coedges[1].reversed,
        m.loops[1].coedges[3].reversed
    );
    let again = strip::inspect_geometry(m, &mut budget).unwrap();
    assert_eq!(again.boundary_uses, 8);
    assert!(budget.used() <= 1_000_000);
    let mut bad = m.clone();
    bad.vertices[0].point[2] += num_rational::BigRational::from_float(2f64.powi(-100)).unwrap();
    assert!(strip::inspect_geometry(&bad, &mut budget).is_err());
    let mut bad = m.clone();
    bad.loops[0].coedges[0].pcurve = m.loops[0].coedges[2].pcurve.clone();
    assert!(strip::inspect_geometry(&bad, &mut budget).is_err());
    let mut bad = m.clone();
    bad.shells[0].closed = true;
    assert!(strip::inspect_geometry(&bad, &mut budget).is_err());
    let mut bad = m.clone();
    bad.loops[0].coedges[0] = bad.loops[0].coedges[0]
        .clone()
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [0., 0.5],
            pcurve_parameter: [0., 1.],
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    assert!(strip::inspect_geometry(&bad, &mut budget).is_err());
    assert_eq!(m.vertices.len(), 6);
    assert!(m.bodies.is_empty());
}
#[test]
fn insufficient_work_never_publishes_an_exact_strip() {
    assert!(strip::assemble(panels(), 1e-7, &mut Budget::new(0).unwrap()).is_err());
}
#[test]
fn shared_edge_splits_keep_non_dyadic_root_ends_on_both_faces() {
    use num_rational::BigRational as Q;
    let mut budget = Budget::new(1_000_000).unwrap();
    let mut owned = strip::assemble(panels(), 1e-7, &mut budget).unwrap();
    let shared = owned.topology().loops[0].coedges[1].edge;
    let root_points = owned.topology().edges[shared].curve.root_points().to_vec();
    let surface = owned.topology().faces[0].surface.clone();
    let first = owned
        .split_edge(shared, Q::new(1.into(), 3.into()), &mut budget)
        .unwrap();
    assert_eq!(first.face_uses, 2);
    assert_ne!(first.root_parameter, Q::from_float(1. / 3.).unwrap());
    let second = owned
        .split_edge(first.right_edge, Q::new(1.into(), 2.into()), &mut budget)
        .unwrap();
    assert_eq!(second.root_parameter, Q::new(2.into(), 3.into()));
    assert_eq!(second.root_id, first.root_id);
    let top = owned.topology();
    for edge in [shared, second.left_edge, second.right_edge] {
        assert_eq!(top.edges[edge].curve.root_points(), root_points);
        assert_eq!(top.edges[edge].curve.root_id(), first.root_id);
    }
    assert_eq!(
        top.edges[shared].curve.root_parameters(),
        &[Q::from_integer(0.into()), Q::new(1.into(), 3.into())]
    );
    assert_eq!(
        top.edges[second.left_edge].curve.root_parameters(),
        &[Q::new(1.into(), 3.into()), Q::new(2.into(), 3.into())]
    );
    assert_eq!(
        top.edges[second.right_edge].curve.root_parameters(),
        &[Q::new(2.into(), 3.into()), Q::from_integer(1.into())]
    );
    assert_eq!(
        (top.vertices.len(), top.edges.len(), top.faces.len()),
        (8, 9, 2)
    );
    assert_eq!(owned.geometry().boundary_uses, 12);
    assert_eq!(owned.geometry().open_edges.len(), 6);
    assert!(std::sync::Arc::ptr_eq(&surface, &top.faces[0].surface));
    assert!(
        owned.joins()[0].within
            && owned
                .fields()
                .iter()
                .all(|f| f.absolute_curvature_upper.unwrap() <= 10.)
    );
    let counts = (
        top.vertices.len(),
        top.edges.len(),
        owned.geometry().boundary_uses,
    );
    assert!(owned.split_edge(shared,Q::new_raw(1.into(),0.into()),&mut budget).is_err());
    assert!(
        owned
            .split_edge(
                shared,
                Q::new(1.into(), 7.into()),
                &mut Budget::new(0).unwrap()
            )
            .is_err()
    );
    assert_eq!(
        (
            owned.topology().vertices.len(),
            owned.topology().edges.len(),
            owned.geometry().boundary_uses
        ),
        counts
    );
    assert!(budget.used() <= 1_000_000);
}
#[test]
fn open_rim_split_adds_one_exact_boundary_use_without_closing_a_body() {
    let mut budget = Budget::new(1_000_000).unwrap();
    let mut owned = strip::assemble(panels(), 1e-7, &mut budget).unwrap();
    let edge = owned.topology().loops[0].coedges[0].edge;
    let result = owned
        .split_edge(
            edge,
            num_rational::BigRational::new(1.into(), 3.into()),
            &mut budget,
        )
        .unwrap();
    assert_eq!(result.face_uses, 1);
    assert_eq!(owned.geometry().open_edges.len(), 7);
    assert_eq!(owned.geometry().boundary_uses, 9);
    assert!(!owned.topology().shells[0].closed && owned.topology().bodies.is_empty());
}
#[test]
fn original_source_contacts_bind_saved_edges_and_expire_after_root_interval_edits(){
    use num_rational::BigRational as Q;
    let mut budget=Budget::new(2_000_000).unwrap();
    let mut owned=strip::assemble(panels(),1e-7,&mut budget).unwrap();
    let bindings=owned.source_contacts(&mut budget).unwrap();
    assert_eq!(bindings.len(),4);
    for binding in &bindings {
        assert!(binding.is_current(&owned));
        assert_eq!(binding.contact().points(),owned.topology().edges[binding.edge()].curve.points());
        assert_eq!(binding.contact().source(),owned.topology().faces[binding.transition_face()].surface.sources()[binding.source_side()]);
    }
    let edge=bindings[0].edge();owned.split_edge(edge,Q::new(1.into(),3.into()),&mut budget).unwrap();
    assert!(!bindings[0].is_current(&owned));
    assert!(bindings[1..].iter().all(|binding|binding.is_current(&owned)));
    let fresh=owned.source_contacts(&mut budget).unwrap();assert_eq!(fresh.len(),5);
    assert!(fresh.iter().all(|binding|binding.is_current(&owned)));
    let left=fresh.iter().find(|binding|binding.edge()==edge).unwrap();
    assert_eq!(left.contact().root_parameters(),&[Q::from_integer(0.into()),Q::new(1.into(),3.into())]);
    assert!(owned.source_contacts(&mut Budget::new(0).unwrap()).is_err());
}
#[test]
fn end_rims_split_into_exact_source_chart_uses_with_second_order_contact(){
    use num_rational::BigRational as Q;
    let mut budget=Budget::new(2_000_000).unwrap();
    let mut owned=strip::assemble(panels(),1e-7,&mut budget).unwrap();
    assert!(owned.source_end_contacts(&mut budget).is_err());
    let plan=owned.source_end_splits().unwrap();assert_eq!(plan.len(),2);
    for proposal in plan {assert_eq!(proposal.parameter(),&Q::new(1.into(),2.into()));owned.apply_source_end_split(&proposal,&mut budget).unwrap();}
    assert!(owned.source_end_splits().unwrap().is_empty());
    let bindings=owned.source_end_contacts(&mut budget).unwrap();assert_eq!(bindings.len(),4);
    assert!(bindings.iter().all(|binding|binding.is_current(&owned)));
    for binding in &bindings {
        assert_eq!(binding.contact().points(),owned.topology().edges[binding.edge()].curve.points());
        assert_eq!(binding.contact().source(),owned.topology().faces[binding.transition_face()].surface.sources()[binding.contact().side()]);
    }
    assert_eq!(owned.geometry().open_edges.len(),8);
    assert_eq!(owned.source_contacts(&mut budget).unwrap().len()+bindings.len(),8);
    let edge=bindings[0].edge();owned.split_edge(edge,Q::new(1.into(),3.into()),&mut budget).unwrap();
    assert!(!bindings[0].is_current(&owned));assert_eq!(owned.source_end_contacts(&mut budget).unwrap().len(),5);
    assert!(!owned.topology().shells[0].closed&&owned.topology().bodies.is_empty());
}

#[test]
fn late_end_split_proposal_cannot_apply_to_a_changed_root_interval(){
    use num_rational::BigRational as Q;
    let mut b=Budget::new(2_000_000).unwrap();let mut owned=strip::assemble(panels(),1e-7,&mut b).unwrap();
    let proposals=owned.source_end_splits().unwrap();let p=&proposals[0];assert!(p.is_current(&owned));
    owned.split_edge(p.edge(),Q::new(1.into(),3.into()),&mut b).unwrap();assert!(!p.is_current(&owned));
    let counts=(owned.topology().edges.len(),owned.topology().vertices.len());
    assert!(owned.apply_source_end_split(p,&mut b).is_err());
    assert_eq!((owned.topology().edges.len(),owned.topology().vertices.len()),counts);
    for fresh in owned.source_end_splits().unwrap(){assert!(fresh.is_current(&owned));owned.apply_source_end_split(&fresh,&mut b).unwrap();}
    assert!(owned.source_end_splits().unwrap().is_empty());
    assert_eq!(owned.source_end_contacts(&mut b).unwrap().len(),5);
}
#[test]
fn retained_original_regions_have_exact_closed_graph_notches_and_positive_area(){
    use num_rational::BigRational as Q;
    let mut budget=Budget::new(3_000_000).unwrap();let mut owned=strip::assemble(panels(),1e-7,&mut budget).unwrap();
    for p in owned.source_end_splits().unwrap(){owned.apply_source_end_split(&p,&mut budget).unwrap();}
    let regions=owned.retained_regions(&mut budget).unwrap();
    for region in &regions {
        assert!(region.is_current(&owned));assert_eq!(region.parameter_area(),&Q::new(7.into(),2.into()));
        assert_eq!(region.uses().len(),9);
        assert_eq!(region.uses().iter().filter(|(u,_)|u.edge().is_some()).count(),4);
        assert_eq!(region.source(),owned.topology().faces[0].surface.sources()[region.side()]);
    }
    let edge=regions[0].uses().iter().find_map(|(u,_)|match u{strip::RetainedUse::Contact(c)=>Some(c.edge()),_=>None}).unwrap();
    owned.split_edge(edge,Q::new(1.into(),3.into()),&mut budget).unwrap();
    assert!(!regions[0].is_current(&owned));assert!(regions[1].is_current(&owned));
    let fresh=owned.retained_regions(&mut budget).unwrap();assert_eq!(fresh[0].parameter_area(),fresh[1].parameter_area());
    assert_eq!(fresh[0].uses().len(),10);assert_eq!(fresh[1].uses().len(),9);
    assert!(owned.retained_regions(&mut Budget::new(0).unwrap()).is_err());
}
#[test]
fn retained_source_faces_sew_to_transition_edges_with_only_original_outer_rim_open(){
    let mut budget=Budget::new(3_000_000).unwrap();let mut owned=strip::assemble(panels(),1e-7,&mut budget).unwrap();
    for proposal in owned.source_end_splits().unwrap(){owned.apply_source_end_split(&proposal,&mut budget).unwrap();}
    let transition_edges=owned.topology().edges.len();let transition_faces=owned.topology().faces.len();
    let roof=owned.sew_retained(&mut budget).unwrap();let m=roof.topology();
    assert_eq!((m.faces.len(),m.edges.len(),m.vertices.len()),(4,17,14));
    assert_eq!(roof.geometry().open_edges.len(),6);
    assert!(!m.shells[0].closed&&m.bodies.is_empty());
    let mut counts=vec![0;m.edges.len()];for l in &m.loops{for c in &l.coedges{counts[c.edge]+=1;}}
    assert!(counts[..transition_edges].iter().all(|&n|n==2));
    assert_eq!(m.vertices.len() as isize-m.edges.len() as isize+m.faces.len() as isize,1);
    let fresh=strip::inspect_roof_geometry(m,&mut budget).unwrap();assert_eq!(fresh.open_edges,roof.geometry().open_edges);
    let mut bad=m.clone();bad.shells[0].closed=true;assert!(strip::inspect_roof_geometry(&bad,&mut budget).is_err());
    let mut bad=m.clone();bad.loops[transition_faces].coedges[0].pcurve=strip::FaceUseGeometry::Retained(1);
    assert!(strip::inspect_roof_geometry(&bad,&mut budget).is_err());
    let mut foreign=strip::assemble(panels(),1e-7,&mut budget).unwrap();
    for p in foreign.source_end_splits().unwrap(){foreign.apply_source_end_split(&p,&mut budget).unwrap();}
    let foreign=foreign.sew_retained(&mut budget).unwrap();
    let mut bad=m.clone();bad.edges[0].curve=foreign.topology().edges[0].curve.clone();
    assert!(strip::inspect_roof_geometry(&bad,&mut budget).is_err());
    assert!(roof.transition_joins().iter().all(|r|r.within));
    assert_eq!(roof.source_fields().len(),2);assert!(roof.source_fields().iter().all(|r|r.absolute_curvature_upper<=10.));
    assert!(roof.transition_fields().iter().all(|r|r.absolute_curvature_upper.unwrap()<=10.));
}

#[test]
fn original_support_field_proves_plane_and_refuses_certified_curvature_excess(){
    let mut source=nurbs_core::surface::Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:(0..3).map(|i|vec![vec![i as f64/2.,0.,0.],vec![i as f64/2.,1.,0.]]).collect(),weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false};
    let r=strip::inspect_source_support_field(&source).unwrap();assert!(r.absolute_curvature_upper<1e-10&&r.cells>0);
    source.control_points[2][0][2]=50.;source.control_points[2][1][2]=50.;
    assert!(strip::inspect_source_support_field(&source).is_err());
}
#[test]
fn sewn_roof_boundaries_retain_named_original_edges_and_rational_parent_intervals(){
    use num_rational::BigRational as Q;
    let(mut source,request,_,_)=fixture::fixture();
    let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let source_faces=plan.edges[0].supports;
    let mut budget=Budget::new(4_000_000).unwrap();let mut proposal=strip::assemble(panels(),1e-7,&mut budget).unwrap();
    for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut budget).unwrap();}
    let roof=proposal.sew_retained(&mut budget).unwrap();
    let map=roof.bind_original_boundaries(&source,source_faces,&mut budget).unwrap();
    assert!(map.is_current(&source,&roof));assert_eq!(map.edges().len(),8);
    assert!(roof.geometry().open_edges.iter().all(|edge|map.edges().iter().any(|b|b.roof_edge==*edge)));
    let crease=map.edges().iter().filter(|b|b.source_edge==request.edge).collect::<Vec<_>>();
    assert_eq!(crease.len(),2);
    assert!(crease.iter().any(|b|b.source_range==[Q::from_integer(0.into()),Q::new(1.into(),4.into())]));
    assert!(crease.iter().any(|b|b.source_range==[Q::new(3.into(),4.into()),Q::from_integer(1.into())]));
    assert!(map.edges().iter().all(|b|b.source_id==source.1.edges[b.source_edge]));
    assert!(map.endpoints().iter().any(|p|p.source_vertex.is_none()&&p.source_parameter==Q::new(1.into(),4.into())));
    let mut changed=source.clone();changed.edges[request.edge].curve.control_points[1][1]=2f64.powi(-100);
    assert!(!map.is_current(&changed,&roof));
    assert!(roof.bind_original_boundaries(&changed,source_faces,&mut budget).is_err());
    let vertex=map.endpoints().iter().find_map(|p|p.source_vertex.filter(|&i|source.vertices[i].point[1]==0.)).unwrap();
    source.vertices[vertex].point[1]=2f64.powi(-100);assert!(!map.is_current(&source,&roof));
}

#[test]
fn original_faces_close_exact_shell_without_admitting_an_unchecked_solid() {
    let(source,request,_,_)=fixture::fixture();let before=source.clone();
    let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let mut b=Budget::new(5_000_000).unwrap();
    let mut proposal=strip::assemble(panels(),1e-7,&mut b).unwrap();
    for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut b).unwrap();}
    let roof=proposal.sew_retained(&mut b).unwrap();
    let closed=roof.close_original_shell(&source,plan.edges[0].supports,&mut b).unwrap();
    let top=closed.topology();
    assert_eq!((top.faces.len(),top.edges.len(),top.vertices.len()),(9,25,18));
    assert!(top.shells[0].closed&&top.bodies.is_empty());
    assert_eq!(closed.geometry().boundary_uses,50);assert!(closed.geometry().open_edges.is_empty());
    assert_eq!(top.vertices.len() as isize-top.edges.len() as isize+top.faces.len() as isize,2);
    let mut counts=vec![0;top.edges.len()];for l in &top.loops{for c in &l.coedges{counts[c.edge]+=1;}}
    assert!(counts.iter().all(|&n|n==2));
    assert_eq!(closed.imported_fields().len(),5);assert!(closed.imported_fields().iter().all(|r|r.absolute_curvature_upper<=10.));
    assert_eq!(closed.source_vertex_links().len(),10);assert_eq!(closed.source_face_links().len(),7);
    assert_eq!(closed.source_edge_links().len(),16);
    assert_eq!(closed.original_source(),&before);assert_eq!(source,before);
    let fresh=strip::inspect_roof_geometry(top,&mut b).unwrap();assert!(fresh.open_edges.is_empty());
    let mut bad=top.clone();let imported=bad.faces.iter().find(|f|matches!(f.surface,strip::FaceGeometry::Imported(_))).unwrap().outer;
    bad.loops[imported].coedges[0].pcurve=strip::FaceUseGeometry::Imported(1);
    assert!(strip::inspect_roof_geometry(&bad,&mut b).is_err());
    let mut bad=top.clone();bad.vertices.last_mut().unwrap().point[0]+=num_rational::BigRational::new(1.into(),num_bigint::BigInt::from(1)<<100usize);
    assert!(strip::inspect_roof_geometry(&bad,&mut b).is_err());
    // The imported floor is not in the roof binding map: prove that its
    // actual authored UV/world curves are checked during closure as well.
    let floor=source.faces.iter().position(|f|f.surface.control_points.iter().flatten().all(|p|p[2]==-4.)).unwrap();
    for rational in [false,true] {
        let mut changed=source.clone();let outer=changed.faces[floor].outer;let c=changed.loops[outer].coedges[0].clone();
        if rational {changed.edges[c.edge].curve.weights[0]=2.;}
        else {changed.loops[outer].coedges[0].pcurve.control_points[0][0]+=0.125;}
        let mut proposal=strip::assemble(panels(),1e-7,&mut b).unwrap();
        for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut b).unwrap();}
        let roof=proposal.sew_retained(&mut b).unwrap();
        assert!(roof.close_original_shell(&changed,plan.edges[0].supports,&mut b).is_err());
    }
}

#[test]
fn closed_shell_signed_flux_matches_independent_graph_volume() {
    use num_rational::BigRational as Q;
    let(source,request,_,_)=fixture::fixture();let before=source.clone();
    let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let mut b=Budget::new(8_000_000).unwrap();let mut proposal=strip::assemble(panels(),1e-7,&mut b).unwrap();
    let shared=proposal.topology().loops[0].coedges[1].edge;
    let split=proposal.split_edge(shared,Q::new(1.into(),3.into()),&mut b).unwrap();
    proposal.split_edge(split.right_edge,Q::new(1.into(),2.into()),&mut b).unwrap();
    for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut b).unwrap();}
    let closed=proposal.sew_retained(&mut b).unwrap().close_original_shell(&source,plan.edges[0].supports,&mut b).unwrap();
    let flux=closed.signed_flux(&mut b).unwrap();
    // Original height is q(x)-h(x)|y| over [-1,3]x[-1,1], floor=-4.
    // Its volume is 32 + 2*integral(q) - integral(h).
    let s=&source.faces[plan.edges[0].supports[0]].surface;
    let q=s.control_points.iter().map(|r|Q::from_float(r[0][2]).unwrap()).sum::<Q>();
    let h=s.control_points.iter().map(|r|Q::from_float(r[0][2]).unwrap()-Q::from_float(r[1][2]).unwrap()).sum::<Q>();
    let original=Q::from_integer(32.into())+(q*Q::from_integer(8.into())-h*Q::from_integer(4.into()))/Q::from_integer(9.into());
    // Quartic transverse phi has integral 3/5, versus |y/w| integral 1/2.
    // At w=1/4 the lost volume is integral_0^2(h)/80. Independently,
    // h=280*x^4*(2-x)^4/2^18 and integral_0^2(h)=1/1152.
    let expected=original-Q::new(1.into(),92160.into());
    assert_eq!(flux.total(),&expected);assert!(flux.total()> &Q::from_integer(0.into()));
    assert_eq!(flux.face_contributions().len(),9);
    assert_eq!(flux.face_contributions().iter().cloned().sum::<Q>(),*flux.total());
    assert!(closed.signed_flux(&mut Budget::new(0).unwrap()).is_err());
    assert!(closed.topology().bodies.is_empty());assert_eq!(source,before);
    let hulls=closed.inspect_pair_hulls(36,&mut b).unwrap();assert_eq!(hulls.pairs().len(),36);
    assert_eq!(hulls.separated()+hulls.unresolved(),36);
    assert!(hulls.separated()>0&&hulls.unresolved()>0);
    let adjacent=hulls.pairs().iter().find(|p|p.faces==[0,1]).unwrap();
    assert_eq!(adjacent.status,strip::PairHullStatus::Unresolved);assert!(!adjacent.shared_edges.is_empty());
    assert!(hulls.pairs().iter().filter(|p|matches!(p.status,strip::PairHullStatus::Separated{..})).all(|p|p.shared_edges.is_empty()));
    assert!(closed.inspect_pair_hulls(35,&mut b).is_err());
    assert!(closed.inspect_pair_hulls(36,&mut Budget::new(0).unwrap()).is_err());
    let contacts=closed.inspect_transverse_pairs(36,&mut b).unwrap();
    assert_eq!(contacts.contacts().len(),1);assert_eq!(contacts.contacts()[0].faces(),[0,1]);
    assert_eq!(contacts.contacts()[0].shared_edges().len(),3);
    assert_eq!(contacts.hulls().separated()+contacts.contacts().len()+contacts.unresolved().len(),36);
    assert_eq!(contacts.unresolved().len(),hulls.unresolved()-1);
    assert!(closed.inspect_transverse_pairs(35,&mut b).is_err());
    assert!(closed.inspect_transverse_pairs(36,&mut Budget::new(0).unwrap()).is_err());
    let charts=closed.inspect_face_charts(&mut b).unwrap();
    assert_eq!(charts.proven().len(),9);assert!(charts.unresolved().is_empty());
    assert!(closed.inspect_face_charts(&mut Budget::new(0).unwrap()).is_err());
    let retained=closed.inspect_retained_pairs(36,&mut b).unwrap();
    assert_eq!(retained.contacts().len(),4);assert_eq!(retained.unresolved().len(),15);
    assert_eq!(retained.transverse().hulls().separated()+retained.transverse().contacts().len()+retained.contacts().len()+retained.unresolved().len(),36);
    assert!(closed.inspect_retained_pairs(35,&mut b).is_err());
    assert!(closed.inspect_retained_pairs(36,&mut Budget::new(0).unwrap()).is_err());
    let original=closed.inspect_original_pairs(36,&mut b).unwrap();
    assert_eq!(original.contacts().len(),15);assert!(original.unresolved().is_empty());
    assert_eq!(original.charts().proven().len(),9);assert!(original.globally_embedded());
    let crease=original.contacts().iter().find(|(faces,_)|*faces==[2,3]).unwrap();assert_eq!(crease.1.len(),2);
    assert!(closed.inspect_original_pairs(35,&mut b).is_err());
    assert!(closed.inspect_original_pairs(36,&mut Budget::new(0).unwrap()).is_err());
    assert!(closed.topology().bodies.is_empty());
    let limits=strip::VolumeBounds::new(expected.clone(),expected.clone()).unwrap();
    let too_small=strip::VolumeBounds::new(Q::from_integer(0.into()),&expected-Q::new(1.into(),92160.into())).unwrap();
    assert!(closed.qualify_volume(&too_small,36,&mut b).is_err());
    let too_large=strip::VolumeBounds::new(&expected+Q::new(1.into(),92160.into()),&expected+Q::from_integer(1.into())).unwrap();
    assert!(closed.qualify_volume(&too_large,36,&mut b).is_err());
    assert!(closed.qualify_volume(&limits,35,&mut b).is_err());
    assert!(closed.qualify_volume(&limits,36,&mut Budget::new(0).unwrap()).is_err());
    assert!(closed.topology().bodies.is_empty());assert_eq!(closed.original_source(),&before);
    let native=closed.admit_volume(limits,36,&mut b).unwrap();
    assert_eq!(native.volume(),&expected);assert!(native.qualification().embedding().globally_embedded());
    assert_eq!(native.topology().bodies.len(),1);assert_eq!(native.topology().bodies[0].outer_shell,0);
    assert!(native.topology().bodies[0].inner_shells.is_empty());assert!(native.original_shell().topology().bodies.is_empty());
    assert_eq!(native.original_shell().original_source(),&before);
    let radius=native.inspect_transition_normal_radius(1.,0.001,[[0.,1.],[0.25,0.75]],100,&mut b).unwrap();
    assert_eq!(radius.reports().len(),2);assert!(!radius.declared_direction_within());
    assert!(radius.reports().iter().all(|(_,r)|r.decision==graph::NormalRadiusDecision::Exceeds&&r.witness_uv.is_some()));
    assert!(native.inspect_transition_normal_radius(1.,0.001,[[0.,1.];2],100,&mut Budget::new(0).unwrap()).is_err());
    assert_eq!(native.volume(),&expected);
}
#[test]
fn exact_volume_bounds_refuse_invalid_intervals(){
    use num_rational::BigRational as Q;
    let q=|n:i64|Q::from_integer(n.into());
    assert!(strip::VolumeBounds::new(q(-1),q(1)).is_err());
    assert!(strip::VolumeBounds::new(q(2),q(1)).is_err());
    assert!(strip::VolumeBounds::new(q(0),q(0)).is_err());
    assert!(strip::VolumeBounds::new(q(1),q(1)).is_ok());
    assert!(strip::VolumeBounds::new(q(0),Q::from_integer(num_bigint::BigInt::from(1)<<8193usize)).is_err());
}

#[test]
fn rebuilt_g2_profile_sews_and_admits_with_independent_exact_volume(){
    use num_rational::BigRational as Q;
    let(mut source,request,_,_)=fixture::fixture();
    // Put the 4-mm endcap witness before the thinner side-wall pair.
    // Automatic search must continue and find the attained 2-mm bound.
    source.shells[0].faces.reverse();let before=source.clone();
    let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let mut b=Budget::new(10_000_000).unwrap();
    let source_laws=(0..2).map(|i|{
        let w=Curve{degree:1,knots:vec![i as f64,i as f64,i as f64+1.,i as f64+1.],control_points:vec![vec![0.25,0.,0.];2],weights:vec![1.;2],periodic:false};
        (i,graph::SourceContactCurvatureLaw::new(&source.faces[plan.edges[0].supports[0]].surface,&w,&mut b).unwrap())
    }).collect::<Vec<_>>();let mut proposal=strip::assemble(panels_profile(true),1e-7,&mut b).unwrap();
    for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut b).unwrap();}
    let closed=proposal.sew_retained(&mut b).unwrap().close_original_shell(&source,plan.edges[0].supports,&mut b).unwrap();
    let s=&source.faces[plan.edges[0].supports[0]].surface;
    let q=s.control_points.iter().map(|r|Q::from_float(r[0][2]).unwrap()).sum::<Q>();
    let h=s.control_points.iter().map(|r|Q::from_float(r[0][2]).unwrap()-Q::from_float(r[1][2]).unwrap()).sum::<Q>();
    let original=Q::from_integer(32.into())+(q*Q::from_integer(8.into())-h*Q::from_integer(4.into()))/Q::from_integer(9.into());
    // Integral phi = 2/3 - (1/8)*(1/21-2/23+1/25) = 2683/4025.
    // With w=1/4 and integral_0^2 h=1/1152, loss=(2*mean(phi)-1)/18432.
    let loss=Q::new(1341.into(),(4025i64*18432).into());let expected=original-loss;
    let native=closed.admit_volume(strip::VolumeBounds::new(expected.clone(),expected.clone()).unwrap(),36,&mut b).unwrap();
    assert_eq!(native.volume(),&expected);assert!(native.qualification().embedding().globally_embedded());
    assert_eq!((native.topology().faces.len(),native.topology().edges.len(),native.topology().vertices.len()),(9,25,18));
    assert_eq!(native.topology().bodies.len(),1);
    assert_eq!(native.topology().edges.iter().map(|e|e.curve.points().len()).max(),Some(25));
    assert_eq!(native.original_shell().original_source(),&before);
    let laws=native.inspect_transition_source_laws(&source_laws,0.001,0.001, 0.001,[[0.,1.],[0.25,0.75]],&mut b).unwrap();
    assert_eq!(laws.reports().len(),2);assert!(laws.all_within());
    assert!(native.inspect_transition_source_laws(&source_laws[..1],0.001,0.001, 0.001,[[0.,1.],[0.25,0.75]],&mut b).is_err());
    let incomplete=native.inspect_transition_source_laws(&source_laws,0.001,0.001, 0.001,[[0.,1.],[0.25,0.75]],&mut Budget::new(0).unwrap()).unwrap();assert!(!incomplete.all_within());
    assert!(incomplete.reports().iter().all(|(_,r)|r.decision==graph::CurvatureLawDecision::Unresolved));
    let wall=native.inspect_aligned_wall_lower_bound(Q::from_integer(2.into()),1e-6,45,&mut b).unwrap();
    assert!(wall.complete());assert_eq!(wall.pairs().len(),45);assert_eq!(wall.pairs().iter().filter(|p|p.faces[0]==p.faces[1]).count(),9);
    assert!(std::ptr::eq(wall.body(),&native));
    let witness=wall.find_axis_material_witness(&mut b).unwrap().unwrap();
    assert_eq!(witness.axis(),1);assert_eq!(witness.length_mm(),&Q::from_integer(2.into()));
    assert_eq!(witness.excluded_faces().len(),7);assert!(std::ptr::eq(witness.body(),&native));
    assert_eq!(wall.exact_minimum_with(&witness),Some(&Q::from_integer(2.into())));

    let too_thick=native.inspect_aligned_wall_lower_bound(Q::new(21.into(),10.into()),1e-6,45,&mut b).unwrap();
    assert!(!too_thick.complete());assert!(too_thick.exact_minimum_with(&witness).is_none());assert!(witness.length_mm()<too_thick.minimum_mm());assert!(too_thick.pairs().iter().any(|p|p.decision==strip::AlignedWallPairDecision::Unresolved));
    assert!(native.inspect_aligned_wall_lower_bound(Q::from_integer(2.into()),1e-6,44,&mut b).is_err());
    assert!(native.inspect_aligned_wall_lower_bound(Q::from_integer(2.into()),1e-6,45,&mut Budget::new(0).unwrap()).is_err());
    let mut wrong=source_laws;wrong[0].0=1;wrong[1].0=0;
    assert!(native.inspect_transition_source_laws(&wrong,0.001,0.001, 0.001,[[0.,1.],[0.25,0.75]],&mut b).is_err());
}

#[test]
fn convex_g2_rolloff_closes_with_independent_exact_volume() {
    use num_rational::BigRational as Q;
    let(source,request,_,_)=fixture::fixture();let before=source.clone();
    let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let supports=plan.edges[0].supports.map(|i|&source.faces[i].surface);
    let mut b=Budget::new(10_000_000).unwrap();
    let patches=(0..2).map(|i|{
        let w=Curve{degree:1,knots:vec![i as f64,i as f64,i as f64+1.,i as f64+1.],control_points:vec![vec![0.25,0.,0.];2],weights:vec![1.;2],periodic:false};
        graph::build_convex_g2(supports,&w,&mut b).unwrap()
    }).collect();
    let mut roof=strip::assemble(patches,1e-7,&mut b).unwrap();
    assert!(roof.joins().iter().all(|j|j.within));
    for split in roof.source_end_splits().unwrap(){roof.apply_source_end_split(&split,&mut b).unwrap();}
    let shell=roof.sew_retained(&mut b).unwrap().close_original_shell(&source,plan.edges[0].supports,&mut b).unwrap();
    let s=supports[0];
    let q=s.control_points.iter().map(|r|Q::from_float(r[0][2]).unwrap()).sum::<Q>();
    let h=s.control_points.iter().map(|r|Q::from_float(r[0][2]).unwrap()-Q::from_float(r[1][2]).unwrap()).sum::<Q>();
    let original=Q::from_integer(32.into())+(q*Q::from_integer(8.into())-h*Q::from_integer(4.into()))/Q::from_integer(9.into());
    // Independent integration: phi=1-2v+(28/3)v^3-18v^4+16v^5-(16/3)v^6.
    // mean(phi)=67/105; loss=(2*mean-1)/18432=29/(105*18432).
    let expected=original-Q::new(29.into(),(105i64*18432).into());
    let native=shell.admit_volume(strip::VolumeBounds::new(expected.clone(),expected.clone()).unwrap(),36,&mut b).unwrap();
    assert_eq!(native.volume(),&expected);
    assert!(native.qualification().embedding().globally_embedded());
    assert_eq!(native.topology().bodies.len(),1);
    assert_eq!(native.topology().faces.len(),9);
    let wall=native.inspect_aligned_wall_lower_bound(Q::from_integer(2.into()),1e-6,45,&mut b).unwrap();
    assert!(wall.complete());
    let witness=wall.find_axis_material_witness(&mut b).unwrap().unwrap();
    assert_eq!(wall.exact_minimum_with(&witness),Some(&Q::from_integer(2.into())));
    assert_eq!(source,before);
}

#[test]
fn original_shell_sews_independent_trimmed_uv_clocks_without_changing_world_roots() {
    let (mut source, request, _, _) = fixture::fixture();
    let plan = brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let world_domains = source.edges.iter().map(|e|e.curve.domain()).collect::<Vec<_>>();
    for wire in &mut source.loops {
        for c in &mut wire.coedges {
            let mut edited = c.clone();
            let a = c.pcurve.control_points[0].clone();
            let b = c.pcurve.control_points[1].clone();
            edited.pcurve.control_points = vec![
                a.iter().zip(&b).map(|(a,b)| 2.*a-b).collect(),
                a.iter().zip(&b).map(|(a,b)| 3.*b-2.*a).collect(),
            ];
            edited.pcurve.knots = vec![2.,2.,6.,6.];
            *c = edited.with_trim(brep_topology::CoedgeTrim {
                curve_parameter: world_domains[c.edge],
                pcurve_parameter: [3.,4.],
                periodic_lift: [[0;2];2],
            }).unwrap();
        }
    }
    let before = source.clone();
    let close = |source: &brep_core::Model| {
        let mut b = Budget::new(8_000_000).unwrap();
        let mut proposal = strip::assemble(panels(),1e-7,&mut b).unwrap();
        for p in proposal.source_end_splits().unwrap() { proposal.apply_source_end_split(&p,&mut b).unwrap(); }
        proposal.sew_retained(&mut b).unwrap().close_original_shell(source,plan.edges[0].supports,&mut b)
    };
    let closed = close(&source).unwrap();
    assert!(closed.geometry().open_edges.is_empty());
    assert_eq!(closed.topology().edges.len(),25);
    assert_eq!(closed.original_source(),&before);
    assert_eq!(source,before);
    assert!(closed.imported_fields().iter().all(|f|f.absolute_curvature_upper<=10.));
    assert!(closed.inherited_roof().transition_joins().iter().all(|j|j.within));
    // Import and source-binding paths both reject changed clocks, partial
    // world ownership and periodic metadata rather than dropping the trim.
    let floor = source.faces.iter().position(|f| f.surface.control_points.iter().flatten().all(|p|p[2]==-4.)).unwrap();
    for face in [floor,plan.edges[0].supports[0]] {
        for defect in 0..3 {
            let mut bad = source.clone();
            let outer = bad.faces[face].outer;
            let c = &mut bad.loops[outer].coedges[0];
            let mut trim = c.trim().unwrap();
            match defect {
                0 => trim.pcurve_parameter[1] = 4.125,
                1 => trim.curve_parameter[1] *= 0.5,
                _ => trim.periodic_lift[1][0] = 1,
            }
            *c = c.clone().with_trim(trim).unwrap();
            assert!(close(&bad).is_err(),"accepted defect {defect} on face {face}");
        }
    }
}

#[test]
fn partial_original_world_edges_sew_both_faces_with_shared_nondyadic_root_ends() {
    use num_rational::BigRational as Q;
    let (source,request,_,_)=fixture::fixture();
    let before=source.clone();
    let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let mut b=Budget::new(10_000_000).unwrap();
    let mut proposal=strip::assemble(panels_profile(true),1e-7,&mut b).unwrap();
    for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut b).unwrap();}
    let mut roof=proposal.sew_retained(&mut b).unwrap();
    let links=roof.bind_original_boundaries(&source,plan.edges[0].supports,&mut b).unwrap();
    let candidates=links.edges().iter().filter(|l| roof.geometry().open_edges.contains(&l.roof_edge)
        && l.source_range==[Q::from_integer(0.into()),Q::from_integer(1.into())]).cloned().collect::<Vec<_>>();
    assert!(!candidates.is_empty());
    // Split every natural outer boundary, covering straight and curved source
    // edges and opposite neighboring coedge traversal directions.
    let mut children=Vec::new();
    for link in &candidates {
        let first=roof.split_natural_edge(link.roof_edge,Q::new(1.into(),3.into()),&mut b).unwrap();
        let second=roof.split_natural_edge(first.right_edge,Q::new(1.into(),2.into()),&mut b).unwrap();
        assert_eq!(first.root_id,second.root_id);
        assert_eq!(second.root_parameter,Q::new(2.into(),3.into()));
        children.push((link.source_edge,[first.left_edge,second.left_edge,second.right_edge]));
    }
    let closed=roof.close_original_shell(&source,plan.edges[0].supports,&mut b).unwrap();
    assert!(closed.geometry().open_edges.is_empty());
    for (source_edge,edges) in children {
        let links=closed.source_edge_links().iter().filter(|l|l.source_edge==source_edge).collect::<Vec<_>>();
        assert_eq!(links.len(),3);
        for edge in edges {
            let top=closed.topology();
            let uses=top.shells[0].faces.iter().flat_map(|f|top.loops[top.faces[f.face].outer].coedges.iter().filter(move |c|c.edge==edge).map(move |c|c.reversed^f.reversed)).collect::<Vec<_>>();
            assert_eq!(uses.len(),2);
            assert_ne!(uses[0],uses[1]);
        }
    }
    let native=closed.admit_volume(strip::VolumeBounds::new(Q::from_integer(0.into()),Q::from_integer(40.into())).unwrap(),100,&mut b).unwrap();
    assert!(native.qualification().embedding().globally_embedded());
    assert_eq!(native.topology().bodies.len(),1);
    assert_eq!(native.original_shell().original_source(),&before);
    assert!(native.original_shell().inherited_roof().transition_joins().iter().all(|j|j.within));
    assert!(native.original_shell().inherited_roof().transition_fields().iter().all(|f|f.absolute_curvature_upper.is_some_and(|k|k<=10.)));
    assert_eq!(source,before);
}

#[test]
fn natural_root_split_refuses_invalid_or_unproved_edits_without_publishing_children() {
    use num_rational::BigRational as Q;
    let mut b=Budget::new(10_000_000).unwrap();
    let mut proposal=strip::assemble(panels_profile(true),1e-7,&mut b).unwrap();
    for p in proposal.source_end_splits().unwrap(){proposal.apply_source_end_split(&p,&mut b).unwrap();}
    let mut roof=proposal.sew_retained(&mut b).unwrap();
    let edge=roof.geometry().open_edges[0];
    let counts=(roof.topology().vertices.len(),roof.topology().edges.len(),roof.topology().loops.iter().map(|l|l.coedges.len()).collect::<Vec<_>>());
    let points=roof.topology().edges.iter().map(|e|e.curve.points().to_vec()).collect::<Vec<_>>();
    for q in [Q::from_integer(0.into()),Q::from_integer(1.into()),Q::new_raw(1.into(),0.into())] {
        assert!(roof.split_natural_edge(edge,q,&mut b).is_err());
    }
    assert!(roof.split_natural_edge(edge,Q::new(1.into(),3.into()),&mut Budget::new(0).unwrap()).is_err());
    let transition=roof.topology().faces.iter().find(|f|matches!(f.surface,strip::FaceGeometry::Transition(_))).unwrap();
    let bound=roof.topology().loops[transition.outer].coedges[0].edge;
    assert!(roof.split_natural_edge(bound,Q::new(1.into(),3.into()),&mut b).is_err());
    assert_eq!(counts,(roof.topology().vertices.len(),roof.topology().edges.len(),roof.topology().loops.iter().map(|l|l.coedges.len()).collect::<Vec<_>>()));
    assert_eq!(points,roof.topology().edges.iter().map(|e|e.curve.points().to_vec()).collect::<Vec<_>>());
    strip::inspect_roof_geometry(roof.topology(),&mut b).unwrap();
}
