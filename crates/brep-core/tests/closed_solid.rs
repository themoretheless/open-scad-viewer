use brep_core::{closed_solid as solid, healing_report::{AutoRepairOperation, HealingStatus}};

#[path = "common/full_periodic_cylinder.rs"]
mod full_periodic_cylinder;

#[test]
fn closed_material_ray_length_clips_cavities_and_nonunit_direction() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let cavity = brep_core::minkowski3d::curved::certify_enclosed_difference(&outer, &inner, 0)
        .unwrap().into_model();
    let body = solid::qualify(cavity, policy()).unwrap().into_solid().unwrap();
    let origin = [-1., 2., 2.];
    let direction = [2., 0., 0.];
    for (end, expected) in [(0.25, 0.), (0.75, 0.5), (1.5, 1.), (2.25, 1.5), (3., 2.)] {
        let [lo, hi] = body.ray_material_length_mm(origin, direction, end, 100_000, 200_000).unwrap();
        assert!(lo <= expected && expected <= hi, "{lo}..{hi} vs {expected}");
        assert!(hi - lo < 1e-6);
    }
    assert_eq!(body.ray_material_length_mm(origin, direction, 0., 100_000, 200_000).unwrap(), [0.; 2]);
    assert!(body.ray_material_length_mm(origin, direction, -1., 100_000, 200_000).is_err());
    assert!(body.ray_material_length_mm(origin, direction, 3., 1, 1).is_err());
}

#[test]
fn closed_material_ray_preserves_entry_and_exit_enclosures() {
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let solid = solid::qualify(model, policy()).unwrap().into_solid().unwrap();
    let direction = [1., 0., 0.];
    for (origin, expected) in [([-1., 0.75, 1.25], [1., 3.]), ([1., 0.75, 1.25], [0., 1.])] {
        let intervals = solid.ray_material_intervals(origin, direction, 100_000, 200_000).unwrap();
        assert_eq!(intervals.len(), 1);
        for ([lo, hi], expected) in intervals[0].into_iter().zip(expected) {
            assert!(lo <= expected && expected <= hi);
        }
        if expected[0] == 0. {
            assert_eq!(intervals[0][0], [0., 0.]);
        }
    }
    assert!(solid.ray_material_intervals([-1., 3., 1.], direction, 100_000, 200_000).unwrap().is_empty());
    assert!(solid.ray_material_intervals([-1., 0., 1.], direction, 100_000, 200_000).is_err());
    assert!(solid.ray_material_intervals([-1., 0.75, 1.25], direction, 1, 1).is_err());

    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let cavity = brep_core::minkowski3d::curved::certify_enclosed_difference(&outer, &inner, 0)
        .unwrap().into_model();
    let cavity = solid::qualify(cavity, policy()).unwrap().into_solid().unwrap();
    let intervals = cavity.ray_material_intervals([-1., 2., 2.], direction, 100_000, 200_000).unwrap();
    assert_eq!(intervals.len(), 2);
    for (interval, expected) in intervals.into_iter().zip([[1., 2.], [4., 5.]]) {
        for ([lo, hi], expected) in interval.into_iter().zip(expected) {
            assert!(lo <= expected && expected <= hi);
        }
    }
}

#[test]
fn periodic_cylinder_qualifies_as_closed_material_through_its_owned_rims() {
    let mut model = full_periodic_cylinder::source();
    let edge = &model.loops[0].coedges[1];
    let trim = brep_topology::CoedgeTrim {
        curve_parameter: model.edges[edge.edge].curve.domain(),
        pcurve_parameter: edge.pcurve.domain(),
        periodic_lift: [[1, 0]; 2],
    };
    model.loops[0].coedges[1] = edge.clone().with_trim(trim).unwrap();
    model.rebuild_topology_ids();
    let limited = solid::qualify(model.clone(), solid::AdmissionPolicy {
        max_linear_cells: 1,
        ..policy()
    }).unwrap();
    assert!(!limited.geometry().boundary.intersections.faces.faces[0].periodic_quotient);
    let audit = solid::qualify(model, policy()).unwrap();
    let boundary = &audit.geometry().boundary;
    assert!(boundary.trim.all_valid);
    assert!(boundary.agreement.all_joins_exact);
    assert!(boundary.agreement.all_equal, "{:?}", boundary.agreement.uses);
    assert!(boundary.intersections.faces.faces[0].periodic_quotient);
    assert!(boundary.intersections.faces.all_faces_injective);
    assert!(
        boundary.intersections.pairs.all_pairs_classified,
        "{:?}",
        boundary.intersections.pairs.pairs
    );
    for (faces, edge) in [([0, 1], 0), ([0, 2], 1)] {
        let contact = boundary.hull_contacts.iter().find(|proof| proof.faces == faces).unwrap();
        assert_eq!(contact.edges, [edge]);
    }
    let body = audit.into_solid().expect("periodic cylinder admits closed material");
    let directions = [[0., 0., 1.], [1., 0.317, 0.173]];
    for (point, inside) in [([0., 0., 1.], true), ([2., 0., 1.], false)] {
        assert_eq!(
            body.classify_point(point, &directions, 10000, 100000).unwrap().parity,
            Some(inside)
        );
    }
}

#[path = "common/full_periodic_frustum.rs"]
mod full_periodic_frustum;

#[test]
fn periodic_frusta_and_cone_admit_affine_material() {
    use brep_core::transform::affine;
    let transform = [
        [1., 0., 1., 5.],
        [0., 1., 1., -3.],
        [1., 2., 4., 2.],
        [0., 0., 0., 1.],
    ];
    for (scale, inverted) in [(1., false), (0.5, false), (0.5, true), (0., false)] {
        let model = affine(&full_periodic_frustum::source(scale, inverted), transform).unwrap();
        let audit = solid::qualify(model, policy()).unwrap();
        let b = &audit.geometry().boundary;
        let body = audit.solid().unwrap_or_else(|| {
            let pairs = b.intersections.pairs.pairs.iter().map(|p| p.reason).collect::<Vec<_>>();
            panic!(
                "scale={scale}, inverted={inverted}, trim={}, exact={}, charts={}, pairs={pairs:?}",
                b.trim.all_valid, b.agreement.all_equal, b.intersections.faces.all_faces_injective,
            );
        });
        let directions = [[1., 1., 4.], [0.239, 1., 0.419]];
        for (point, inside) in [([6., -2., 6.], true), ([8., -2., 8.], false)] {
            assert_eq!(
                body.classify_point(point, &directions, 10000, 100000).unwrap().parity,
                Some(inside),
                "scale={scale}, inverted={inverted}, point={point:?}"
            );
        }
    }
}

#[path = "common/perforated_periodic_cylinder.rs"]
mod perforated_periodic_cylinder;

#[test]
fn a_partitioned_periodic_wall_admits_its_retained_curved_patch() {
    let model = perforated_periodic_cylinder::source([1., 2.], [0.25, 0.75]);
    let audit = solid::qualify(model, policy()).unwrap();
    let b = &audit.geometry().boundary;
    assert!(b.trim.all_valid);
    assert!(b.agreement.all_joins_exact);
    assert!(b.agreement.all_equal, "{:?}", b.agreement.uses);
    assert!(b.intersections.faces.all_faces_injective);
    assert!(b.intersections.pairs.all_pairs_classified, "{:?}", b.intersections.pairs.pairs);
    let contact = b.hull_contacts.iter().find(|contact| contact.faces == [0, 3]).unwrap();
    assert_eq!(contact.edges, [6, 5, 4, 3]);
    for cap in [1, 2] {
        assert!(b.intersections.pairs.pairs.iter().any(|p| p.faces == [cap, 3] && p.hull_disjoint));
    }
    let body = audit.into_solid().expect("partitioned periodic cylinder admits closed material");
    let directions = [[0., 0., 1.], [1., 0.317, 0.173]];
    for (point, inside) in [([0., 0., 1.], true), ([2., 0., 1.], false)] {
        assert_eq!(
            body.classify_point(point, &directions, 10000, 100000).unwrap().parity,
            Some(inside),
        );
    }
}

