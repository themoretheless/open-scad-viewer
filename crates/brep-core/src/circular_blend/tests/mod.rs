use super::*;
#[test]
fn torus_support_has_exact_radius_and_contact_boundaries() {
    for sweep in [0.31, 2.2, -2.2, std::f64::consts::TAU] {
        let spans = plane_cylinder_rim(20., 6., 1.25, 5.9, sweep).unwrap();
        for span in &spans {
            for i in 0..=20 {
                let u = i as f64 / 20.;
                let center = span.centers.evaluate(u).unwrap().point;
                for j in 0..=20 {
                    let point = span.surface.evaluate(u, j as f64 / 20.).unwrap().point;
                    let distance = (point[0] - center[0])
                        .hypot(point[1] - center[1])
                        .hypot(point[2] - center[2]);
                    assert!((distance - 1.25).abs() < 1e-11);
                }
                for (v, rail) in [(0., &span.plane_contact), (1., &span.cylinder_contact)] {
                    let evaluation = span.surface.evaluate(u, v).unwrap();
                    let point = evaluation.point;
                    let normal = evaluation.unit_normal().unwrap();
                    let target = if v == 0. {
                        [0., 0., 1.]
                    } else {
                        [point[0] / 20., point[1] / 20., 0.]
                    };
                    let alignment: f64 = (0..3).map(|k| normal[k] * target[k]).sum();
                    assert!((alignment.abs() - 1.).abs() < 1e-11);
                    let expected = rail.evaluate(u).unwrap().point;
                    for k in 0..3 {
                        assert!((point[k] - expected[k]).abs() < 1e-11);
                    }
                }
            }
        }
        for pair in spans.windows(2) {
            for v in [0., 0.3, 1.] {
                let a = pair[0].surface.evaluate(1., v).unwrap().point;
                let b = pair[1].surface.evaluate(0., v).unwrap().point;
                assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-11));
            }
        }
    }
}
#[test]
fn transition_radius_law_contacts_and_collapsed_end() {
    for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
        let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, -0.7).unwrap();
        for i in 0..=40 {
            let u = i as f64 / 40.;
            let smooth = 3. * u * u - 2. * u * u * u;
            let radius = r0 + (r1 - r0) * smooth;
            let center = span.centers.evaluate(u).unwrap().point;
            for j in 0..=20 {
                let point = span.surface.evaluate(u, j as f64 / 20.).unwrap().point;
                let distance = (point[0] - center[0])
                    .hypot(point[1] - center[1])
                    .hypot(point[2] - center[2]);
                assert!((distance - radius).abs() < 1e-10);
            }
            let plane = span.surface.evaluate(u, 0.).unwrap();
            let cylinder = span.surface.evaluate(u, 1.).unwrap();
            assert!((plane.point[2] - 6.).abs() < 1e-10);
            assert!((cylinder.point[0].hypot(cylinder.point[1]) - 20.).abs() < 1e-10);
            if radius > 1e-5 {
                assert!((plane.unit_normal().unwrap()[2].abs() - 1.).abs() < 1e-10);
                let n = cylinder.unit_normal().unwrap();
                let alignment = (n[0] * cylinder.point[0] + n[1] * cylinder.point[1]) / 20.;
                assert!((alignment.abs() - 1.).abs() < 1e-10);
            }
        }
    }
    assert!(plane_cylinder_transition(20., 6., 0., 0., 0., 0.5).is_err());
}

#[test]
fn polar_projection_certifies_constant_torus_chart_from_original_controls() {
    for (outer,height,radius,start,sweep) in [(20.,6.,1.25,0.,std::f64::consts::FRAC_PI_2),(8.,4.,0.5,5.9,0.31),(12.,8.,3.,-1.2,0.7)] {
        for direction in [-1., 1.] {
            let span = plane_cylinder_rim(outer,height,radius,start,direction*sweep).unwrap().remove(0);
            let (sin,cos) = (start+direction*sweep*0.5).sin_cos();
            let basis = [[cos,sin,0.,0.],[-sin,cos,0.,0.],[0.,0.,1.,0.]];
            let proof = nurbs_core::surface_injectivity::certify_polar_projection(&span.surface,basis,16,256).unwrap();
            assert!(proof.is_some(), "polar contraction: {outer}, {radius}, {start}, {direction}: {proof:?}");
            assert!(nurbs_core::surface_injectivity::certify_polar_projection(&span.surface,basis,16,255).unwrap().is_none());
            let mut folded = span.surface.clone();
            folded.control_points[2] = folded.control_points[0].clone();
            folded.weights[2] = folded.weights[0].clone();
            assert!(nurbs_core::surface_injectivity::certify_polar_projection(&folded,basis,16,256).unwrap().is_none());
            let automatic = nurbs_core::surface_injectivity::certify(&span.surface,1000).unwrap();
            assert!(automatic.proven, "automatic polar candidate: {automatic:?}");
            assert!(automatic.polar_projection.is_some());
            assert_eq!(automatic.reason,"global-polar-projection-contraction");
        }
    }
}

