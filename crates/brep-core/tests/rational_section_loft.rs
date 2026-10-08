use brep_core::rational_section_loft;
use nurbs_core::{curve::Curve, surface::Axis};
fn circles(z: f64, scale: f64) -> Vec<Vec<Curve>> {
    vec![
        vec![nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 3. * scale).unwrap()],
        vec![
            nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], scale)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ]
}
#[test]
fn rational_hollow_loft_keeps_curved_walls_caps_and_shared_edges() {
    let model =
        rational_section_loft(&[circles(0., 1.), circles(5., 1.5), circles(10., 2.)]).unwrap();
    let report = model.validate().unwrap();
    assert_eq!(report.boundary_edge_count, 0);
    assert_eq!(model.faces.len(), 18);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.shells[0].closed);
    assert_eq!(
        model.faces.iter().filter(|f| !f.holes.is_empty()).count(),
        2
    );
    for face in &model.faces[..16] {
        assert_eq!(face.surface.degree_u, 2);
        assert!(face.surface.weights.iter().flatten().any(|w| *w != 1.));
        for (u, v) in [(0.13, 0.27), (0.61, 0.83)] {
            let p = face.surface.evaluate(u, v).unwrap().point;
            let radius = p[0].hypot(p[1]);
            let factor = 1. + p[2] / 10.;
            assert!((radius / factor - 1.).abs() < 1e-12 || (radius / factor - 3.).abs() < 1e-12);
        }
    }
    // Authoritative edge/pcurve agreement is independently checked by validate;
    // every undirected edge must have two incident coedges after capping.
    for edge in 0..model.edges.len() {
        assert_eq!(
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    let cap = &model.faces[16];
    assert_eq!(cap.holes.len(), 1);
    let outer = &model.loops[cap.outer];
    assert!(
        outer
            .coedges
            .iter()
            .any(|c| model.edges[c.edge].curve.degree == 2)
    );
    assert_eq!(
        model.faces[0].surface.iso(Axis::V, 0.).unwrap().weights,
        vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
    );
}
#[test]
fn invalid_holes_joins_correspondence_and_nonplanar_caps_refuse() {
    let start = circles(0., 1.);
    let end = circles(10., 1.);
    let mut wrong = start.clone();
    wrong[1][0] = wrong[1][0].reverse().unwrap();
    assert!(rational_section_loft(&[wrong, end.clone()]).is_err());
    let mut outside = start.clone();
    for p in &mut outside[1][0].control_points {
        p[0] += 10.;
    }
    assert!(rational_section_loft(&[outside, end.clone()]).is_err());
    let mut broken = start.clone();
    broken[0][0].control_points[0][0] += 0.01;
    assert!(rational_section_loft(&[broken, end.clone()]).is_err());
    let mut nonplanar = start.clone();
    nonplanar[0][0].control_points[1][2] = 0.1;
    assert!(rational_section_loft(&[nonplanar, end.clone()]).is_err());
    let mut weight = end.clone();
    weight[0][0].weights[1] = -1.;
    assert!(rational_section_loft(&[start.clone(), weight]).is_err());
    assert!(rational_section_loft(&[start]).is_err());
}
fn law(a: f64, b: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
#[test]
fn progressive_rational_hollow_body_caps_only_accepted_transport() {
    use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 16,
        max_deviation: 0.001,
    };
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let (body, evidence) = brep_core::progressive_profile_body(
        &circles(0., 1.),
        &path,
        &law(1., 2.),
        &law(0., 0.1),
        options,
    )
    .unwrap();
    assert!(evidence.levels.last().unwrap().accepted);
    assert!(evidence.levels.len() > 1);
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(body.faces.iter().filter(|f| !f.holes.is_empty()).count(), 2);
    assert!(
        brep_core::progressive_profile_body(
            &circles(0., 1.),
            &path,
            &law(1., 2.),
            &law(0., 0.1),
            Options {
                max_sections: 3,
                ..options
            }
        )
        .is_err()
    );
}
#[test]
fn retained_hollow_rational_loft_step_roundtrip_preserves_topology_and_surfaces() {
    let model = rational_section_loft(&[circles(0., 1.), circles(10., 2.)]).unwrap();
    let (text, _, _) = brep_core::export_step_v5(&model).unwrap();
    assert!(text.contains("RATIONAL_B_SPLINE_SURFACE"));
    let (restored, _, _) = brep_core::import_step_v5(&text).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(
        restored
            .faces
            .iter()
            .filter(|f| !f.holes.is_empty())
            .count(),
        2
    );
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        for (uf, vf) in [(0.13, 0.27), (0.61, 0.83)] {
            let [ua,ub]=[a.surface.knots_u[a.surface.degree_u],a.surface.knots_u[a.surface.control_points.len()]];
            let [va,vb]=[a.surface.knots_v[a.surface.degree_v],a.surface.knots_v[a.surface.control_points[0].len()]];
            let u=ua+(ub-ua)*uf;let v=va+(vb-va)*vf;
            let original_u = if model.shells[0]
                .faces
                .iter()
                .find(|f| f.face == i)
                .unwrap()
                .reversed
            {
                ua+ub-u
            } else {
                u
            };
            let p = a.surface.evaluate(original_u, v).unwrap().point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-10), "STEP changed face {i}");
        }
    }
}
#[test]
fn bent_path_preserves_rational_hollow_section_and_nonparallel_caps() {
    use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
    let boundaries = vec![
        vec![nurbs_core::primitives::circle([10., 0., 0.], [0., 1., 0.], 0.3).unwrap()],
        vec![
            nurbs_core::primitives::circle([10., 0., 0.], [0., 1., 0.], 0.1)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ];
    let path = nurbs_core::primitives::circle_arc([0.; 3], [0., 0., 1.], 10., 0., 45.).unwrap();
    let (body, approx) = brep_core::progressive_profile_body(
        &boundaries,
        &path,
        &law(1., 1.),
        &law(0., 0.),
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 16,
            max_deviation: 0.02,
        },
    )
    .unwrap();
    assert!(approx.levels.last().unwrap().accepted);
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
    let (text, _, _) = brep_core::export_step_v5(&body).unwrap();
    let (restored, _, _) = brep_core::import_step_v5(&text).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), body.faces.len());
    let caps = &body.faces[body.faces.len() - 2..];
    assert!(caps.iter().all(|f| f.holes.len() == 1));
    let center=|s:&nurbs_core::surface::Surface|s.evaluate(
        (s.knots_u[s.degree_u]+s.knots_u[s.control_points.len()])*0.5,
        (s.knots_v[s.degree_v]+s.knots_v[s.control_points[0].len()])*0.5).unwrap();
    let a = center(&caps[0].surface);
    let b = center(&caps[1].surface);
    assert!(a.point.iter().zip(b.point).any(|(x, y)| (x - y).abs() > 1.));
}