#[test]
fn exact_periodic_edge_lift_crosses_the_native_seam_in_both_directions() {
    use cad_predicates::BezierIdentity::{Different, Equal};
    use nurbs_core::{curve::Curve, curve_surface_agreement::verify_exact};

    let model = full_periodic_cylinder::source();
    let surface = &model.faces[0].surface;
    let profile = &model.edges[0].curve;
    let mut edge = Curve {
        degree: 2,
        knots: vec![3., 3., 3., 4., 4., 5., 5., 5.],
        control_points: [6, 7, 0, 1, 2].map(|i| {
            let mut p = profile.control_points[i].clone();
            p[2] = 1.;
            p
        }).to_vec(),
        weights: [6, 7, 0, 1, 2].map(|i| profile.weights[i]).to_vec(),
        periodic: false,
    };
    let pcurve = Curve::from_polyline(vec![vec![3., 0.5], vec![5., 0.5]]).unwrap();
    for tile in [-1., 0., 1.] {
        let mut lifted = pcurve.clone();
        for uv in &mut lifted.control_points {
            uv[0] += 4. * tile;
        }
        for reversed in [false, true] {
            let trace = if reversed { lifted.reverse().unwrap() } else { lifted.clone() };
            let proof = verify_exact(&edge, &trace, surface, reversed, 100000).unwrap().unwrap();
            assert_eq!(proof.outcome, Equal, "tile={tile}, reversed={reversed}");
        }
    }
    let proof = nurbs_core::curve_surface_agreement::verify_exact_trimmed(
        &edge, &pcurve, surface, false, [3.5, 4.5], [0.25, 0.75], 100000,
    ).unwrap().unwrap();
    assert_eq!(proof.outcome, Equal);
    let ruling = Curve::from_polyline(vec![vec![0., 1., 0.5], vec![0., 1., 1.5]]).unwrap();
    let lifted = Curve::from_polyline(vec![vec![5., 0.25], vec![5., 0.75]]).unwrap();
    let proof = verify_exact(&ruling, &lifted, surface, false, 100000).unwrap().unwrap();
    assert_eq!(proof.outcome, Equal);
    edge.control_points[3][2] += 0.25;
    let proof = verify_exact(&edge, &pcurve, surface, false, 100000).unwrap().unwrap();
    assert_eq!(proof.outcome, Different);
}

#[test]
fn paired_outer_seam_preserves_a_periodic_hole_across_both_sides() {
    use brep_core::face_domain::{FaceDomain, audit_trim_regions};
    use nurbs_core::trim_domain::Location;

    let model = perforated_periodic_cylinder::source([3., 5.], [0.25, 0.75]);
    let audit = audit_trim_regions(&model, 1e-8, 1000, 10000, 100000).unwrap();
    assert!(audit.all_valid, "{:?}", audit.faces.iter().map(|entry|
        entry.as_ref().map(|r| (r.valid, r.reason))).collect::<Vec<_>>());
    let exact = brep_core::boundary_agreement::verify_exact(&model, 1_000_000).unwrap();
    assert!(exact.all_equal && exact.all_joins_exact, "{:?}", exact.uses);
    let wall = FaceDomain::new(&model, 0, 1e-8).unwrap();
    let patch = FaceDomain::new(&model, 3, 1e-8).unwrap();
    for u in [0.5, 3.5] {
        let uv = [[u; 2], [0.5; 2]];
        assert_eq!(wall.classify(uv, 10000).unwrap().location, Location::Outside);
        assert_eq!(patch.classify(uv, 10000).unwrap().location, Location::Inside);
    }
    let uv = [[2.; 2], [0.5; 2]];
    assert_eq!(wall.classify(uv, 10000).unwrap().location, Location::Inside);
    assert_eq!(patch.classify(uv, 10000).unwrap().location, Location::Outside);
    assert!(!wall.on_linear_boundary([[4.; 2], [0., 0.25]]));
    let solid = solid::qualify(model.clone(), policy()).unwrap();
    let boundary = &solid.geometry().boundary;
    assert!(solid.solid().is_some(), "charts={:?}, pairs={:?}, orientation={:?}",
        boundary.intersections.faces.faces.iter().map(|f| f.absence_proven()).collect::<Vec<_>>(),
        boundary.intersections.pairs.pairs.iter().map(|p| (p.faces, p.reason)).collect::<Vec<_>>(),
        solid.geometry().orientations.iter().map(|o| o.outward).collect::<Vec<_>>());
    let rim = boundary.hull_contacts.iter().find(|contact| contact.faces == [0, 3]).unwrap();
    assert_eq!(rim.edges, [6, 5, 4, 3]);
    let body = solid.solid().unwrap();
    for (point, inside) in [([0., 0., 1.], true), ([2., 0., 1.], false)] {
        // The oblique ray exits through the lifted patch and avoids the UV seam.
        let report = body.classify_point(point, &[[1., 0.317, 0.173]], 10000, 100000).unwrap();
        assert_eq!(report.parity, Some(inside), "{report:?}");
    }
    let mut unpaired = model.clone();
    unpaired.loops[0].coedges[1].edge = 0;
    let err = FaceDomain::new(&unpaired, 0, 1e-8).err().unwrap();
    assert_eq!(err.code, "BREP_TRIM_DOMAIN");
}

#[test]
fn periodic_cylinder_intersects_a_stock_box_between_two_original_planes() {
    use brep_core::{root_model::{QualifiedRootShellPositiveFlux, RootPeriodicBandLimits, RootShellFluxLimits}, source_box};
    let policy = policy();
    let cylinder = solid::qualify(full_periodic_cylinder::source(), policy).unwrap().into_solid().unwrap();
    let stock_limits = source_box::Limits {
        operations: 100000,
        body: brep_core::source_body_restore::Limits {
            shell: brep_core::source_shell_restore::Limits {
                regions: brep_core::source_region_restore::Limits {
                    region: brep_core::trimmed_face_recipe::Limits {
                        pairs: 1000, region_cells: 10000, domain_cells: 100000, agreement_cells: 10000,
                    },
                    search_cells: 10000, point_checks: 10000, mapping_cells: 10000,
                    driver_cells: 10000, membership_cells: 10000, controls: 10000,
                    winding_cells: 10000, steps: 16,
                },
                exact_work: 1000000, driver_cells: 10000,
            },
            embedding: brep_core::source_shell_geometry::Limits {
                tolerance_uv: 1e-8, corners: 1000, spans: 10000, linear_cells: 10000,
                pairs: policy.geometry.boundary.contacts, exact_work: 1000000, driver_cells: 10000,
            },
            volume: brep_core::source_volume::Limits {
                axis: 2, origin: 0., absolute_error: 1e-6, tolerance_uv: 1e-8,
                cells: 10000, spans: 10000, domain_cells: 100000,
            },
        },
    };
    let stock_model = brep_core::cuboid([-2., -2., 0.5], [2., 2., 1.5]).unwrap();
    let stock = source_box::qualify(&stock_model, &stock_limits).unwrap();
    let limits = RootPeriodicBandLimits {
        max_chart_attempts: 64,
        max_orientation_probes: 128,
        max_chart_spans: 8192,
        max_contact_cells: 4096,
        flux: RootShellFluxLimits {
            axis: 2, origin: 0.5, max_cells: 2048, max_spans: 8192, max_depth: 16,
            winding: Default::default(),
        },
    };
    let result = QualifiedRootShellPositiveFlux::intersect_periodic_stock_band(
        &cylinder, &stock, Default::default(), limits,
    ).unwrap();
    let model = result.to_body().unwrap();
    assert_eq!(model.topology().bodies.len(), 1);
    assert!(model.topology().boundary_edges().is_empty());
    let [lo, hi] = result.report().signed_bounds;
    assert!(lo > 0. && lo <= std::f64::consts::PI && std::f64::consts::PI <= hi);
    let lateral_model = brep_core::cuboid([0., -2., 0.5], [2., 2., 1.5]).unwrap();
    let lateral = source_box::qualify(&lateral_model, &stock_limits).unwrap();
    assert!(QualifiedRootShellPositiveFlux::intersect_periodic_stock_band(
        &cylinder, &lateral, Default::default(), limits,
    ).is_err());
}

#[test]
fn public_boolean_clips_an_original_periodic_cylinder_at_a_lateral_box_face() {
    let source = full_periodic_cylinder::source();
    let cutter = brep_core::cuboid([0., -2., -1.], [2., 2., 3.]).unwrap();
    for op in ["intersection", "difference"] {
        let result = brep_core::boolean::operations::boolean(&source, &cutter, op)
            .unwrap_or_else(|e| panic!("{op}: {e:?}"));
        let audit = solid::qualify(result, policy()).unwrap();
        let b = &audit.geometry().boundary;
        let body = audit.solid().unwrap_or_else(|| panic!(
            "{op}: trim={}, exact={}, charts={}, pairs={:?}, uses={:?}, orientation={:?}",
            b.trim.all_valid, b.agreement.all_equal, b.intersections.faces.all_faces_injective,
            b.intersections.pairs.pairs.iter().map(|p| (p.faces,p.reason)).collect::<Vec<_>>(),
            b.agreement.uses.iter().filter(|r| !r.decision.as_ref().is_some_and(|d|
                d.outcome == cad_predicates::BezierIdentity::Equal)).map(|r| (r.face,r.coedge,r.edge)).collect::<Vec<_>>(),
            audit.geometry().orientations.iter().map(|r| r.outward).collect::<Vec<_>>(),
        ));
        let expected = std::f64::consts::PI;
        let volume = body.volume_bounds(solid::VolumeLimits {
            axis: 2, origin: 1., absolute_error: 0.06,
            cells: 100000, spans: 200000, domain_cells: 200000,
            algebra: Default::default(),
        }).unwrap();
        let [lo, hi] = volume.positive_volume
            .unwrap_or_else(|| panic!("{op} volume: {} {:?}", volume.reason, volume.signed_bounds));
        assert!(lo <= expected && expected <= hi, "{op}: [{lo}, {hi}]");
        for (x, inside) in [(0.5, op == "intersection"), (-0.5, op == "difference")] {
            assert_eq!(
                body.classify_point([x, 0., 1.], &[[0., 0., 1.]], 10000, 100000).unwrap().parity,
                Some(inside),
            );
        }
    }
}