#[test]
fn partial_transition_poles_pass_weighted_quotient_proof_without_claiming_g1() {
    for radius in [1.25,2.5] { for direction in [-1.,1.] {
        let model=partial_annular_quarter(20.,5.,6.,radius,direction,1e-7).unwrap();
        for (face,end,projection) in [(0,0,[[0.,direction,0.],[1.,0.,-1.]]),(10,1,[[1.,0.,0.],[0.,direction,-1.]])] {
            let s=&model.faces[face].surface;
            let proof=nurbs_core::surface_quotient_injectivity::certify(s,end,projection,16,256).unwrap();
            assert!(proof.proven,"face {face}, direction {direction}: {proof:?}");
            let source_proof=nurbs_core::surface_quotient_injectivity::certify_source_frame(s,end,16,256).unwrap();
            assert!(source_proof.proven,"source frame: {source_proof:?}");
            assert!(!nurbs_core::surface_injectivity::certify(s,1000).unwrap().proven);
            if radius==2.5 {
                let [a,b,e,c]=proof.weighted_bounds.unwrap();assert!(a*b<c*e);
                assert_eq!(proof.reason,"global-localized-weighted-quotient-dominance");
                assert!(proof.band_margins_lower.unwrap().iter().all(|m|*m>0.));
            }
        }
    }}
}
#[test]
fn zero_radius_neighbor_controls_do_not_acquire_transverse_roundoff() {
    for (r0,r1,row) in [(0.,1.25,1),(1.25,0.,4)] {
        for (start,sweep) in [(0.,0.3),(5.9,-0.7),(0.3,1.2)] {
            let span=plane_cylinder_transition(20.,6.,r0,r1,start,sweep).unwrap();
            let controls=&span.surface.control_points[row];
            assert!(controls.iter().all(|p|*p==controls[0]));
            assert_eq!(controls[0][2],6.);
            assert_eq!(controls[0],span.centers.control_points[row]);
        }
    }
}
#[test]
fn collapsed_transition_tip_has_distinct_normal_limits_not_a_regular_g1_point() {
    // Both contact rails end at the original sharp rim. Their limiting
    // tangent planes differ, even though every nonzero section is tangent
    // to both supports. A collapsed UV edge cannot certify a regular G1 tip.
    for direction in [-1., 1.] {
        for (r0, r1, pole_u) in [(0., 1.25, 0.), (1.25, 0., 1.)] {
            let span = plane_cylinder_transition(20., 6., r0, r1, 0.3, direction * 0.7).unwrap();
            for distance in [0.1, 0.01, 0.001] {
                let u = if pole_u == 0. { distance } else { 1. - distance };
                let plane = span.surface.evaluate(u, 0.).unwrap();
                let cylinder = span.surface.evaluate(u, 1.).unwrap();
                let np = plane.unit_normal().unwrap();
                let nc = cylinder.unit_normal().unwrap();
                let dot: f64 = (0..3).map(|k| np[k] * nc[k]).sum();
                assert!(dot.abs() < 1e-7, "distinct limiting support normals: {dot}");
                assert!((np[2].abs() - 1.).abs() < 1e-7);
                assert!(nc[2].abs() < 1e-7);
            }
            assert!(span.surface.evaluate(pole_u, 0.5).unwrap().unit_normal().is_none());
        }
    }
}

#[test]
fn transition_joins_constant_radius_patch_with_matching_tangent_planes() {
    for direction in [-1., 1.] {
        let transition =
            plane_cylinder_transition(20., 6., 0., 1.25, 0.3, direction * 0.7).unwrap();
        let constant =
            plane_cylinder_rim(20., 6., 1.25, 0.3 + direction * 0.7, direction * 0.4).unwrap();
        for i in 0..=40 {
            let v = i as f64 / 40.;
            let a = transition.surface.evaluate(1., v).unwrap();
            let b = constant[0].surface.evaluate(0., v).unwrap();
            assert!((0..3).all(|k| (a.point[k] - b.point[k]).abs() < 1e-10));
            let na = a.unit_normal().unwrap();
            let nb = b.unit_normal().unwrap();
            let alignment: f64 = (0..3).map(|k| na[k] * nb[k]).sum();
            assert!((alignment - 1.).abs() < 1e-10);
        }
    }
}