#[test]
fn rational_topology_supports_more_than_sixteen_sections_without_refit() {
    let sections = (0..33)
        .map(|i| circles(i as f64, 1. + i as f64 / 32.))
        .collect::<Vec<_>>();
    let model = rational_section_loft(&sections).unwrap();
    assert_eq!(model.faces.len(), 258);
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn reverse_travel_keeps_consistent_outward_shell_senses() {
    let up = rational_section_loft(&[circles(0., 1.), circles(10., 1.)]).unwrap();
    let down = rational_section_loft(&[circles(10., 1.), circles(0., 1.)]).unwrap();
    assert!(
        up.shells[0]
            .faces
            .iter()
            .zip(&down.shells[0].faces)
            .all(|(a, b)| a.reversed != b.reversed)
    );
    assert_eq!(down.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn natural_capped_loft_interpolates_middle_section_and_curved_shared_seams() {
    let sections = [circles(0., 1.), circles(5., 2.), circles(10., 1.)];
    let model = brep_core::natural_section_loft(&sections, &[0., 0.5, 1.]).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.shells[0].closed);
    assert_eq!(model.faces.len(), 10);
    for face in &model.faces[..8] {
        assert_eq!(face.surface.degree_v, 3);
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let p = face.surface.evaluate(u, 0.5).unwrap().point;
            assert!((p[2] - 5.).abs() < 1e-12);
            let r = p[0].hypot(p[1]);
            assert!((r - 6.).abs() < 1e-12 || (r - 2.).abs() < 1e-12);
        }
    }
    assert!(brep_core::natural_section_loft(&sections, &[0., 0., 1.]).is_err());
}

#[test]
fn capped_supplied_loft_patches_share_full_curved_seams() {
    let sections = [circles(0., 1.), circles(5., 2.), circles(10., 1.)];
    let prepared = sections
        .iter()
        .map(|s| {
            s.iter()
                .map(|ring| {
                    ring.iter()
                        .flat_map(|c| {
                            c.decompose()
                                .unwrap()
                                .into_iter()
                                .map(|p| p.definition().clone())
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let sides = prepared[0]
        .iter()
        .enumerate()
        .map(|(r, ring)| {
            ring.iter()
                .enumerate()
                .map(|(i, _)| {
                    let curves = prepared
                        .iter()
                        .map(|s| {
                            let mut c = s[r][i].clone();
                            let [a, b] = c.domain();
                            for k in &mut c.knots {
                                *k = (*k - a) / (b - a);
                            }
                            c
                        })
                        .collect::<Vec<_>>();
                    nurbs_core::natural_loft::interpolate(&curves, &[0., 0.5, 1.]).unwrap()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let model = brep_core::capped_loft_surfaces(&sections[0], &sections[2], &sides).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    let mut broken = sides.clone();
    broken[0][0].control_points[0][0][2] += 1.;
    assert!(brep_core::capped_loft_surfaces(&sections[0], &sections[2], &broken).is_err());
}
#[test]
fn affine_progressive_body_retains_elliptical_holes_and_authored_endpoint_laws() {
    use nurbs_core::progressive_sweep::{Options, Orientation, Spacing, constant_vector_law};
    let mut axes = constant_vector_law([1.; 3]).unwrap();
    axes.control_points[1] = vec![2., 1., 1.];
    let mut center = constant_vector_law([0.; 3]).unwrap();
    center.control_points[1] = vec![0.5, 0., 0.];
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let (body, approx) = brep_core::progressive_affine_profile_body(
        &circles(0., 1.),
        &path,
        &law(1., 2.),
        &law(0., 0.1),
        &axes,
        &center,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 128,
            max_deviation: 0.005,
        },
    )
    .unwrap();
    assert!(approx.levels.last().unwrap().accepted);
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
    assert!(
        body.faces[body.faces.len() - 2..]
            .iter()
            .all(|f| f.holes.len() == 1)
    );
    let p = body.vertices[body.vertices.len() - 8].point;
    let (sin, cos) = 0.1_f64.sin_cos();
    for (a, b) in p.iter().zip([12.5 * cos, 12.5 * sin, 10.]) {
        assert!((a - b).abs() < 1e-12);
    }
}

#[test]
fn natural_capped_loft_step_roundtrip_and_independent_fixture() {
    let model = brep_core::natural_section_loft(
        &[circles(0., 1.), circles(5., 2.), circles(10., 1.)],
        &[0., 0.5, 1.],
    )
    .unwrap();
    let (text, _, _) = brep_core::export_step_v9(&model).unwrap();
    let restored = brep_core::import_step_v9(&text).unwrap().0;
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.bodies.len(), 1);
    assert_eq!(restored.faces.len(), model.faces.len());
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        let reverse = model.shells[0]
            .faces
            .iter()
            .find(|f| f.face == i)
            .unwrap()
            .reversed;
        for (uf, vf) in [(0.13, 0.27), (0.61, 0.83)] {
            let [ua,ub]=[a.surface.knots_u[a.surface.degree_u],a.surface.knots_u[a.surface.control_points.len()]];
            let [va,vb]=[a.surface.knots_v[a.surface.degree_v],a.surface.knots_v[a.surface.control_points[0].len()]];
            let u=ua+(ub-ua)*uf;let v=va+(vb-va)*vf;
            let p = a
                .surface
                .evaluate(if reverse { ua+ub-u } else { u }, v)
                .unwrap()
                .point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-9));
        }
    }
    if let Ok(dir) = std::env::var("ADVANCED_LOFT_STEP_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/natural-hollow-solid.step"), text).unwrap();
    }
}

#[test]
fn differing_section_weights_retain_rational_sections_and_closed_topology() {
    let mut sections = [circles(0., 1.), circles(5., 1.2), circles(10., 1.5)];
    for (j, section) in sections.iter_mut().enumerate() {
        for ring in section {
            // Change the conic shape, not just its homogeneous scale.
            ring[0].weights[1] *= 1. + j as f64 / 8.;
        }
    }
    let ruled = rational_section_loft(&sections).unwrap();
    assert_eq!(ruled.validate().unwrap().boundary_edge_count, 0);
    let natural = brep_core::natural_section_loft(&sections, &[0., 0.5, 1.]).unwrap();
    assert_eq!(natural.validate().unwrap().boundary_edge_count, 0);
    for (j, section) in sections.iter().enumerate() {
        let spans = section.iter().flat_map(|ring| ring[0].decompose().unwrap())
            .map(|s| s.definition().clone()).collect::<Vec<_>>();
        for (i, span) in spans.iter().enumerate() {
            for u in [0., 0.13, 0.5, 0.87, 1.] {
                let [a,b] = span.domain();
                let expected = span.evaluate(a + u * (b-a)).unwrap().point;
                let actual = natural.faces[i].surface.evaluate(u, j as f64 / 2.).unwrap().point;
                for k in 0..3 { assert!((actual[k]-expected[k]).abs() < 1e-11); }
            }
        }
    }
    let (step, _, _) = brep_core::export_step_v5(&natural).unwrap();
    assert!(step.contains("RATIONAL_B_SPLINE_SURFACE"));
}

#[test]
fn closed_progressive_hollow_body_owns_periodic_seam_and_inner_shell() {
    use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
    let loops = vec![
        vec![nurbs_core::primitives::circle([5., 0., 0.], [0., 1., 0.], 0.5).unwrap()],
        vec![
            nurbs_core::primitives::circle([5., 0., 0.], [0., 1., 0.], 0.2)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ];
    let path = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let (body, approx) = brep_core::progressive_profile_body(
        &loops,
        &path,
        &law(1., 1.),
        &law(0., 0.),
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 128,
            max_deviation: 0.02,
        },
    )
    .unwrap();
    let report = approx.levels.last().unwrap();
    assert!(report.accepted && report.closed_path);
    assert_eq!(body.vertices.len(), 8 * (report.sections - 1));
    assert_eq!(body.faces.len(), 8 * (report.sections - 1));
    assert!(body.faces.iter().all(|f| f.holes.is_empty()));
    assert_eq!(body.shells.len(), 2);
    assert_eq!(body.bodies[0].inner_shells, vec![1]);
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
    for edge in 0..body.edges.len() {
        assert_eq!(
            body.loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    let (text, _, _) = brep_core::export_step_v5(&body).unwrap();
    assert!(text.contains("BREP_WITH_VOIDS"));
    let (restored, _, _) = brep_core::import_step_v5(&text).unwrap();
    assert_eq!(restored.bodies[0].inner_shells.len(), 1);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
}
#[test]
fn periodic_wall_budget_uses_faces_without_reserving_caps() {
    use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
    let loops = vec![
        vec![nurbs_core::primitives::circle([5., 0., 0.], [0., 1., 0.], 0.5).unwrap()],
        vec![
            nurbs_core::primitives::circle([5., 0., 0.], [0., 1., 0.], 0.2)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ];
    let path = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let (body, approx) = brep_core::progressive_profile_body(
        &loops,
        &path,
        &law(1., 1.),
        &law(0., 0.),
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 129,
            max_deviation: 0.005,
        },
    )
    .unwrap();
    assert_eq!(approx.levels.last().unwrap().sections, 129);
    assert_eq!(body.faces.len(), brep_core::MAX_FACES);
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn authored_frames_keep_hollow_body_manifold_caps() {
    use nurbs_core::progressive_sweep::{Options,Orientation,Spacing,constant_vector_law};
    let path=nurbs_core::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let mut axis=constant_vector_law([0.,0.,1.]).unwrap();
    axis.control_points[1]=vec![0.,1.,1.];
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let axes=constant_vector_law([1.;3]).unwrap();
    let center=constant_vector_law([0.;3]).unwrap();
    let (model,evidence)=brep_core::progressive_authored_profile_body(&circles(0.,1.),
        &path,&scale,&twist,&axis,&normal,&axes,&center,Options {
        normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.005,
    }).unwrap();
    assert!(evidence.levels.last().unwrap().accepted);
    assert_eq!(model.validate().unwrap().boundary_edge_count,0);
    assert_eq!(model.faces.iter().filter(|f| !f.holes.is_empty()).count(),2);
    assert_eq!(model.bodies.len(),1);
}


#[test]
fn common_contact_anchor_retains_rational_outer_and_hole_body() {
    use nurbs_core::progressive_sweep::{MultiSweep,Options,Orientation,Spacing,constant_vector_law};
    let profiles=circles(0.,1.).into_iter().flatten().collect::<Vec<_>>();
    let path=nurbs_core::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let rail=nurbs_core::primitives::line([3.,0.,0.],[6.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let mut sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options {
        normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.001,
    }).unwrap().with_contact_guide(&rail,0,0.).unwrap();
    let mut admitted=None;
    for level in sweep.by_ref(){let level=level.unwrap();if level.report.accepted {admitted=Some(level.report);}}
    let report=admitted.unwrap();
    let transported=sweep.sections_at(report.sections).unwrap();
    for i in 0..report.sections {
        let f=i as f64/(report.sections-1) as f64;
        for (index,radius) in [(0,3.),(1,1.)] {
            let p=transported[index][i].evaluate(0.).unwrap().point;
            assert!((p[0]-radius*(1.+f)).abs()<1e-10);
            assert!(p[1].abs()<1e-10 && (p[2]-10.*f).abs()<1e-10);
            assert_eq!(transported[index][i].weights,profiles[index].weights);
        }
    }
    let sections=(0..report.sections).map(|i|vec![vec![transported[0][i].clone()],vec![transported[1][i].clone()]]).collect::<Vec<_>>();
    let model=rational_section_loft(&sections).unwrap();
    let axes=constant_vector_law([1.;3]).unwrap();
    let center=constant_vector_law([0.;3]).unwrap();
    let loops=vec![vec![profiles[0].clone()],vec![profiles[1].clone()]];
    let (public,evidence)=brep_core::progressive_guided_profile_body(&loops,&path,&scale,&twist,
        &rail,Some((0,0.)),&axes,&center,Options {
        normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.001,
    }).unwrap();
    assert!(evidence.levels.last().unwrap().accepted);
    assert_eq!(public.faces.len(),model.faces.len());
    for (a,b) in public.faces.iter().zip(&model.faces) {
        assert_eq!(a.surface.control_points,b.surface.control_points);
        assert_eq!(a.surface.weights,b.surface.weights);
    }
    assert_eq!(public.validate().unwrap().boundary_edge_count,0);

    assert_eq!(model.validate().unwrap().boundary_edge_count,0);
    assert_eq!(model.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    assert!(model.shells[0].closed);
    let (step,_,_)=brep_core::export_step_v5(&model).unwrap();
    let (restored,_,_)=brep_core::import_step_v5(&step).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count,0);
    assert_eq!(restored.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    assert_eq!(restored.faces.len(),model.faces.len());
}

#[test]
fn cap_projection_certifies_within_existing_coordinate_limits() {
    let section = |origin: f64, z: f64| {
        let points = [[origin, origin, z], [origin + 1., origin + 1., z],
            [origin, origin + 2., z], [origin - 1., origin + 1., z]];
        vec![(0..4).map(|i| Curve {
            degree: 1, knots: vec![0.,0.,1.,1.],
            control_points: vec![points[i].to_vec(),points[(i+1)%4].to_vec()],
            weights: vec![1.,1.],periodic:false,
        }).collect::<Vec<_>>()]
    };
    rational_section_loft(&[section(0.,0.),section(0.,10.)]).unwrap();
    rational_section_loft(&[section(9e5,0.),section(9e5,10.)]).unwrap();
}


#[test]
fn fixed_original_transport_bound_reaches_hollow_brep_and_step_without_global_promotion(){
    use nurbs_core::progressive_sweep::{Options,Orientation,Spacing};
    let path=nurbs_core::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=law(1.,2.);let twist=law(0.,0.25);
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:129,max_deviation:0.01};
    let (body,evidence)=brep_core::progressive_profile_body(&circles(0.,1.),&path,&scale,&twist,options).unwrap();
    let report=evidence.levels.last().unwrap();
    assert!(report.accepted&&report.continuous_bound);
    assert!(report.continuous_error_upper.unwrap()<=options.max_deviation);
    assert!(report.error_certificate_cells<=10000);
    assert_eq!(body.validate().unwrap().boundary_edge_count,0);
    assert_eq!(body.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    let (step,_,_)=brep_core::export_step_v5(&body).unwrap();
    let (restored,_,_)=brep_core::import_step_v5(&step).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count,0);
    assert_eq!(restored.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    assert!(brep_core::progressive_profile_body(&circles(0.,1.),&path,&scale,&twist,
        Options {max_deviation:1e-30,..options}).is_err());
}


#[test]
fn fixed_normal_original_transport_bound_reaches_hollow_brep_and_step_without_global_promotion(){
    use nurbs_core::progressive_sweep::{Options,Orientation,Spacing};
    let path=nurbs_core::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=law(1.,2.);let twist=law(0.,0.25);
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::FixedNormal,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:129,max_deviation:0.01};
    let (body,evidence)=brep_core::progressive_profile_body(&circles(0.,1.),&path,&scale,&twist,options).unwrap();
    let report=evidence.levels.last().unwrap();
    assert!(report.accepted&&report.continuous_bound);
    assert!(report.continuous_error_upper.unwrap()<=options.max_deviation);
    assert!(report.error_certificate_cells<=10000);
    assert_eq!(body.validate().unwrap().boundary_edge_count,0);
    assert_eq!(body.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    let (step,_,_)=brep_core::export_step_v5(&body).unwrap();
    let (restored,_,_)=brep_core::import_step_v5(&step).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count,0);
    assert_eq!(restored.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    assert!(brep_core::progressive_profile_body(&circles(0.,1.),&path,&scale,&twist,
        Options {max_deviation:1e-30,..options}).is_err());
}

#[test]
fn frenet_curved_hollow_body_retains_original_error_and_native_step_topology(){
    use nurbs_core::progressive_sweep::{Options,Orientation,Spacing};
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let profiles=vec![
        vec![nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.1).unwrap()],
        vec![nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.05).unwrap().reverse().unwrap()],
    ];
    let scale=law(1.,1.);let twist=law(0.,0.);
    let options=Options {normal:[0.,0.,1.],orientation:Orientation::Frenet,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:129,max_deviation:0.01};
    let (body,evidence)=brep_core::progressive_profile_body(&profiles,&path,&scale,&twist,options).unwrap();
    let report=evidence.levels.last().unwrap();
    assert!(report.accepted&&report.continuous_bound);
    assert!(report.continuous_error_upper.unwrap()<=options.max_deviation);
    assert!(report.error_certificate_cells<=10000);
    assert_eq!(body.validate().unwrap().boundary_edge_count,0);
    assert_eq!(body.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    let (step,_,_)=brep_core::export_step_v5(&body).unwrap();
    let (restored,_,_)=brep_core::import_step_v5(&step).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count,0);
    assert_eq!(restored.faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    assert!(brep_core::progressive_profile_body(&profiles,&path,&scale,&twist,
        Options {max_deviation:1e-30,..options}).is_err());
}