#[test]
fn lateral_boolean_preserves_a_dyadic_interior_cut_on_periodic_quadratics() {
    let mut source = full_periodic_cylinder::source();
    for edge in &mut source.edges {
        edge.curve.weights.fill(1.);
    }
    for face in &mut source.faces {
        for row in &mut face.surface.weights {
            row.fill(1.);
        }
    }
    for wire in &mut source.loops {
        for coedge in &mut wire.coedges {
            coedge.pcurve.weights.fill(1.);
        }
    }
    let cutter = brep_core::cuboid([0.75, -2., -1.], [2., 2., 3.]).unwrap();
    let result = brep_core::operations::boolean(&source, &cutter, "intersection").unwrap();
    let audit = solid::qualify(result, policy()).unwrap();
    let boundary = &audit.geometry().boundary;
    let body = audit.solid().unwrap_or_else(|| panic!(
        "trim={}, exact={}, pairs={:?}", boundary.trim.all_valid, boundary.agreement.all_equal,
        boundary.intersections.pairs.pairs.iter().map(|p| p.reason).collect::<Vec<_>>(),
    ));
    let mass = brep_core::analysis::mass_properties(body.model(), 1e-6, 200000).unwrap();
    assert!((mass.signed_volume_mm3 - 13. / 24.).abs() < 1e-6);
    for (x, inside) in [(0.875, true), (0.5, false)] {
        assert_eq!(
            body.classify_point([x, 0., 1.], &[[0., 0., 1.]], 10000, 100000).unwrap().parity,
            Some(inside),
        );
    }
}