#[test]
fn transition_join_has_full_interval_normal_coverage_and_refuses_the_pole(){
    for direction in [-1.,1.] {
        let transition=plane_cylinder_transition(20.,6.,0.,1.25,0.3,direction*0.7).unwrap();
        let constant=plane_cylinder_rim(20.,6.,1.25,0.3+direction*0.7,direction*0.4).unwrap();
        let before=format!("{:?}",transition.surface);
        let mut queue=vec![[0.,1.]];let mut checked=0;let mut admitted=0;
        while let Some(v)=queue.pop(){
            checked+=1;assert!(checked*2<=100000,"whole meridian angular coverage exhausted at {v:?}, admitted={admitted}");
            let r=nurbs_core::normal_alignment::inspect_pair(
                [&transition.surface,&constant[0].surface],[[[1.,1.],v],[[0.,0.],v]],1e-6,2).unwrap();
            if r.aligned==Some(true){admitted+=1;continue}
            assert_ne!(r.aligned,Some(false),"a proved angular break cannot be hidden by subdivision");
            let mid=v[0]/2.+v[1]/2.;assert!(mid>v[0]&&mid<v[1]);
            queue.push([mid,v[1]]);queue.push([v[0],mid]);
        }
        assert!(admitted>1);
        eprintln!("transition direction={direction}: checked={checked}, admitted={admitted}, normal spans={}",checked*2);
        let pole=nurbs_core::normal_alignment::inspect_pair(
            [&transition.surface,&constant[0].surface],[[[0.,0.],[0.5,0.5]],[[0.,0.],[0.5,0.5]]],1e-6,2).unwrap();
        assert_eq!(pole.aligned,None);assert_eq!(pole.reason,"normal-unresolved");
        assert_eq!(format!("{:?}",transition.surface),before);
    }
}

#[test]
fn original_transition_projected_cusp_has_quadratic_separator() {
    let span=plane_cylinder_transition(20.,6.,0.,1.25,0.,std::f64::consts::FRAC_PI_6).unwrap();
    let project=|c:&Curve| {
        let mut c=c.clone();for p in &mut c.control_points {p.truncate(2);} c
    };
    let curves=[project(&span.plane_contact),project(&span.cylinder_contact)];
    let join=&curves[0].control_points[0];
    let t=&curves[0].control_points[1];
    let tangent=[t[0]-join[0],t[1]-join[1]];
    let ratio=|curve:&Curve,u:f64| {
        let p=curve.evaluate(u).unwrap().point;
        let delta=[p[0]-join[0],p[1]-join[1]];
        let l=tangent[0]*delta[1]-tangent[1]*delta[0];
        let b=tangent[0]*delta[0]+tangent[1]*delta[1];
        l/(b*b)
    };
    let mut work=0;let mut proven=false;
    for u in [0.1,0.25,0.5,0.75,0.9] {
        let beta=(ratio(&curves[0],u)+ratio(&curves[1],u))*0.5;
        let r=nurbs_core::curve_quadratic_separator::certify([&curves[0],&curves[1]],[0,0],1,beta,1_000_000).unwrap();
        work+=r.exact_work;
        eprintln!("original projected cusp beta={beta} work={} sides={:?}",r.exact_work,r.certificate.as_ref().map(|c|c.sides()));
        if r.certificate.is_some() {proven=true;break;}
    }
    assert!(proven,"original cusp quadratic separation unproven; work={work}");
    let jordan=nurbs_core::surface_projected_jordan::certify(&span.surface,[0,1],1e-8,100_000_000,100000).unwrap();
    eprintln!("original projected Jordan: {} work={} cells={}",jordan.reason,jordan.exact_work,jordan.boundary_cells);
    assert!(jordan.certificate.is_some(),"{}",jordan.reason);
    let certificate=jordan.certificate.unwrap();
    assert_eq!(certificate.surface(),&span.surface);
    assert_eq!(certificate.collapsed_boundaries(),&[3]);
    assert_eq!(certificate.boundary().len(),3);
    assert_eq!(certificate.separators().len(),1);
}
#[test]
fn transition_rounding_correction_preserves_interior_projection_orientation() {
    for direction in [-1.,1.] {
        for (a,b) in [(0.,1.25),(1.25,0.),(0.5,1.25),(1.25,0.5)] {
            let span=plane_cylinder_transition(20.,6.,a,b,0.3,direction*0.7).unwrap();
            let report=nurbs_core::surface_projection_jacobian::certify(&span.surface,[0,1],100_000_000).unwrap();
            assert!(report.certificate.is_some(),"radii {a}->{b} direction={direction}: {} {:?}",report.reason,report.exact_reason);
            assert_eq!(report.opposite_v_boundary_signs,None);
            let expected=if direction>0. {cad_predicates::Sign::Negative} else {cad_predicates::Sign::Positive};
            assert_eq!(report.certificate.unwrap().orientation(),expected);
        }
    }
}
#[test]
fn transition_boundaries_certify_poles_and_form_an_oriented_loop() {
    for (r0, r1, pole_index) in [(0., 1.25, 3), (1.25, 0., 1)] {
        let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, 0.7).unwrap();
        let boundaries = span.boundaries().unwrap();
        assert_eq!(
            boundaries
                .iter()
                .filter(|b| b.collapsed_pole.is_some())
                .count(),
            1
        );
        assert!(boundaries[pole_index].collapsed_pole.is_some());
        for i in 0..4 {
            let a = boundaries[i].curve.evaluate(1.).unwrap().point;
            let b = boundaries[(i + 1) % 4].curve.evaluate(0.).unwrap().point;
            assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-10));
        }
        let mut bad = span.surface.clone();
        let row = if r0 == 0. { 0 } else { 5 };
        bad.control_points[row][1][0] += 1e-8;
        let boundary = &boundaries[pole_index];
        assert!(
            crate::validate_pole_boundary(
                &bad,
                &boundary.pcurve,
                boundary.collapsed_pole.unwrap()
            )
            .is_err()
        );
    }
}

#[test]
fn transition_open_sheet_validates_pole_incidence_and_uv_agreement() {
    for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
        let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, -0.7).unwrap();
        let sheet = span.to_open_sheet(1e-7).unwrap();
        let report = sheet.validate().unwrap();
        assert_eq!(report.body_count, 0);
        assert_eq!(report.face_count, 1);
        assert_eq!(
            report.boundary_edge_count,
            if r0 == 0. || r1 == 0. { 3 } else { 4 }
        );
        assert_eq!(
            sheet.vertices.len(),
            if r0 == 0. || r1 == 0. { 3 } else { 4 }
        );
        assert!(
            sheet
                .edges
                .iter()
                .filter(|e| e.degenerate)
                .all(|e| e.vertices[0] == e.vertices[1])
        );
        let mut invalid_body = sheet.clone();
        invalid_body.0.bodies.push(crate::Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        invalid_body.rebuild_topology_ids();
        assert!(invalid_body.validate().is_err());
    }
}

#[test]
fn three_span_strip_shares_two_seams_and_keeps_two_poles() {
    for direction in [-1., 1.] {
        let strip = circular_blend_strip(
            20.,
            6.,
            1.25,
            5.9,
            direction * 0.3,
            direction * 0.7,
            direction * 0.4,
            1e-7,
        )
        .unwrap();
        let report = strip.validate().unwrap();
        assert_eq!(report.face_count, 3);
        assert_eq!(report.body_count, 0);
        assert_eq!(strip.edges.len(), 10);
        assert_eq!(strip.vertices.len(), 6);
        assert_eq!(strip.edges.iter().filter(|e| e.degenerate).count(), 2);
        assert_eq!(report.boundary_edge_count, 6);
        let uses: Vec<_> = (0..strip.edges.len())
            .map(|i| {
                strip
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == i)
                    .count()
            })
            .collect();
        assert_eq!(uses.iter().filter(|&&n| n == 2).count(), 2);
    }
}

#[test]
fn cylinder_trim_uses_contact_rail_as_natural_ruled_boundary() {
    for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
        let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, -0.7).unwrap();
        let sheet = span.trimmed_cylinder_sheet(1e-7).unwrap();
        assert_eq!(sheet.validate().unwrap().boundary_edge_count, 4);
        let mut bad = sheet.clone();
        bad.0.loops[0].coedges[2].pcurve.control_points[1][1] += 0.01;
        assert!(bad.validate().is_err());
        for i in 0..=40 {
            let t = i as f64 / 40.;
            let uv = sheet.loops[0].coedges[2].pcurve.evaluate(t).unwrap().point;
            let actual = sheet.faces[0].surface.evaluate(uv[0], uv[1]).unwrap().point;
            let expected = sheet.edges[2].curve.evaluate(t).unwrap().point;
            assert!((0..3).all(|k| (actual[k] - expected[k]).abs() < 1e-10));
        }
    }
}