fn policy() -> solid::AdmissionPolicy {
    solid::AdmissionPolicy {
        tolerance_uv: 1e-8,
        max_linear_cells: 100000,
        geometry: brep_core::volume_validity::Limits {
            boundary: brep_core::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 1000,
                trim_cells: 10000,
                trim_domain_cells: 100000,
                spans: 100,
                contacts: brep_core::face_contacts::Limits {
                    pairs: 1000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 16,
                    domain_cells_per_pair: 1000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 10000,
            nesting_domain_cells: 100000,
            orientation_cells: 10000,
            orientation_domain_cells: 100000,
            orientation_spans: 100,
        },
    }
}

#[test]
fn open_ss_network_selects_boolean_material_inside_its_actual_descendants() {
    use brep_core::ss_face_membership as trim;
    use nurbs_core::{ss_intersection as ss, surface::Axis, surface_edit::elevate};
    let a = brep_core::cuboid([0., 0., -1.], [1., 1., 0.]).unwrap();
    let taller = brep_core::cuboid([0., 0., -2.], [1., 1., 0.]).unwrap();
    let mut b = brep_core::transform::affine(&taller, [
        [1., 0., 0., 0.], [0., 1., 0., 0.],
        [1., 0., 1., -0.5], [0., 0., 0., 1.],
    ]).unwrap();
    let bottom = elevate(&b.faces[0].surface, Axis::U, 2).unwrap();
    let mut bottom = elevate(&bottom, Axis::V, 2).unwrap();
    bottom.control_points[1][1][2] -= 0.25;
    b.faces[0].surface = bottom;
    b.faces[1].surface = elevate(&b.faces[1].surface, Axis::U, 2).unwrap();
    let certify_affine = nurbs_core::surface_injectivity::certify_affine_parameterization;
    assert!(certify_affine(&b.faces[0].surface).is_none());
    assert!(certify_affine(&b.faces[1].surface).is_some());
    b.rebuild_topology_ids();
    let [a, b] = [a, b].map(|model| solid::qualify(model, policy()).unwrap().into_solid().unwrap());
    let face = 1;
    let source = ss::FiberSources::new(&a.model().faces[face].surface, &b.model().faces[face].surface).unwrap();
    let branches = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let roots = trim::qualify_coedge_roots(
        &branches, [(a.model(), face), (b.model(), face)], 1e-8, Default::default(),
    ).unwrap();
    let reps = trim::qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = trim::partition_cut_representatives(&reps, Default::default()).unwrap();
    let network = trim::discover_open_ss_network(&input, 0, Default::default(), Default::default())
        .unwrap().unwrap();
    let directions = [[0., 0., 1.], [1., 0.317, 0.173], [0.239, 1., 0.419]];
    let query = solid::BooleanFace {
        solids: [&a, &b], face,
        side: solid::BooleanSourceSide::First, operation: solid::BooleanMaterialOp::Intersection,
    };
    assert_eq!(network.faces()[0].len(), 2);
    let automatic = network.discover_boolean_descendants(query, &directions, Default::default()).unwrap();
    let xor = solid::BooleanFace { operation: solid::BooleanMaterialOp::Xor, ..query };
    let automatic_xor = network.discover_boolean_descendants(xor, &directions, Default::default()).unwrap();
    assert_eq!(automatic.iter().map(|&(piece, _)| piece).collect::<Vec<_>>(), network.faces()[0]);
    assert_eq!(automatic_xor.len(), 2);
    assert!(network.discover_boolean_descendants(query, &directions, solid::BooleanFaceSeedLimits {
        domain_cells: 1, ..Default::default()
    }).is_err());
    for (u, expected) in [([0.125, 0.25], false), ([0.75, 0.875], true)] {
        let area = [u, [0.25, 0.75]];
        let reports: Vec<_> = network.faces()[0].iter().map(|&piece| {
            network.qualify_boolean_rectangle(query, piece, area, &directions, Default::default())
        }).collect();
        let mut proved = reports.iter().filter_map(|result| result.as_ref().ok());
        let report = proved.next().unwrap_or_else(|| panic!(
            "No material proof for {area:?}: {:?}",
            reports.iter().filter_map(|result| result.as_ref().err()).collect::<Vec<_>>(),
        ));
        assert!(proved.next().is_none());
        assert_eq!(report.retain(), expected);
        report.reverify().unwrap();
        let region: Vec<_> = network.faces()[0].iter().filter_map(|&piece| {
            network.qualify_boolean_descendant(query, piece, area, &directions, Default::default()).ok()
        }).collect();
        let disposition = if expected {
            solid::BooleanFaceDisposition::Keep
        } else {
            solid::BooleanFaceDisposition::Discard
        };
        assert_eq!(region, [disposition]);
        let piece = automatic.iter().find(|&&(_, result)| result == disposition).unwrap().0;
        assert_eq!(
            network.qualify_boolean_descendant(query, piece, area, &directions, Default::default()).unwrap(),
            disposition,
        );
        let xor_regions: Vec<_> = network.faces()[0].iter().filter_map(|&piece| {
            network.qualify_boolean_descendant(xor, piece, area, &directions, Default::default()).ok()
        }).collect();
        let xor_disposition = if expected {
            solid::BooleanFaceDisposition::Reverse
        } else {
            solid::BooleanFaceDisposition::Keep
        };
        assert_eq!(xor_regions, [xor_disposition]);
        assert!(automatic_xor.contains(&(piece, xor_disposition)));
    }
    for &piece in &network.faces()[0] {
        assert!(network.qualify_boolean_rectangle(
            query, piece, [[0.375, 0.625], [0.25, 0.75]], &directions, Default::default(),
        ).is_err());
    }
    let other = solid::BooleanFace { side: solid::BooleanSourceSide::Second, ..query };
    let selected = network.discover_boolean_descendants(other, &directions, Default::default()).unwrap();
    assert_eq!(selected.len(), 2);
    for (piece, disposition) in selected {
        let domain = brep_core::face_domain::FaceDomain::new(&network.models()[1], piece, 1e-8).unwrap();
        let left = domain.classify([[0.25; 2], [0.5; 2]], 10_000).unwrap();
        let expected = if left.location == nurbs_core::trim_domain::Location::Inside {
            solid::BooleanFaceDisposition::Keep
        } else {
            assert_eq!(left.location, nurbs_core::trim_domain::Location::Outside);
            solid::BooleanFaceDisposition::Discard
        };
        assert_eq!(disposition, expected);
        assert_eq!(network.models()[1].faces[piece].surface, b.model().faces[face].surface);
    }
    for operation in [solid::BooleanMaterialOp::Intersection, solid::BooleanMaterialOp::Union] {
        let seams = network.qualify_boolean_seams([&a, &b], operation, &directions, Default::default()).unwrap();
        assert_eq!(seams.len(), 1);
        let [left, right] = seams[0];
        assert_eq!([left.model, right.model], [0, 1]);
        assert_ne!(left.reversed, right.reversed);
        assert_eq!(network.models()[0].edges[left.edge].curve, network.models()[1].edges[right.edge].curve);
        let selection = network.discover_boolean_descendants(
            solid::BooleanFace { operation, ..query }, &directions, Default::default(),
        ).unwrap();
        assert!(selection.contains(&(left.face, solid::BooleanFaceDisposition::Keep)));
    }
    let xor_seams = network.qualify_boolean_seams(
        [&a, &b], solid::BooleanMaterialOp::Xor, &directions, Default::default(),
    ).unwrap();
    let difference = network.qualify_boolean_seams(
        [&a, &b], solid::BooleanMaterialOp::Difference, &directions, Default::default(),
    ).unwrap();
    assert_eq!(xor_seams.len(), 2);
    assert_eq!(&xor_seams[..1], difference);
    let other_xor = network.discover_boolean_descendants(
        solid::BooleanFace { side: solid::BooleanSourceSide::Second, ..xor },
        &directions, Default::default(),
    ).unwrap();
    for (index, [left, right]) in xor_seams.iter().enumerate() {
        assert_ne!(left.reversed, right.reversed);
        let [keep, reverse] = [solid::BooleanFaceDisposition::Keep, solid::BooleanFaceDisposition::Reverse];
        assert!(automatic_xor.contains(&(left.face, if index == 0 { keep } else { reverse })));
        assert!(other_xor.contains(&(right.face, if index == 0 { reverse } else { keep })));
    }
    for side in 0..2 {
        assert_eq!(xor_seams[0][side].edge, xor_seams[1][side].edge);
        assert_ne!(xor_seams[0][side].face, xor_seams[1][side].face);
    }
    for (operation, count) in [
        (solid::BooleanMaterialOp::Intersection, 1),
        (solid::BooleanMaterialOp::Union, 1),
        (solid::BooleanMaterialOp::Difference, 1),
        (solid::BooleanMaterialOp::Xor, 2),
    ] {
        let patches = network.sew_boolean_patches(
            [&a, &b], operation, &directions, Default::default(),
        ).unwrap();
        assert_eq!(patches.len(), count);
        for patch in patches {
            patch.validate().unwrap();
            assert_eq!(patch.faces.len(), 2);
            assert_eq!(patch.shells.len(), 1);
            assert!(!patch.shells[0].closed);
            let first = &patch.loops[patch.faces[0].outer].coedges;
            let second = &patch.loops[patch.faces[1].outer].coedges;
            let shared: Vec<_> = first.iter().filter(|c| second.iter().any(|d| d.edge == c.edge)).collect();
            assert_eq!(shared.len(), 1);
            assert_eq!(patch.edges[shared[0].edge].curve, network.models()[0].edges[xor_seams[0][0].edge].curve);
        }
    }
    let reference = brep_core::extrude_polygon(
        &[[0., -1.], [1., -1.], [1., 0.], [0.5, 0.], [0., -0.5]], 0., 1.,
    ).unwrap();
    let reference = brep_core::transform::affine(&reference, [
        [1., 0., 0., 0.], [0., 0., -1., 1.],
        [0., 1., 0., 0.], [0., 0., 0., 1.],
    ]).unwrap();
    let remaining: Vec<_> = reference.faces.iter().enumerate()
        .filter(|(_, face)| face.surface.control_points.iter().flatten().any(|p| p[2] == -1.))
        .map(|(index, _)| index).collect();
    assert_eq!(remaining.len(), 5);
    let completion = [open_patch(&reference, &remaining)];
    let shell = solid::ShellRepairPolicy {
        establish_forest: policy(), establish_orientation: policy(), readmit: policy(),
    };
    let assembled = network.assemble_selected_ss_solids(
        [&a, &b], solid::BooleanMaterialOp::Intersection, &directions, Default::default(),
        &[&completion], 32, shell,
    ).unwrap();
    assert_eq!(assembled.len(), 1);
    assert_eq!(assembled[0].model().faces.len(), 7);
    assert_eq!(assembled[0].model().bodies.len(), 1);
    assembled[0].reverify().unwrap();
    let volume = assembled[0].volume_bounds(solid::VolumeLimits {
        axis: 2, origin: 0., absolute_error: 1e-4, cells: 100_000, spans: 100_000,
        domain_cells: 100_000, algebra: Default::default(),
    }).unwrap().positive_volume.unwrap();
    assert!(volume[0] <= 0.875 && 0.875 <= volume[1]);
    let mut candidates = vec![brep_core::cuboid([8.; 3], [9.; 3]).unwrap()];
    candidates.extend(remaining.iter().map(|&face| open_patch(&reference, &[face])));
    let discovered = network.discover_selected_ss_solids(
        [&a, &b], solid::BooleanMaterialOp::Intersection, &directions, Default::default(),
        &candidates, 32, shell,
    ).unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].model().faces.len(), 7);
    assert_eq!(discovered[0].model().bodies.len(), 1);
    assert!(discovered[0].model().vertices.iter().all(|vertex| vertex.point[0] <= 1.));
    let mut mixed = network.discover_uncut_boolean_patches(
        [&a, &b], solid::BooleanMaterialOp::Intersection, &directions, Default::default(),
    ).unwrap();
    assert!(mixed.iter().any(|patch| patch.faces[0].surface == a.model().faces[0].surface));
    for axis in [0, 1] {
        for coordinate in [0., 1.] {
            let wall = |model: &brep_core::Model| {
                model.faces.iter().position(|face| {
                    face.surface.control_points.iter().flatten().all(|p| p[axis] == coordinate)
                }).unwrap()
            };
            let faces = [wall(a.model()), wall(b.model())];
            let patch = brep_core::boolean::operations::exact_coplanar_intersection_patch(
                [&a, &b], faces, 100,
            ).unwrap().unwrap();
            let source = &a.model().faces[faces[0]].surface;
            for coedge in &patch.loops[0].coedges {
                let [u, v] = [coedge.pcurve.control_points[0][0], coedge.pcurve.control_points[0][1]];
                let vertex = patch.edges[coedge.edge].vertices[0];
                assert_eq!(source.evaluate(u, v).unwrap().point.as_slice(), &patch.vertices[vertex].point);
            }
            if axis == 1 {
                assert_eq!(patch.vertices.len(), 5);
                assert!(patch.vertices.iter().any(|v| v.point == [0.5, coordinate, 0.]));
            }
            mixed.push(patch);
        }
    }
    let discovered = network.discover_selected_ss_solids(
        [&a, &b], solid::BooleanMaterialOp::Intersection, &directions, Default::default(),
        &mixed, 32, shell,
    ).unwrap();
    assert_eq!(discovered[0].model().faces.len(), 7);
    let volume = discovered[0].volume_bounds(solid::VolumeLimits {
        axis: 2, origin: 0., absolute_error: 1e-4, cells: 100_000, spans: 100_000,
        domain_cells: 100_000, algebra: Default::default(),
    }).unwrap().positive_volume.unwrap();
    assert!(volume[0] <= 0.875 && 0.875 <= volume[1]);
    let limits = trim::OpenSSIntersectionCompletionLimits {
        face_pairs: 25, polygon_tests_per_pair: 100, rim_pairs: 32,
    };
    for restricted in [
        trim::OpenSSIntersectionCompletionLimits { face_pairs: 24, ..limits },
        trim::OpenSSIntersectionCompletionLimits { polygon_tests_per_pair: 8, ..limits },
    ] {
        let error = network.complete_source_intersection(
            [&a, &b], &directions, Default::default(), restricted, shell,
        ).err().expect("Completion must reject incomplete search");
        assert_eq!(error.code, "BREP_RESOURCE_LIMIT");
    }
    let automatic = network.complete_source_intersection(
        [&a, &b], &directions, Default::default(), limits, shell,
    ).unwrap();
    assert_eq!(automatic.len(), 1);
    let automatic = &automatic[0];
    assert_eq!(automatic.model().faces.len(), 7);
    assert_eq!(automatic.model().bodies.len(), 1);
    let volume = automatic.volume_bounds(solid::VolumeLimits {
        axis: 2, origin: 0., absolute_error: 1e-4, cells: 100_000, spans: 100_000,
        domain_cells: 100_000, algebra: Default::default(),
    }).unwrap().positive_volume.unwrap();
    assert!(volume[0] <= 0.875 && 0.875 <= volume[1]);
    candidates.push(completion[0].clone());
    assert!(network.discover_selected_ss_solids(
        [&a, &b], solid::BooleanMaterialOp::Intersection, &directions, Default::default(),
        &candidates, 32, shell,
    ).is_err());
}
fn open_patch(source: &brep_core::Model, faces: &[usize]) -> brep_core::Model {
    let mut model = source.clone();
    model.faces = faces.iter().map(|&id| source.faces[id].clone()).collect();
    model.loops.clear();
    model.edges.clear();
    model.vertices.clear();
    let mut vertices = std::collections::BTreeMap::new();
    let mut edges = std::collections::BTreeMap::new();
    let mut loops = Vec::new();
    for face in &mut model.0.faces {
        let mut wire = source.loops[face.outer].clone();
        face.outer = loops.len();
        for coedge in &mut wire.coedges {
            coedge.edge = *edges.entry(coedge.edge).or_insert_with(|| {
                let mut edge = source.edges[coedge.edge].clone();
                edge.vertices = edge.vertices.map(|v| {
                    *vertices.entry(v).or_insert_with(|| {
                        let index = model.0.vertices.len();
                        model.0.vertices.push(source.vertices[v].clone());
                        index
                    })
                });
                let index = model.0.edges.len();
                model.0.edges.push(edge);
                index
            });
        }
        loops.push(wire);
    }
    model.loops = loops;
    model.shells[0].faces = faces.iter().map(|&id| &source.shells[0].faces[id]).enumerate()
        .map(|(face, use_)| brep_core::FaceUse { face, reversed: use_.reversed })
        .collect();
    model.shells[0].closed = false;
    model.bodies.clear();
    model.rebuild_topology_ids();
    model
}