#[test]
fn plane_chart_contains_quarter_arc_controls_beyond_nominal_radius() {
    let outer=arc(20.,6.,std::f64::consts::PI,std::f64::consts::FRAC_PI_2);
    let sheet=trimmed_plane(&outer,&outer,20.,6.,[0.;4],5.,1e-7).unwrap();let face=&sheet.faces[0];
    for use_ in &sheet.loops[face.outer].coedges {
        let r=nurbs_core::curve_surface_agreement::verify_exact(&sheet.edges[use_.edge].curve,&use_.pcurve,&face.surface,use_.reversed,10000).unwrap().unwrap();
        assert_eq!(r.outcome,cad_predicates::BezierIdentity::Equal,"edge {}: {r:?}",use_.edge);
    }
}
#[test]
fn constant_rim_contact_rails_equal_surface_boundary_controls() {
    for direction in [-1.,1.] {for radius in [0.25,1.25,2.5] {
        let span=plane_cylinder_rim(20.,6.,radius,0.3,direction*0.7).unwrap().remove(0);
        for (v,curve) in [(0.,&span.plane_contact),(1.,&span.cylinder_contact)] {
            let p=nurbs_core::curve::Curve::from_polyline(vec![vec![0.,v],vec![1.,v]]).unwrap();
            let r=nurbs_core::curve_surface_agreement::verify_exact(curve,&p,&span.surface,false,10000).unwrap().unwrap();
            assert_eq!(r.outcome,cad_predicates::BezierIdentity::Equal,"radius {radius} v={v}: {r:?}");
        }
    }}
}
#[test]
fn cylinder_vertical_rails_lift_exactly_in_height_coordinates() {
    for direction in [-1.,1.] {
        let span=plane_cylinder_transition(20.,6.,0.,1.25,0.,direction*0.7).unwrap();
        let sheet=span.trimmed_cylinder_sheet(1e-7).unwrap();let face=&sheet.faces[0];
        for i in [1,3] {
            let use_=&sheet.loops[face.outer].coedges[i];
            let r=nurbs_core::curve_surface_agreement::verify_exact(&sheet.edges[use_.edge].curve,&use_.pcurve,&face.surface,use_.reversed,1_000_000).unwrap().unwrap();
            assert_eq!(r.outcome,cad_predicates::BezierIdentity::Equal,"vertical rail {i}: {r:?}");
        }
    }
}
#[test]
fn plane_trim_lifts_authored_boundaries_exactly_without_uv_roundtrip() {
    for direction in [-1.,1.] {
        let span=plane_cylinder_transition(20.,6.,0.,1.25,0.,direction*0.7).unwrap();
        let sheet=span.trimmed_plane_sheet(5.,1e-7).unwrap();
        let face=&sheet.faces[0];
        for use_ in &sheet.loops[face.outer].coedges {
            let result=nurbs_core::curve_surface_agreement::verify_exact(&sheet.edges[use_.edge].curve,&use_.pcurve,&face.surface,use_.reversed,1_000_000).unwrap().unwrap();
            assert_eq!(result.outcome,cad_predicates::BezierIdentity::Equal,"edge {}: {result:?}",use_.edge);
        }
    }
}
#[test]
fn plane_trim_maps_all_boundaries_and_rejects_contact_overlap() {
    for direction in [-1., 1.] {
        for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
            let span =
                plane_cylinder_transition(20., 6., r0, r1, 5.9, direction * 0.7).unwrap();
            let sheet = span.trimmed_plane_sheet(5., 1e-7).unwrap();
            assert_eq!(sheet.validate().unwrap().boundary_edge_count, 4);
            for coedge in &sheet.loops[0].coedges {
                for i in 0..=40 {
                    let t = i as f64 / 40.;
                    let uv = coedge.pcurve.evaluate(t).unwrap().point;
                    let actual = sheet.faces[0].surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let expected = sheet.edges[coedge.edge].curve.evaluate(t).unwrap().point;
                    assert!((0..3).all(|k| (actual[k] - expected[k]).abs() < 1e-10));
                }
            }
            assert!(span.trimmed_plane_sheet(19., 1e-7).is_err());
            assert!(span.trimmed_plane_sheet(0., 1e-7).is_err());
        }
    }
}

#[test]
fn neighbor_regions_share_plane_and_cylinder_contact_edges() {
    for direction in [-1., 1.] {
        for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
            let span =
                plane_cylinder_transition(20., 6., r0, r1, 5.9, direction * 0.7).unwrap();
            let region = span.trimmed_support_region(5., 1e-7).unwrap();
            let report = region.validate().unwrap();
            assert_eq!(report.face_count, 3);
            assert_eq!(report.body_count, 0);
            assert_eq!(region.edges.len(), 10);
            assert_eq!(
                report.boundary_edge_count,
                if r0 == 0. || r1 == 0. { 7 } else { 8 }
            );
            let shared = (0..region.edges.len())
                .filter(|&i| {
                    region
                        .loops
                        .iter()
                        .flat_map(|l| &l.coedges)
                        .filter(|c| c.edge == i)
                        .count()
                        == 2
                })
                .count();
            assert_eq!(shared, 2);
        }
    }
}

#[test]
fn retained_strip_shares_all_internal_seams_and_keeps_only_outer_boundary() {
    for direction in [-1., 1.] {
        let region = circular_blend_retained_strip(
            20.,
            5.,
            6.,
            1.25,
            5.9,
            direction * 0.3,
            direction * 0.7,
            direction * 0.4,
            1e-7,
        )
        .unwrap();
        let report = region.validate().unwrap();
        assert_eq!(report.face_count, 9);
        assert_eq!(report.body_count, 0);
        assert_eq!(region.edges.len(), 24);
        assert_eq!(region.vertices.len(), 14);
        assert_eq!(report.boundary_edge_count, 10);
        assert_eq!(region.edges.iter().filter(|e| e.degenerate).count(), 2);
        let shared = (0..region.edges.len())
            .filter(|&i| {
                region
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == i)
                    .count()
                    == 2
            })
            .count();
        assert_eq!(shared, 12);
        for edge in 0..region.edges.len() {
            let senses: Vec<_> = region.shells[0]
                .faces
                .iter()
                .flat_map(|face_use| {
                    region.loops[region.faces[face_use.face].outer]
                        .coedges
                        .iter()
                        .filter(move |c| c.edge == edge)
                        .map(move |c| c.reversed ^ face_use.reversed)
                })
                .collect();
            assert!(senses.len() == 1 || senses.len() == 2);
            if senses.len() == 2 {
                assert_ne!(senses[0], senses[1]);
            }
        }
    }
}

#[test]
fn annular_sector_has_complete_walls_and_only_two_angular_cuts() {
    for direction in [-1., 1.] {
        for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
            let span =
                plane_cylinder_transition(20., 6., r0, r1, 5.9, direction * 0.7).unwrap();
            let sector = span.annular_side_sector(5., 1e-7).unwrap();
            let report = sector.validate().unwrap();
            assert_eq!(report.face_count, 5);
            assert_eq!(report.body_count, 0);
            assert_eq!(sector.edges.len(), 15);
            assert_eq!(
                report.boundary_edge_count,
                if r0 == 0. || r1 == 0. { 9 } else { 10 }
            );
            let shared = (0..sector.edges.len())
                .filter(|&i| {
                    sector
                        .loops
                        .iter()
                        .flat_map(|l| &l.coedges)
                        .filter(|c| c.edge == i)
                        .count()
                        == 2
                })
                .count();
            assert_eq!(shared, 5);
        }
    }
}

#[test]
fn annular_strip_keeps_only_the_two_unrounded_end_sections() {
    for direction in [-1., 1.] {
        let strip = circular_blend_annular_strip(
            20.,
            5.,
            6.,
            1.25,
            5.9,
            direction * 0.3,
            direction * 0.7,
            direction * 0.4,
            1e-7,
        )
        .unwrap();
        let report = strip.validate().unwrap();
        assert_eq!(report.face_count, 15);
        assert_eq!(report.body_count, 0);
        assert_eq!(strip.edges.len(), 35);
        assert_eq!(strip.vertices.len(), 18);
        assert_eq!(report.boundary_edge_count, 8);
        assert_eq!(strip.edges.iter().filter(|e| e.degenerate).count(), 2);
        for edge in 0..strip.edges.len() {
            let senses: Vec<_> = strip.shells[0]
                .faces
                .iter()
                .flat_map(|f| {
                    strip.loops[strip.faces[f.face].outer]
                        .coedges
                        .iter()
                        .filter(move |c| c.edge == edge)
                        .map(move |c| c.reversed ^ f.reversed)
                })
                .collect();
            assert!(senses.len() == 1 || senses.len() == 2);
            if senses.len() == 2 {
                assert_ne!(senses[0], senses[1]);
            }
        }
    }
}

#[test]
fn quarter_annular_blend_closes_without_welding_and_has_one_body() {
    for direction in [-1., 1.] {
        let model = partial_annular_quarter(20., 5., 6., 1.25, direction, 1e-7).unwrap();
        let report = model.validate().unwrap();
        assert_eq!(report.body_count, 1);
        assert_eq!(report.boundary_edge_count, 0);
        assert_eq!(model.faces.len(), 27);
        assert_eq!(model.edges.len(), 55);
        assert_eq!(model.vertices.len(), 26);
        assert_eq!(model.edges.iter().filter(|e| e.degenerate).count(), 2);
    }
}

#[test]
fn different_partial_arc_angles_close_with_shared_endpoints() {
    for sweep in [0.31, std::f64::consts::PI / 3., std::f64::consts::PI] {
        for sign in [-1., 1.] {
            let model = partial_annular_arc(20., 5., 6., 1.25, sign * sweep, 1e-7).unwrap();
            let report = model.validate().unwrap();
            assert_eq!(report.body_count, 1);
            assert_eq!(report.boundary_edge_count, 0);
            assert_eq!(model.edges.iter().filter(|e| e.degenerate).count(), 2);
        }
    }
    for sweep in [0., f64::NAN, 4.] {
        assert!(partial_annular_arc(20., 5., 6., 1.25, sweep, 1e-7).is_err());
    }
}