#[test]
fn coplanar_intersection_preserves_split_regions_and_holes() {
    use brep_core::boolean::operations::exact_coplanar_intersection_patches;
    let make_solid = |model| {
        let mut proof = policy();
        proof.geometry.boundary.contacts.cells_per_pair = 128;
        let result = solid::qualify(model, proof).unwrap();
        assert!(result.solid().is_some(), "Boundary proof failed: equal {}, trim {}, pairs {:?}",
            result.geometry().boundary.agreement.all_equal, result.geometry().boundary.trim.all_valid,
            result.geometry().boundary.intersections.pairs.pairs.iter()
                .filter(|pair| pair.reason == "pair-unresolved").map(|pair| pair.faces).collect::<Vec<_>>());
        result.into_solid().unwrap()
    };
    let u_shape = make_solid(brep_core::extrude_polygon(&[
        [0., 0.], [4., 0.], [4., 4.], [3., 4.],
        [3., 1.], [1., 1.], [1., 4.], [0., 4.],
    ], 0., 1.).unwrap());
    let bar = make_solid(brep_core::cuboid([0., 2., 0.], [4., 4., 1.]).unwrap());
    let top = |s: &solid::QualifiedClosedSolid| {
        s.model().faces.iter().position(|f| {
            f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
        }).unwrap()
    };
    let patches = exact_coplanar_intersection_patches(
        [&u_shape, &bar], [top(&u_shape), top(&bar)], 4096,
    ).unwrap();
    let error = exact_coplanar_intersection_patches(
        [&u_shape, &bar], [top(&u_shape), top(&bar)], 64,
    ).unwrap_err();
    assert_eq!(error.code, "BREP_RESOURCE_LIMIT");
    assert_eq!(patches.len(), 2);
    let mut spans: Vec<_> = patches.iter().map(|patch| {
        assert_eq!(patch.vertices.len(), 4);
        assert!(patch.faces[0].holes.is_empty());
        patch.vertices.iter().fold([4_i32, 0], |[lo, hi], vertex| {
            let x = vertex.point[0] as i32;
            [lo.min(x), hi.max(x)]
        })
    }).collect();
    spans.sort_unstable();
    assert_eq!(spans, [[0, 1], [3, 4]]);

    let frame = make_solid(brep_core::extrude_polygon_with_holes(
        &[[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
        &[vec![[1., 1.], [1., 3.], [3., 3.], [3., 1.]]],
        0., 1.,
    ).unwrap());
    let box_solid = make_solid(brep_core::cuboid([0.; 3], [4., 4., 1.]).unwrap());
    let patches = exact_coplanar_intersection_patches(
        [&frame, &box_solid], [top(&frame), top(&box_solid)], 4096,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].faces[0].holes.len(), 1);
    assert_eq!(patches[0].vertices.len(), 8);
}

fn transpose_top_chart(mut model: brep_core::Model) -> brep_core::Model {
    let face = model.faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let f = &mut model.faces[face];
    let surface = &mut f.surface;
    assert_eq!((surface.degree_u, surface.degree_v), (1, 1));
    let old = surface.control_points.clone();
    surface.control_points = (0..2).map(|i| (0..2).map(|j| old[j][i].clone()).collect()).collect();
    let knots = surface.knots_u.clone();
    surface.knots_u = std::mem::replace(&mut surface.knots_v, knots);
    let wires = std::iter::once(f.outer).chain(f.holes.iter().copied()).collect::<Vec<_>>();
    for wire in wires {
        let coedges = &mut model.loops[wire].coedges;
        coedges.reverse();
        for coedge in coedges {
            coedge.reversed = !coedge.reversed;
            coedge.pcurve = coedge.pcurve.reverse().unwrap();
            for point in &mut coedge.pcurve.control_points {
                point.swap(0, 1);
            }
        }
    }
    let use_ = model.shells.iter_mut().flat_map(|s| &mut s.faces).find(|u| u.face == face).unwrap();
    use_.reversed = !use_.reversed;
    model.rebuild_topology_ids();
    model
}

#[test]
fn coplanar_intersection_keeps_original_rational_boundary() {
    use brep_core::boolean::operations::exact_coplanar_intersection_patches;
    let mut proof = policy();
    proof.geometry.boundary.contacts.cells_per_pair = 128;
    let round = brep_core::cylinder(1., 1.).unwrap();
    let square = brep_core::cuboid([-2., -2., 0.], [2., 2., 1.]).unwrap();
    let disjoint = brep_core::cuboid([2., 2., 0.], [4., 4., 1.]).unwrap();
    let [round, square, disjoint] = [round, square, disjoint].map(|model| {
        solid::qualify(model, proof).unwrap().into_solid().unwrap()
    });
    let top = |s: &solid::QualifiedClosedSolid| s.model().faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let patches = exact_coplanar_intersection_patches(
        [&square, &round], [top(&square), top(&round)], 100,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    let cap = &round.model().faces[top(&round)];
    assert_eq!(patch.faces[0].surface, cap.surface);
    for (actual, original) in patch.loops[0].coedges.iter()
        .zip(&round.model().loops[cap.outer].coedges)
    {
        assert_eq!(actual.pcurve, original.pcurve);
        assert_eq!(patch.edges[actual.edge].curve, round.model().edges[original.edge].curve);
    }
    assert!(exact_coplanar_intersection_patches(
        [&round, &disjoint], [top(&round), top(&disjoint)], 100,
    ).unwrap().is_empty());
    let tube = solid::qualify(transpose_top_chart(brep_core::tube(2., 0.5, 1.).unwrap()), proof)
        .unwrap().into_solid().unwrap();
    let patches = exact_coplanar_intersection_patches(
        [&round, &tube], [top(&round), top(&tube)], 100,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    assert_eq!(patch.faces[0].surface, cap.surface);
    assert_eq!(patch.faces[0].holes.len(), 1);
    let source_hole = tube.model().faces[top(&tube)].holes[0];
    for (actual, original) in patch.loops[patch.faces[0].holes[0]].coedges.iter()
        .zip(tube.model().loops[source_hole].coedges.iter().rev())
    {
        assert_eq!(patch.edges[actual.edge].curve, tube.model().edges[original.edge].curve);
    }
}

#[test]
fn coplanar_contact_splits_and_sews_exact_curve_fragments() {
    use brep_core::boolean::operations::exact_coplanar_intersection_patches;
    let mut proof = policy();
    proof.geometry.boundary.contacts.cells_per_pair = 128;
    let round = brep_core::cylinder(1., 1.).unwrap();
    let mut half_space = brep_core::cuboid([0., -2., 0.], [2., 2., 1.]).unwrap();
    let edge = half_space.edges.iter().position(|edge| edge.vertices.iter().all(|&v| {
        let p = half_space.vertices[v].point;
        p[0] == 0. && p[2] == 1.
    })).unwrap();
    half_space.edges[edge].curve = half_space.edges[edge].curve.insert(0.5, 1).unwrap();
    for coedge in half_space.loops.iter_mut().flat_map(|wire| &mut wire.coedges) {
        if coedge.edge == edge {
            coedge.pcurve = coedge.pcurve.insert(0.5, 1).unwrap();
        }
    }
    let top_face = half_space.faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let face = &mut half_space.faces[top_face];
    face.surface.knots_v = vec![-1., -1., 2., 2.];
    for row in &mut face.surface.control_points {
        row[0][1] = -6.;
        row[1][1] = 6.;
    }
    let wire = face.outer;
    let use_ = half_space.loops[wire].coedges.iter_mut().find(|c| c.edge == edge).unwrap();
    let mut longer = use_.pcurve.clone();
    for p in &mut longer.control_points {
        p[1] = p[1] * 2. - 0.5;
    }
    *use_ = brep_core::Coedge::new(edge, use_.reversed, longer).with_trim(brep_topology::CoedgeTrim {
        curve_parameter: [0., 1.],
        pcurve_parameter: [0.25, 0.75],
        periodic_lift: [[0; 2]; 2],
    }).unwrap();
    half_space.rebuild_topology_ids();
    let tangent = brep_core::cuboid([1., -2., 0.], [3., 2., 1.]).unwrap();
    let [round, half_space, tangent] = [round, half_space, tangent].map(|model| {
        solid::qualify(model, proof).unwrap().into_solid().unwrap()
    });
    let top = |s: &solid::QualifiedClosedSolid| s.model().faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let patches = exact_coplanar_intersection_patches(
        [&round, &half_space], [top(&round), top(&half_space)], 100,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    assert_eq!(patch.edges.len(), 4);
    assert_eq!(patch.edges.iter().filter(|e| e.curve.degree == 2).count(), 2);
    let mut spans: Vec<_> = patch.edges.iter().filter(|e| e.curve.degree == 1).map(|edge| {
        let mut ends = edge.vertices.map(|i| patch.vertices[i].point[1]);
        ends.sort_by(f64::total_cmp);
        ends
    }).collect();
    spans.sort_by(|a, b| a[0].total_cmp(&b[0]));
    assert_eq!(spans, [[-1., 0.], [0., 1.]]);
    // The vertical clipping edge reaches y=-1 at a third of its span.
    let rational_clock = solid::qualify(
        brep_core::cuboid([0., -2., 0.], [2., 1., 1.]).unwrap(), proof,
    ).unwrap().into_solid().unwrap();
    let patches = exact_coplanar_intersection_patches(
        [&round, &rational_clock], [top(&round), top(&rational_clock)], 100,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].edges.len(), 3);
    let closing = patches[0].edges.iter().find(|edge| edge.curve.degree == 1).unwrap();
    assert_eq!(closing.curve.domain(), [0., 1.]);
    let mut y = closing.vertices.map(|v| patches[0].vertices[v].point[1]);
    y.sort_by(f64::total_cmp);
    assert_eq!(y, [-1., 1.]);
    assert!(exact_coplanar_intersection_patches(
        [&round, &tangent], [top(&round), top(&tangent)], 100,
    ).unwrap().is_empty());
}

#[test]
fn coplanar_contact_opens_one_rational_hole_and_keeps_another() {
    use brep_core::boolean::operations::exact_coplanar_intersection_patches;
    use nurbs_core::curve::Curve;
    let outer = vec![Curve::from_polyline(vec![
        vec![-4., -4.], vec![4., -4.], vec![4., 4.], vec![-4., 4.], vec![-4., -4.],
    ]).unwrap()];
    let hole = |x: f64| {
        let corners = [[0.5, 0.], [0., -0.5], [-0.5, 0.], [0., 0.5]];
        (0..4).map(|i| {
            let [ax, ay] = corners[i];
            let [bx, by] = corners[(i + 1) % 4];
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![vec![x + ax, ay], vec![x + ax + bx, ay + by], vec![x + bx, by]],
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            }
        }).collect::<Vec<_>>()
    };
    let frame = brep_core::prism::extrude(&[outer, hole(0.), hole(2.5)], 0., 1.).unwrap();
    let mut clip = transpose_top_chart(brep_core::cuboid([0., -4., 0.], [8., 4., 1.]).unwrap());
    let top_edge = clip.edges.iter().position(|edge| edge.vertices.iter().all(|&v| {
        let p = clip.vertices[v].point;
        p[1] == 4. && p[2] == 1.
    })).unwrap();
    clip.edges[top_edge].curve = clip.edges[top_edge].curve.insert(0.5, 1).unwrap();
    for coedge in clip.loops.iter_mut().flat_map(|wire| &mut wire.coedges) {
        if coedge.edge == top_edge {
            coedge.pcurve = coedge.pcurve.insert(0.5, 1).unwrap();
        }
    }
    clip.rebuild_topology_ids();
    let mut proof = policy();
    proof.geometry.boundary.contacts.cells_per_pair = 256;
    let [frame, clip] = [frame, clip].map(|model| {
        solid::qualify(model, proof).unwrap().into_solid().unwrap()
    });
    let top = |s: &solid::QualifiedClosedSolid| s.model().faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let patches = exact_coplanar_intersection_patches(
        [&frame, &clip], [top(&frame), top(&clip)], 4096,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    let face = &patch.faces[0];
    assert_eq!(face.holes.len(), 1);
    let curved = |wire: usize| patch.loops[wire].coedges.iter()
        .filter(|c| patch.edges[c.edge].curve.degree == 2).count();
    assert_eq!(curved(face.outer), 2);
    assert_eq!(curved(face.holes[0]), 4);
    for y in [-4., 4.] {
        let edges: Vec<_> = patch.edges.iter().filter(|edge| {
            edge.vertices.iter().all(|&v| patch.vertices[v].point[1] == y)
        }).collect();
        assert_eq!(edges.len(), 1);
        let mut x = edges[0].vertices.map(|v| patch.vertices[v].point[0]);
        x.sort_by(f64::total_cmp);
        assert_eq!(x, [0., 4.]);
    }
}

#[test]
fn coplanar_intersection_resolves_whole_shared_rational_spans() {
    use brep_core::boolean::operations::exact_coplanar_intersection_patches;
    let cylinder = brep_core::cylinder(1., 1.).unwrap();
    let mut equivalent = transpose_top_chart(cylinder.clone());
    let top = |m: &brep_core::Model| m.faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let wire = equivalent.faces[top(&equivalent)].outer;
    for coedge in &mut equivalent.loops[wire].coedges {
        for w in &mut coedge.pcurve.weights {
            *w *= 2.;
        }
        for knot in &mut coedge.pcurve.knots {
            *knot = 2. + *knot * 4.;
        }
    }
    equivalent.rebuild_topology_ids();
    let annulus = brep_core::tube(2., 1., 1.).unwrap();
    let mut proof = policy();
    proof.geometry.boundary.contacts.cells_per_pair = 128;
    let [cylinder, equivalent, annulus] = [cylinder, equivalent, annulus].map(|model| {
        solid::qualify(model, proof).unwrap().into_solid().unwrap()
    });
    let patch = exact_coplanar_intersection_patches(
        [&cylinder, &equivalent], [top(cylinder.model()), top(equivalent.model())], 100,
    ).unwrap();
    assert_eq!(patch.len(), 1);
    assert_eq!(patch[0].edges.len(), 4);
    assert!(patch[0].edges.iter().all(|edge| edge.curve.degree == 2));
    assert!(exact_coplanar_intersection_patches(
        [&cylinder, &annulus], [top(cylinder.model()), top(annulus.model())], 100,
    ).unwrap().is_empty());
}

#[test]
fn closed_solid_coplanar_intersection_compares_shared_arcs_across_source_knots() {
    use brep_core::boolean::operations::exact_coplanar_intersection_patches;
    use nurbs_core::curve::Curve;
    let arc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![-1., 0.], vec![0., -2.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let remainder = Curve::from_polyline(vec![
        vec![1., 0.], vec![2., 0.], vec![2., -2.], vec![3., -2.],
        vec![3., 2.], vec![-1., 2.], vec![-1., 0.],
    ]).unwrap();
    let with_knot = arc.insert(0.5, 2).unwrap();
    let split = brep_core::prism::extrude(&[vec![with_knot, remainder.clone()]], 0., 1.).unwrap();
    let mut joined = brep_core::prism::extrude(&[vec![arc, remainder]], 0., 1.).unwrap();
    for edge in &mut joined.edges {
        if edge.curve.degree == 2 {
            edge.curve = edge.curve.insert(0.5, 2).unwrap();
        }
    }
    for coedge in joined.loops.iter_mut().flat_map(|wire| &mut wire.coedges) {
        if coedge.pcurve.degree == 2 {
            coedge.pcurve = coedge.pcurve.insert(0.5, 2).unwrap();
        }
    }
    joined.rebuild_topology_ids();
    let mut proof = policy();
    proof.geometry.boundary.contacts.cells_per_pair = 256;
    let qualify = |model| {
        let admission = solid::qualify(model, proof).unwrap();
        assert!(admission.geometry().boundary.agreement.all_equal);
        admission.into_solid().expect("Source prism needs continuous material proof")
    };
    let [joined, split] = [qualify(joined), qualify(split)];
    let top = |m: &brep_core::Model| m.faces.iter().position(|f| {
        f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)
    }).unwrap();
    let patches = exact_coplanar_intersection_patches(
        [&joined, &split], [top(joined.model()), top(split.model())], 256,
    ).unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].edges.len(), 8);
    assert_eq!(patches[0].edges.iter().filter(|edge| edge.curve.degree == 2).count(), 2);
    assert!(patches[0].vertices.iter().any(|v| v.point == [0., -1., 1.]));
}

#[test]
fn exact_patch_sewing_reverses_a_periodic_rim_clock() {
    let source = full_periodic_cylinder::source();
    let wall = open_patch(&source, &[0]);
    let mut cap = open_patch(&source, &[1]);
    assert!(cap.edges[0].curve.periodic);
    cap.edges[0].curve = cap.edges[0].curve.reverse().unwrap();
    cap.loops[0].coedges[0].reversed ^= true;
    cap.rebuild_topology_ids();
    let top = open_patch(&source, &[2]);

    let shell = solid::ShellRepairPolicy {
        establish_forest: policy(),
        establish_orientation: policy(),
        readmit: policy(),
    };
    let result = brep_core::ss_face_membership::assemble_exact_patch_solid(
        &[wall, cap, top], 8, shell,
    ).unwrap();
    assert_eq!(result.model().bodies.len(), 1);
    assert_eq!(result.model().faces.len(), 3);
    assert_eq!(result.model().edges.len(), 3);
    result.reverify().unwrap();
}

#[test]
fn exact_patch_rims_rebuild_and_readmit_a_closed_body() {
    let source = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let mut patches = [open_patch(&source, &[0]), open_patch(&source, &[1, 2, 3, 4, 5])];
    let reversed = brep_topology::explorer::boundary_edges(&patches[1].0)[0];
    let edge = &mut patches[1].edges[reversed];
    edge.vertices.reverse();
    edge.curve = edge.curve.reverse().unwrap();
    edge.curve.knots.iter_mut().for_each(|knot| *knot = 2. + 4. * *knot);
    edge.curve.weights.iter_mut().for_each(|weight| *weight *= 3.);
    edge.curve = edge.curve.elevate(2).unwrap();
    edge.curve = edge.curve.insert(4., 1).unwrap();
    for coedge in patches[1].loops.iter_mut().flat_map(|wire| &mut wire.coedges) {
        if coedge.edge == reversed {
            coedge.reversed = !coedge.reversed;
        }
    }
    patches[1].rebuild_topology_ids();
    let shell = solid::ShellRepairPolicy {
        establish_forest: policy(), establish_orientation: policy(), readmit: policy(),
    };
    let result = brep_core::ss_face_membership::assemble_exact_patch_solid(&patches, 4, shell)
        .unwrap();
    assert_eq!(result.model().bodies.len(), 1);
    assert_eq!(result.model().faces.len(), 6);
    assert_eq!(result.model().edges.len(), 12);
    assert_eq!(result.model().vertices.len(), 8);
    result.reverify().unwrap();
    assert!(brep_core::ss_face_membership::assemble_exact_patch_solid(&patches[..1], 1024, shell).is_err());
    assert!(brep_core::ss_face_membership::assemble_exact_patch_solid(&patches, 3, shell).is_err());
}

#[test]
fn continuous_geometry_admission_owns_point_material_classification() {
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let body = solid::qualify(model.clone(), policy())
        .unwrap()
        .into_solid()
        .unwrap();
    body.reverify().unwrap();
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419]];
    assert_eq!(
        body.classify_point([1.; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(true)
    );
    assert_eq!(
        body.classify_point([3.; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(false)
    );
    let mut inward = model.clone();
    for use_ in &mut inward.shells[0].faces {
        use_.reversed = !use_.reversed;
    }
    assert!(solid::qualify(inward, policy()).unwrap().solid().is_none());
    let mut open = model;
    open.shells[0].closed = false;
    assert!(solid::qualify(open, policy()).is_err());
}
#[test]
fn a_successful_tolerance_repair_cannot_publish_geometrically_unproved_body() {
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let before = model.clone();
    let cap = model
        .tolerance_context()
        .unwrap()
        .spatial_bounds()
        .absolute_mm;
    let mut curve = model.edges[0].curve.clone();
    curve.control_points[1][2] += cap * 0.125;
    let outcome = solid::heal_and_qualify(
        &model,
        &[AutoRepairOperation::RefitEdge {
            edge: 0,
            replacement: curve,
        }],
        policy(),
    );
    assert_eq!(outcome.repair_attempt().status, HealingStatus::Complete);
    assert_eq!(
        outcome.status(),
        solid::GeometricHealingStatus::GeometryUnproved
    );
    assert!(outcome.solid().is_none());
    assert_eq!(outcome.model(), &before);
    assert_eq!(model, before);
}
#[test]
fn restoring_source_edge_geometry_is_readmitted_after_real_healing() {
    let source = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let mut damaged = source.clone();
    let cap = source
        .tolerance_context()
        .unwrap()
        .spatial_bounds()
        .absolute_mm;
    damaged.edges[0].curve.control_points[1][2] += cap * 0.125;
    let before = damaged.clone();
    let outcome = solid::heal_and_qualify(
        &damaged,
        &[AutoRepairOperation::RefitEdge {
            edge: 0,
            replacement: source.edges[0].curve.clone(),
        }],
        policy(),
    );
    assert_eq!(outcome.status(), solid::GeometricHealingStatus::Accepted);
    assert!(outcome.solid().is_some());
    assert!(outcome.geometry().unwrap().proven);
    assert_eq!(damaged, before);
}
#[test]
fn discovered_endpoint_gap_is_sewn_and_readmitted_as_closed_material() {
    let source = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let mut damaged = source.clone();
    let vertex = damaged.edges[0].vertices[0];
    damaged.vertices[vertex].point[0] += damaged.tolerance_mm * 2.;
    let sewing = brep_core::healing::SewingBudget {
        defects: Default::default(),
        user_cap_mm: damaged.tolerance_mm * 3.,
        cumulative_cap_mm: damaged.tolerance_mm * 100.,
        max_candidates: 64,
    };
    let shell = solid::ShellRepairPolicy {
        establish_forest: policy(),
        establish_orientation: policy(),
        readmit: policy(),
    };
    let result = solid::sew_gaps_and_rebuild_shells(&damaged, sewing, shell).unwrap();
    assert_eq!(result.status(), solid::GeometricHealingStatus::Accepted);
    assert!(result.geometry().unwrap().proven);
    assert_eq!(result.model().vertices, source.vertices);
    assert!(!result.repair_attempt().operations.is_empty());
    let body = result.into_solid().unwrap();
    let directions = [[1., 0.317, 0.173]];
    for (point, inside) in [([1.; 3], true), ([3.; 3], false)] {
        assert_eq!(
            body.classify_point(point, &directions, 10000, 100000)
                .unwrap()
                .parity,
            Some(inside),
        );
    }

    let mut exhausted = shell;
    exhausted.readmit.geometry.boundary.exact_work = 1;
    let rejected = solid::sew_gaps_and_rebuild_shells(&damaged, sewing, exhausted).unwrap();
    assert_eq!(rejected.status(), solid::GeometricHealingStatus::GeometryUnproved);
    assert!(rejected.solid().is_none());
    assert_eq!(rejected.model(), &damaged);

    let mut limited = sewing;
    limited.user_cap_mm = damaged.tolerance_mm;
    let refused = solid::sew_gaps_and_rebuild_shells(&damaged, limited, shell).unwrap();
    assert_eq!(refused.status(), solid::GeometricHealingStatus::RepairRefused);
    assert!(refused.solid().is_none());
    assert_eq!(refused.model(), &damaged);
}

#[test]
fn nested_shell_roles_and_material_orientation_are_rebuilt_and_reaudited() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let good = brep_core::minkowski3d::curved::certify_enclosed_difference(&outer, &inner, 0)
        .unwrap()
        .into_model();
    let mut wrong = good.clone();
    wrong.bodies[0].inner_shells.clear();
    wrong.bodies.push(brep_core::Body {
        outer_shell: 1,
        inner_shells: vec![],
    });
    for face in &mut wrong.shells[0].faces {
        face.reversed = !face.reversed;
    }
    wrong.rebuild_topology_ids();
    let before = wrong.clone();
    let p = solid::ShellRepairPolicy {
        establish_forest: policy(),
        establish_orientation: policy(),
        readmit: policy(),
    };
    let result = solid::rebuild_even_odd_shells(&wrong, p);
    assert!(result.solid().is_some(), "{:?}", result.refusal());
    assert_eq!(result.parents().unwrap(), [None, Some(0)]);
    assert_eq!(result.flipped_shells(), [0]);
    assert_eq!(result.model().bodies[0].inner_shells, [1]);
    assert_eq!(result.model().faces, good.faces);
    assert_eq!(result.model().edges, good.edges);
    assert_eq!(result.model().vertices, good.vertices);
    let directions = [[1., 0.317, 0.173]];
    let body = result.solid().unwrap();
    assert_eq!(
        body.classify_point([0.5; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(true)
    );
    assert_eq!(
        body.classify_point([2.; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(false)
    );
    assert_eq!(wrong, before);
    let mut unassigned = good.clone();
    unassigned.bodies.clear();
    unassigned.rebuild_topology_ids();
    let recovered = solid::rebuild_even_odd_shells(&unassigned, p);
    assert!(recovered.solid().is_some(), "{:?}", recovered.refusal());
    assert_eq!(recovered.parents().unwrap(), [None, Some(0)]);
    assert_eq!(recovered.model().bodies[0].inner_shells, [1]);
    let mut partial = good.clone();
    partial.bodies[0].inner_shells.clear();
    partial.rebuild_topology_ids();
    let recovered = solid::rebuild_even_odd_shells(&partial, p);
    assert!(recovered.solid().is_some(), "{:?}", recovered.refusal());
    assert_eq!(recovered.model().bodies[0].inner_shells, [1]);
    let mut exhausted = p;
    exhausted.readmit.geometry.boundary.exact_work = 1;
    let rejected = solid::rebuild_even_odd_shells(&wrong, exhausted);
    assert!(rejected.solid().is_none());
    assert_eq!(rejected.model(), &before);

    let mut damaged = wrong.clone();
    let cap = damaged.tolerance_context().unwrap().spatial_bounds().absolute_mm;
    damaged.edges[0].curve.control_points[1][2] += cap * 0.125;
    let repairs = [AutoRepairOperation::RefitEdge {
        edge: 0,
        replacement: wrong.edges[0].curve.clone(),
    }];
    let healed = solid::heal_and_rebuild_shells(&damaged, &repairs, p);
    assert_eq!(
        healed.status(),
        solid::GeometricHealingStatus::Accepted,
        "repair refusals: {:?}, geometry refusal: {:?}",
        healed.repair_attempt().refusal_reasons,
        healed.refusal(),
    );
    assert!(healed.geometry().unwrap().proven);
    assert_eq!(healed.repair_attempt().certificates.len(), 1);
    assert!(healed.repair_attempt().certificates.iter().all(|receipt| {
        receipt.sew_complete && !receipt.audit_ok
    }));
    assert_eq!(healed.flipped_shells(), [0]);
    assert_eq!(healed.model().bodies[0].inner_shells, [1]);
    assert_eq!(healed.model().edges, good.edges);
    let rejected = solid::heal_and_rebuild_shells(&damaged, &repairs, exhausted);
    assert_eq!(rejected.status(), solid::GeometricHealingStatus::GeometryUnproved);
    assert_eq!(rejected.model(), &damaged);
}
#[test]
fn rational_surface_material_seed_is_tied_to_the_original_point_not_its_rounding() {
    use nurbs_core::surface::Surface;
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let body = solid::qualify(model, policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let mut source = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.25, 0.25, 1.], vec![0.25, 1.75, 1.]],
            vec![vec![1.75, 0.25, 1.], vec![1.75, 1.75, 1.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 5.]],
        periodic_u: false,
        periodic_v: false,
    };
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    let seed = solid::qualify_surface_material_seed(
        &body,
        &source,
        [0.5; 2],
        &directions,
        Default::default(),
    )
    .unwrap();
    assert!(seed.inside());
    seed.reverify().unwrap();
    for row in &mut source.control_points {
        for p in row {
            p[2] = 3.;
        }
    }
    assert!(
        !solid::qualify_surface_material_seed(
            &body,
            &source,
            [0.5; 2],
            &directions,
            Default::default()
        )
        .unwrap()
        .inside()
    );
    for row in &mut source.control_points {
        for p in row {
            p[2] = 2.;
        }
    }
    assert!(
        solid::qualify_surface_material_seed(
            &body,
            &source,
            [0.5; 2],
            &directions,
            Default::default()
        )
        .is_err()
    );
    assert!(
        solid::qualify_surface_material_seed(
            &body,
            &source,
            [1.5; 2],
            &directions,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn all_boolean_operations_select_original_face_material_and_subtraction_orientation() {
    use solid::{BooleanMaterialOp as Op, BooleanSourceSide as Side};
    let a = solid::qualify(brep_core::cuboid([0.; 3], [2.; 3]).unwrap(), policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let b = solid::qualify(
        brep_core::cuboid([1., -1., -1.], [3.; 3]).unwrap(),
        policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let face = |x: f64| {
        a.model()
            .faces
            .iter()
            .position(|f| f.surface.control_points.iter().flatten().all(|p| p[0] == x))
            .unwrap()
    };
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    for (op, inner_keep, inner_reverse, outer_keep) in [
        (Op::Union, false, false, true),
        (Op::Intersection, true, false, false),
        (Op::Difference, false, false, true),
        (Op::Xor, true, true, true),
    ] {
        let inside = solid::qualify_boolean_face_seed(
            solid::BooleanFace {
                solids: [&a, &b],
                operation: op,
                side: Side::First,
                face: face(2.),
            },
            [0.5; 2],
            &directions,
            Default::default(),
        )
        .unwrap();
        assert_eq!(inside.retain(), inner_keep);
        assert_eq!(inside.reverse_material_normal(), inner_reverse);
        let outside = solid::qualify_boolean_face_seed(
            solid::BooleanFace {
                solids: [&a, &b],
                operation: op,
                side: Side::First,
                face: face(0.),
            },
            [0.5; 2],
            &directions,
            Default::default(),
        )
        .unwrap();
        assert_eq!(outside.retain(), outer_keep);
        assert!(!outside.reverse_material_normal());
    }
    let bf = b
        .model()
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[0] == 1.)
        })
        .unwrap();
    let subtract = solid::qualify_boolean_face_seed(
        solid::BooleanFace {
            solids: [&a, &b],
            operation: Op::Difference,
            side: Side::Second,
            face: bf,
        },
        [0.5; 2],
        &directions,
        Default::default(),
    )
    .unwrap();
    assert!(subtract.retain() && subtract.reverse_material_normal());
    subtract.reverify().unwrap();
    assert!(
        solid::qualify_boolean_face_seed(
            solid::BooleanFace {
                solids: [&a, &b],
                operation: Op::Union,
                side: Side::First,
                face: face(2.),
            },
            [0.; 2],
            &directions,
            Default::default(),
        )
        .is_err()
    );
}
#[test]
fn independent_retained_flux_includes_cavities_and_curved_face_volume() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let cavity = brep_core::minkowski3d::curved::certify_enclosed_difference(&outer, &inner, 0)
        .unwrap()
        .into_model();
    let limits = solid::VolumeLimits {
        axis: 2,
        origin: 0.,
        absolute_error: 1e-6,
        cells: 100000,
        spans: 100000,
        domain_cells: 100000,algebra:Default::default(),
    };
    let body = solid::qualify(cavity, policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let bounds = body.volume_bounds(limits).unwrap().positive_volume.unwrap();
    assert!(bounds[0] <= 56. && 56. <= bounds[1]);
    let mut curved = outer;
    let f = curved
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 4.)
        })
        .unwrap();
    let original = curved.faces[f].surface.clone();
    let mut points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    original
                        .evaluate(i as f64 / 2., j as f64 / 2.)
                        .unwrap()
                        .point.to_vec()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    points[1][1][2] += 0.125;
    curved.faces[f].surface = nurbs_core::surface::Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: points,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let body = solid::qualify(curved, policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let report = body.volume_bounds(limits).unwrap();
    let bounds = report
        .positive_volume
        .unwrap_or_else(|| panic!("{} {:?}", report.reason, report.signed_bounds));
    let expected = 64. + 2. / 9.;
    assert!(bounds[0] <= expected && expected <= bounds[1], "{bounds:?}");
    let exhausted = body
        .volume_bounds(solid::VolumeLimits { cells: 1, ..limits })
        .unwrap();
    assert!(exhausted.positive_volume.is_none());
    assert!(exhausted.signed_bounds.is_none());
}