#[test]
fn impossible_and_nonfinite_support_refuses() {
    for radius in [0., -1., 20., f64::NAN] {
        assert!(plane_cylinder_rim(20., 6., radius, 0., 1.).is_err());
    }
    assert!(plane_cylinder_rim(20., 6., 1., 0., 7.).is_err());
}

#[test]
fn source_preview_admits_only_four_outer_rim_arcs_and_preserves_source() {
    let source = crate::tube(20., 5., 6.).unwrap();
    let original = format!("{source:?}");
    let mut admitted = 0;
    for edge in 0..source.edges.len() {
        if let Ok(result) =
            crate::analytic_features::build_partial_annular_preview(&source, edge, 1.25)
        {
            assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
            assert_eq!(result.bodies.len(), 1);
            assert!(result.persistent_naming_complete());
            assert!(result.1.change_set.validate().is_ok());
            let face_changes: Vec<_> = result
                .1
                .change_set
                .changes
                .iter()
                .filter(|change| {
                    change.topo_kind == crate::TopoKind::Face && !change.parents.is_empty()
                })
                .collect();
            for (index, id) in result.1.faces.iter().enumerate() {
                let owners: Vec<_> = face_changes
                    .iter()
                    .filter(|change| change.children.contains(id))
                    .collect();
                if [0, 5, 10].contains(&index) {
                    assert!(owners.is_empty());
                } else {
                    assert_eq!(owners.len(), 1);
                    assert_eq!(owners[0].parents.len(), 1);
                    assert!(source.1.faces.contains(&owners[0].parents[0]));
                }
            }
            for source_face in &source.1.faces {
                assert!(
                    face_changes
                        .iter()
                        .any(|change| change.parents.contains(source_face))
                );
            }
            assert_eq!(source.1.bodies, result.1.bodies);
            let restored: crate::Model =
                value_codec::from_str(&value_codec::to_string(&result).unwrap()).unwrap();
            restored.validate().unwrap();
            assert_eq!(restored.1.bodies, source.1.bodies);
            assert!(restored.persistent_naming_complete());
            assert!(result.1.change_set.changes.iter().any(|change| change.kind
                == crate::ChangeKind::Persisted
                && change.topo_kind == crate::TopoKind::Body
                && change.parents == source.1.bodies
                && change.children == result.1.bodies));
            for (index, vertex) in source.vertices.iter().enumerate() {
                let target = result
                    .vertices
                    .iter()
                    .position(|candidate| candidate.point == vertex.point)
                    .expect("original sharp endpoints must remain authored vertices");
                assert_eq!(source.1.vertices[index], result.1.vertices[target]);
            }
            assert_eq!(
                source
                    .1
                    .edges
                    .iter()
                    .filter(|id| result.1.edges.contains(id))
                    .count(),
                8
            );
            admitted += 1;
        }
    }
    assert_eq!(admitted, 4);
    assert_eq!(original, format!("{source:?}"));
    assert!(
        crate::analytic_features::build_partial_annular_preview(
            &source,
            source.edges.len(),
            1.25
        )
        .is_err()
    );
}

#[test]
fn source_preview_face_ownership_survives_rigid_placement() {
    let base = crate::tube(20., 5., 6.).unwrap();
    let (sa, ca) = 0.37_f64.sin_cos();
    let (sb, cb) = (-0.61_f64).sin_cos();
    for matrix in [
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
        [
            [ca * cb, -sa, ca * sb, 17.],
            [sa * cb, ca, sa * sb, -9.],
            [-sb, 0., cb, 23.],
            [0., 0., 0., 1.],
        ],
    ] {
        let source = crate::transform::affine(&base, matrix).unwrap();
        let original = format!("{source:?}");
        let mut admitted = 0;
        for edge in 0..source.edges.len() {
            if let Ok(result) =
                crate::analytic_features::build_partial_annular_preview(&source, edge, 1.25)
            {
                result.validate().unwrap();
                assert_eq!(source.1.bodies, result.1.bodies);
                let mut owned = 0;
                for id in &result.1.faces {
                    let owners: Vec<_> = result
                        .1
                        .change_set
                        .changes
                        .iter()
                        .filter(|c| {
                            c.topo_kind == crate::TopoKind::Face
                                && !c.parents.is_empty()
                                && c.children.contains(id)
                        })
                        .collect();
                    if !owners.is_empty() {
                        assert_eq!(owners.len(), 1);
                        assert_eq!(owners[0].parents.len(), 1);
                        assert!(source.1.faces.contains(&owners[0].parents[0]));
                        owned += 1;
                    }
                }
                assert_eq!(owned, 24);
                admitted += 1;
            }
        }
        assert_eq!(admitted, 4);
        assert_eq!(original, format!("{source:?}"));
    }
}
#[test]
fn moving_radius_relation_covers_full_constant_and_varying_support_patches() {
    for direction in [-1., 1.] {
        for span in plane_cylinder_rim(20., 6., 1.25, 0.3, direction * 0.7).unwrap() {
            let out = span.qualify_radius(1e-7, 100, 100_000_000).unwrap();
            assert!(
                out.certificate.is_some(),
                "{} {:?}",
                out.reason,
                out.error_upper
            );
            assert_eq!(out.certificate.unwrap().surface(), &span.surface);
        }
        for (a, b) in [(0.5, 1.25), (1.25, 0.5), (0., 1.25), (1.25, 0.)] {
            let span = plane_cylinder_transition(20., 6., a, b, 0.3, direction * 0.7).unwrap();
            let before = span.surface.clone();
            let tolerance = if a == 0. || b == 0. { 1e-6 } else { 1e-9 };
            let out = span.qualify_radius(tolerance, 100, 100_000_000).unwrap();
            eprintln!(
                "radii {a}->{b} direction {direction}: radius error {:?}, work {}",
                out.error_upper, out.work
            );
            assert!(
                out.certificate.is_some(),
                "{} {:?}",
                out.reason,
                out.error_upper
            );
            assert_eq!(span.surface, before);
            assert_eq!(out.certificate.unwrap().centers(), &span.centers);
        }
    }
}

#[test]
fn constant_rim_envelope_and_varying_section_have_distinct_normal_admission() {
    let limits = || nurbs_core::moving_envelope::Limits {
        max_sine_squared: 0.02,
        cells: 100000,
        surface_spans: 100000,
        center_spans: 100000,
        radial_work: 100_000_000,
    };
    for direction in [-1., 1.] {
        let span = plane_cylinder_rim(20., 6., 1.25, 0.3, direction * 0.7)
            .unwrap()
            .remove(0);
        let report = span
            .qualify_envelope(1e-7, 100, 100_000_000, limits())
            .unwrap()
            .normals
            .unwrap();
        eprintln!(
            "constant rim envelope direction {direction}: {} cells {} accepted {}",
            report.reason, report.cells, report.accepted_cells
        );
        assert!(
            report.envelope.is_some(),
            "{} {:?}",
            report.reason,
            report.uncertain_uv
        );
        assert_eq!(report.envelope.unwrap().radius().surface(), &span.surface);
        let stopped = span.qualify_envelope(1e-7, 100, 1, limits()).unwrap();
        assert!(stopped.normals.is_none());
        assert_eq!(stopped.radius.reason, "moving-radius-work-limit");
        assert!(stopped.radius.uncertain_uv.is_some());
        let transition =
            plane_cylinder_transition(20., 6., 0.5, 1.25, 0.3, direction * 0.7).unwrap();
        let p = transition.surface.evaluate(0.5, 0.5).unwrap();
        let normal = p.unit_normal().unwrap();
        let c = transition.centers.evaluate(0.5).unwrap().point;
        let radial = (0..3).map(|k| p.point[k] - c[k]).collect::<Vec<_>>();
        let length = radial.iter().map(|x| x * x).sum::<f64>().sqrt();
        let dot = (0..3).map(|k| normal[k] * radial[k] / length).sum::<f64>();
        let sine_squared = 1. - dot * dot;
        eprintln!("varying section midpoint sine squared {sine_squared}");
        let mut strict = limits();
        strict.max_sine_squared = 1e-4;
        assert!(sine_squared > strict.max_sine_squared);
        let report = transition
            .qualify_envelope(1e-9, 100, 100_000_000, strict)
            .unwrap()
            .normals
            .unwrap();
        eprintln!(
            "varying section envelope direction {direction}: {} cells {}",
            report.reason, report.cells
        );
        assert!(report.envelope.is_none());
        assert_eq!(report.reason, "moving-envelope-angular-break");
        assert!(report.uncertain_uv.is_some());
        let pole = plane_cylinder_transition(20., 6., 0., 1.25, 0.3, direction * 0.7).unwrap();
        let mut short = limits();
        short.cells = 32;
        let report = pole
            .qualify_envelope(1e-6, 100, 100_000_000, short)
            .unwrap()
            .normals
            .unwrap();
        assert!(report.envelope.is_none());
        assert!(report.uncertain_uv.is_some());
    }
}
