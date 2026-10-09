use super::*;
#[test]
fn push_boolean_partitioned_cap_moves_all_supports() {
    let model = boolean(
        &cuboid([0.; 3], [12., 10., 10.]).unwrap(),
        &cuboid([8., 0., 0.], [20., 10., 10.]).unwrap(),
        "union",
    ).unwrap();
    let caps: Vec<_> = model.faces.iter().enumerate().filter(|(_, face)|
        face.surface.control_points.iter().flatten().all(|p| (p[2] - 10.).abs() < 1e-8)
    ).map(|(id, _)| id).collect();
    assert!(caps.len() > 1, "fixture must contain partitioned cap faces");
    for cap in caps {
        for distance in [2., -2.] {
            let edited = push_planar_face(&model, cap, distance).unwrap();
            assert_eq!(edited.validate().unwrap().boundary_edge_count, 0);
            let volume = crate::analysis::mass_properties(&edited, 1e-7, 200_000).unwrap().signed_volume_mm3;
            assert!((volume - 200. * (10. + distance)).abs() < 1e-5, "{volume}");
            assert_eq!(bounds(&edited), ([0.; 3], [20., 10., 10. + distance]));
            let longitudinal: Vec<_> = edited.edges.iter().enumerate().filter(|(_, edge)| {
                let a = edited.vertices[edge.vertices[0]].point;
                let b = edited.vertices[edge.vertices[1]].point;
                a[0] == b[0] && a[1] == b[1]
            }).map(|(id, _)| id).collect();
            assert_eq!(longitudinal.len(), 4);
            let rounded = crate::analytic_features::exact_convex_prism_fillet(&edited, &longitudinal, 1.).unwrap();
            let volume = crate::analysis::mass_properties(&rounded.model, 1e-7, 200_000).unwrap().signed_volume_mm3;
            let expected = (200. - 4. + std::f64::consts::PI) * (10. + distance);
            assert!((volume - expected).abs() < 1e-5, "rounded {volume}, expected {expected}");
        }
        assert!(push_planar_face(&model, cap, -10.).is_err());
    }
    assert_eq!(bounds(&model), ([0.; 3], [20., 10., 10.]));
}

#[test]
fn push_nonconvex_cap_and_enclosure_preserves_floor() {
    let bracket=extrude_polygon(&[[0.,0.],[40.,0.],[40.,5.],[5.,5.],[5.,30.],[0.,30.]],0.,20.).unwrap();
    let enclosure=boolean(&cuboid([0.;3],[40.,30.,20.]).unwrap(),&cuboid([2.,2.,2.],[38.,28.,22.]).unwrap(),"difference").unwrap();
    for (model,volume,area) in [(bracket,6500.,325.),(enclosure,7152.,264.)] {
        let top=model.faces.iter().enumerate().find(|(_,f)|f.surface.control_points.iter().flatten().all(|p|(p[2]-20.).abs()<1e-8)).unwrap().0;
        let before=model.clone();
        for distance in [1.,-1.] {
            let edited=push_planar_face(&model,top,distance).unwrap();
            let mass=crate::analysis::mass_properties(&edited,1e-7,200_000).unwrap().signed_volume_mm3;
            assert!((mass-(volume+area*distance)).abs()<1e-5,"{mass}");
            assert_eq!(edited.validate().unwrap().boundary_edge_count,0);
            for (before,after) in [(&model.1.vertices,&edited.1.vertices),(&model.1.edges,&edited.1.edges),(&model.1.loops,&edited.1.loops),(&model.1.faces,&edited.1.faces),(&model.1.shells,&edited.1.shells),(&model.1.bodies,&edited.1.bodies)] {
                assert_eq!(before.iter().collect::<BTreeSet<_>>(),after.iter().collect::<BTreeSet<_>>());
            }
            for (i,v) in model.vertices.iter().enumerate() {
                let mut expected=v.point;
                if (expected[2]-20.).abs()<1e-8 {expected[2]+=distance;}
                let target=edited.vertices.iter().position(|v|norm(sub(v.point,expected))<1e-8).unwrap();
                assert_eq!(model.1.vertices[i],edited.1.vertices[target]);
            }
            // Every pre-existing level below the selected cap stays fixed.
            for v in model.vertices.iter().filter(|v|v.point[2]<20.-1e-8) {
                assert!(edited.vertices.iter().any(|p|p.point==v.point));
            }
        }
        assert!(push_planar_face(&model,top,-20.).is_err());
        assert_eq!(model.vertices.iter().map(|v|v.point).collect::<Vec<_>>(),before.vertices.iter().map(|v|v.point).collect::<Vec<_>>());
    }
}


fn bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    (
        std::array::from_fn(|axis| {
            model
                .vertices
                .iter()
                .map(|v| v.point[axis])
                .fold(f64::INFINITY, f64::min)
        }),
        std::array::from_fn(|axis| {
            model
                .vertices
                .iter()
                .map(|v| v.point[axis])
                .fold(f64::NEG_INFINITY, f64::max)
        }),
    )
}

fn rotate_z(model: &mut Model, angle: f64) {
    let transform = |point: &mut Vec<f64>| {
        let (sin, cos) = angle.sin_cos();
        let [x, y] = [point[0], point[1]];
        point[0] = x * cos - y * sin;
        point[1] = x * sin + y * cos;
    };
    for vertex in &mut model.vertices {
        let mut point = vertex.point.to_vec();
        transform(&mut point);
        vertex.point = point.try_into().unwrap();
    }
    for edge in &mut model.edges {
        edge.curve.control_points.iter_mut().for_each(transform);
    }
    for face in &mut model.faces {
        face.surface
            .control_points
            .iter_mut()
            .flatten()
            .for_each(transform);
    }
}

#[test]
fn rotated_nonconvex_cap_moves_along_normal_and_refuses_oblique_sides() {
    let model=extrude_polygon(&[[0.,0.],[4.,0.],[4.,1.],[1.,1.],[1.,3.],[0.,3.]],0.,2.).unwrap();
    let top=model.faces.iter().position(|f|f.surface.control_points.iter().flatten().all(|p|p[2]==2.)).unwrap();
    let rotated=crate::transform::affine(&model,[[1.,0.,0.,7.],[0.,0.6,-0.8,-3.],[0.,0.8,0.6,2.],[0.,0.,0.,1.]]).unwrap();
    let pushed=push_planar_face(&rotated,top,0.5).unwrap();
    let volume=crate::analysis::mass_properties(&pushed,1e-7,200_000).unwrap().signed_volume_mm3;
    assert!((volume-15.).abs()<1e-6);
    let sheared=crate::transform::affine(&model,[[1.,0.,0.25,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]]).unwrap();
    assert!(push_planar_face(&sheared,top,0.5).is_err());
    assert!(push_planar_face(&model,top,-2.).is_err());
}

#[test]
fn prism_draft_zero_preserves_authored_topology() {
    let model = cuboid([0.; 3], [10.; 3]).unwrap();
    let before = format!("{model:?}");
    let out = draft_planar_prism(&model, [0., 0., 1.], [0.; 3], 0.).unwrap();
    assert_eq!(format!("{out:?}"), before);
    assert_eq!(format!("{model:?}"), before);
}

#[test]
fn prism_draft_caps_survive_and_collapse_refuses() {
    let model = cuboid([0.; 3], [10.; 3]).unwrap();
    let before = format!("{model:?}");
    for angle in [-10_f64, 10.] {
        let out = draft_planar_prism(&model, [0., 0., 1e300], [0.; 3], angle).unwrap();
        out.validate().unwrap();
        assert_eq!(out.faces.len(), 6);
        assert_eq!(out.vertices.len(), 8);
        let growth = 10. * angle.to_radians().tan();
        for v in &out.vertices {
            let z = v.point[2];
            assert!(z.abs() < 1e-8 || (z - 10.).abs() < 1e-8);
            let delta = if z.abs() < 1e-8 { 0. } else { growth };
            for point in v.point.iter().take(2) {
                assert!((*point + delta).abs() < 1e-8 || (*point - 10. - delta).abs() < 1e-8);
            }
        }
    }
    assert!(draft_planar_prism(&model, [0., 0., 1.], [0.; 3], -60.).is_err());
    assert!(draft_planar_prism(&model, [1., 0., 1.], [0.; 3], 10.).is_err());
    assert_eq!(format!("{model:?}"), before);
}

#[test]
fn planar_direct_edits_preserve_native_boundaries() {
    let stock = cuboid([0.; 3], [10.; 3]).unwrap();
    let top = stock
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 10.)
        })
        .unwrap();
    let volume = |m: &Model| {
        crate::analysis::mass_properties(m, 1e-7, 200_000)
            .unwrap()
            .signed_volume_mm3
    };
    assert!((volume(&push_planar_face(&stock, top, 2.).unwrap()) - 1200.).abs() < 1e-6);
    assert!(
        (volume(&shell_planar(&stock, &[top], 1.).unwrap()) - (1000. - 8. * 8. * 9.)).abs()
            < 1e-6
    );
    assert!((volume(&shell_planar(&stock, &[], 1.).unwrap()) - 488.).abs() < 1e-6);
    let pair = split_planar(&stock, [1., 0., 0.], 4.).unwrap();
    assert!((volume(&pair[0]) - 400.).abs() < 1e-6);
    assert!((volume(&pair[1]) - 600.).abs() < 1e-6);
    for factor in [1e-300, 1e300] {
        let scaled = split_planar(&stock, [factor, 0., 0.], 4. * factor).unwrap();
        assert!((volume(&scaled[0]) - 400.).abs() < 1e-6);
    }
    assert!(shell_planar(&stock, &[top], 6.).is_err());
    assert!(split_planar(&stock, [1., 0., 0.], 11.).is_err());
    assert!(push_planar_face(&stock, usize::MAX, 2.).is_err());
    assert!(shell_planar(&crate::cylinder(3., 5.).unwrap(), &[], 1.).is_err());
}

#[test]
fn trimmed_concave_boolean_classification_and_rotated_reuse() {
    let outline = [[3., 2.], [3., 5.], [0., 5.], [0., 0.], [5., 0.], [5., 2.]];
    let hole = vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]];
    let mut stock = extrude_polygon_with_holes(&outline, &[hole], 0., 2.).unwrap();
    assert!(contains(&stock, [0.5, 1.5, 1.]).unwrap());
    assert!(!contains(&stock, [1.5, 1.5, 1.]).unwrap());
    assert!(!contains(&stock, [4., 3., 1.]).unwrap());
    let mut cutter = cuboid([0., 0., -1.], [2.5, 6., 3.]).unwrap();
    rotate_z(&mut stock, 0.37);
    rotate_z(&mut cutter, 0.37);
    for op in ["intersection", "difference", "union"] {
        let result = boolean(&stock, &cutter, op).unwrap_or_else(|e| panic!("{op}: {e:?}"));
        assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
        // Consuming the result again exercises newly split boundary faces.
        let again =
            boolean(&result, &cutter, "union").unwrap_or_else(|e| panic!("reuse {op}: {e:?}"));
        assert_eq!(again.validate().unwrap().boundary_edge_count, 0);
    }
}

#[test]
fn overlapping_box_booleans_are_closed_breps() {
    let a = cuboid([0.; 3], [2., 2., 2.]).unwrap();
    let b = cuboid([1., 0., 0.], [3., 2., 2.]).unwrap();
    for operation in ["union", "difference", "intersection"] {
        let result = boolean(&a, &b, operation).unwrap();
        assert!(result.validate().unwrap().boundary_edge_count == 0);
        assert_eq!(result.bodies.len(), 1, "{operation}");
    }
    assert_eq!(
        bounds(&boolean(&a, &b, "union").unwrap()),
        ([0.; 3], [3., 2., 2.])
    );
    assert_eq!(
        bounds(&boolean(&a, &b, "difference").unwrap()),
        ([0.; 3], [1., 2., 2.])
    );
    assert_eq!(
        bounds(&boolean(&a, &b, "intersection").unwrap()),
        ([1., 0., 0.], [2., 2., 2.])
    );
}

#[test]
fn concave_orthogonal_difference_is_supported() {
    let a = cuboid([0.; 3], [3., 3., 3.]).unwrap();
    let b = cuboid([1., 1., 2.], [4., 4., 4.]).unwrap();
    let result = boolean(&a, &b, "difference").unwrap();
    assert!(result.faces.len() > 6);
    result.validate().unwrap();
}

#[test]
fn rotated_convex_planar_booleans_use_clipped_boundaries() {
    let a = cuboid([-2., -2., -1.], [2., 2., 1.]).unwrap();
    let mut b = cuboid([-2., -1., -1.], [2., 1., 1.]).unwrap();
    rotate_z(&mut b, std::f64::consts::FRAC_PI_4);
    b.validate().unwrap();
    for operation in ["union", "difference", "intersection"] {
        let result = boolean(&a, &b, operation).unwrap();
        assert_eq!(
            result.bodies.len(),
            if operation == "difference" { 4 } else { 1 },
            "{operation}"
        );
        assert!(result.faces.len() >= 8);
        assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
    }
}

#[test]
#[cfg(feature = "codec")]
fn topology_ids_survive_preserved_boolean_entities_and_round_trip() {
    let stock = cuboid([0., 0., 0.], [3., 2., 2.]).unwrap();
    let cutter = cuboid([2., 0., 0.], [4., 2., 2.]).unwrap();
    let result = boolean(&stock, &cutter, "difference").unwrap();
    let shared_vertices = result
        .1
        .vertices
        .iter()
        .filter(|id| stock.1.vertices.contains(id))
        .count();
    let shared_edges = result
        .1
        .edges
        .iter()
        .filter(|id| stock.1.edges.contains(id))
        .count();
    let shared_faces = result
        .1
        .faces
        .iter()
        .filter(|id| stock.1.faces.contains(id))
        .count();
    assert!(shared_vertices >= 4);
    assert!(shared_edges >= 4);
    assert!(shared_faces >= 1);
    let restored: Model =
        value_codec::from_str(&value_codec::to_string(&result).unwrap()).unwrap();
    assert_eq!(restored.1.vertices, result.1.vertices);
    assert_eq!(restored.1.edges, result.1.edges);
    assert_eq!(restored.1.faces, result.1.faces);
    let split = boolean(
        &stock,
        &cuboid([1., -1., -1.], [2., 3., 3.]).unwrap(),
        "difference",
    )
    .unwrap();
    assert!(split.1.lineage.iter().any(|record| {
        record.operation == "split"
            && matches!(record.entity_kind.as_str(), "edge" | "face")
            && record.children.len() > 1
    }));
    let merged = boolean(
        &cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(),
        &cuboid([1., 0., 0.], [3., 2., 2.]).unwrap(),
        "union",
    )
    .unwrap();
    assert!(merged.1.lineage.iter().any(|record| {
        record.operation == "merge"
            && matches!(record.entity_kind.as_str(), "edge" | "face")
            && record.parents.len() > 1
    }));
    assert_eq!(restored.1.lineage, result.1.lineage);
}

#[test]
fn concave_and_holed_extrusions_use_planar_face_loops() {
    let concave = extrude_polygon(
        &[[0., 0.], [5., 0.], [5., 2.], [3., 2.], [3., 5.], [0., 5.]],
        0.,
        2.,
    )
    .unwrap();
    assert_eq!(concave.validate().unwrap().boundary_edge_count, 0);
    let result = extrude_polygon_with_holes(
        &[[0., 0.], [5., 0.], [5., 5.], [0., 5.]],
        &[vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]]],
        0.,
        2.,
    )
    .unwrap();
    assert_eq!(
        result
            .faces
            .iter()
            .filter(|face| !face.holes.is_empty())
            .count(),
        2
    );
    assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
    assert!(
        extrude_polygon_with_holes(
            &[[0., 0.], [5., 0.], [5., 2.], [3., 2.], [3., 5.], [0., 5.]],
            &[vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]]],
            0.,
            2.,
        )
        .is_ok()
    );
}

#[test]
fn blend_records_selected_edge_replacement_lineage() {
    let source = cuboid([0.; 3], [4.; 3]).unwrap();
    let result = fillet(&source, 0, 0.5, 4).unwrap();
    assert!(result.1.lineage.iter().any(|record| {
        record.entity_kind == "edge"
            && record.parents == [source.1.edges[0]]
            && !record.children.is_empty()
    }));
}

#[test]
fn rotated_nonconvex_planar_boolean_uses_bounded_arrangement() {
    let stock = cuboid([0., 0., 0.], [4., 4., 2.]).unwrap();
    let notch = cuboid([2., 2., -1.], [5., 5., 3.]).unwrap();
    let mut nonconvex = boolean(&stock, &notch, "difference").unwrap();
    rotate_z(&mut nonconvex, std::f64::consts::PI / 9.);
    let mut cutter = cuboid([1., -1., -1.], [3., 5., 3.]).unwrap();
    rotate_z(&mut cutter, -std::f64::consts::PI / 12.);
    let result = boolean(&nonconvex, &cutter, "intersection").unwrap();
    assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(result.bodies.len(), 1);
    assert!(result.faces.len() > 6);
}

#[test]
fn faceted_round_primitives_are_labeled_topological_solids() {
    let cylinder = faceted_cylinder(2., 5., 16).unwrap();
    assert_eq!((cylinder.faces.len(), cylinder.bodies.len()), (18, 1));
    assert_eq!(cylinder.validate().unwrap().boundary_edge_count, 0);

    let sphere = faceted_sphere(2., 16, 8).unwrap();
    assert_eq!((sphere.faces.len(), sphere.bodies.len()), (224, 1));
    assert_eq!(sphere.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn convex_profile_admission_refuses_a_same_turn_pentagram() {
    let ring: Vec<[f64; 2]> = (0..5)
        .map(|i| {
            let angle = (i as f64) * std::f64::consts::TAU / 5.;
            [angle.cos(), angle.sin()]
        })
        .collect();
    let star: Vec<_> = [0, 2, 4, 1, 3].iter().map(|&i| ring[i]).collect();
    assert!(validate_convex_profile(&star, "Test").is_err());
    assert!(validate_convex_profile(&ring, "Test").is_ok());
    let twice: Vec<_> = ring.iter().chain(&ring).copied().collect();
    assert!(validate_convex_profile(&twice, "Test").is_err());
}

#[test]
fn loft_world_bounds_do_not_limit_translated_local_coordinates() {
    let sections = vec![
        vec![
            [-800000., -1., 0.],
            [800000., -1., 0.],
            [800000., 1., 0.],
            [-800000., 1., 0.],
        ],
        vec![
            [-800000., -1., 2.],
            [800000., -1., 2.],
            [800000., 1., 2.],
            [-800000., 1., 2.],
        ],
    ];
    faceted_loft(&sections).unwrap().validate().unwrap();
}

#[test]
fn faceted_loft_accepts_placed_parallel_sections_and_refuses_nonparallel_ones() {
    let c = 0.5_f64.sqrt();
    let place = |p: [f64; 3]| {
        [
            c * p[0] + c * p[2] + 5.,
            p[1] - 3.,
            -c * p[0] + c * p[2] + 7.,
        ]
    };
    let sections = vec![
        vec![[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]]
            .into_iter()
            .map(place)
            .collect(),
        vec![[-1., -1., 3.], [1., -1., 3.], [1., 1., 3.], [-1., 1., 3.]]
            .into_iter()
            .map(place)
            .collect(),
    ];
    let model = faceted_loft(&sections).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.faces.len(), 10);
    let mut tilted = sections.clone();
    tilted[1][0][2] += 0.1;
    assert!(faceted_loft(&tilted).is_err());
    let mut backwards = sections.clone();
    backwards.reverse();
    assert!(faceted_loft(&backwards).is_err());
}

#[test]
fn faceted_loft_sweep_and_revolve_are_closed_planar_breps() {
    let loft = faceted_loft(&[
        vec![[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]],
        vec![[-1., -1., 3.], [1., -1., 3.], [1., 1., 3.], [-1., 1., 3.]],
    ])
    .unwrap();
    assert_eq!((loft.faces.len(), loft.bodies.len()), (10, 1));
    assert_eq!(loft.validate().unwrap().boundary_edge_count, 0);

    let sweep = faceted_sweep(
        &[[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
        &[[0., 0., 0.], [0., 0., 3.], [2., 0., 5.]],
        [0., 1., 0.],
    )
    .unwrap();
    assert_eq!((sweep.faces.len(), sweep.bodies.len()), (18, 1));
    assert_eq!(sweep.validate().unwrap().boundary_edge_count, 0);

    let revolve = faceted_revolve(&[[0., -2.], [2., -2.], [2., 2.], [0., 2.]], 16).unwrap();
    assert_eq!(revolve.bodies.len(), 1);
    assert_eq!(revolve.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn separated_boolean_preserves_components_and_regularized_empty() {
    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    let separated = cuboid([2., 0., 0.], [3., 1., 1.]).unwrap();
    let union = boolean(&a, &separated, "union").unwrap();
    assert_eq!(union.bodies.len(), 2);
    union.validate().unwrap();
    assert!(boolean(&a, &separated, "intersection").unwrap().is_empty());
}

#[test]
fn enclosed_difference_builds_an_inner_shell_and_remains_a_boolean_operand() {
    let outer = cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = cuboid([1.; 3], [3.; 3]).unwrap();
    let cavity = boolean(&outer, &inner, "difference").unwrap();
    assert_eq!(cavity.bodies.len(), 1);
    assert_eq!(cavity.shells.len(), 2);
    assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
    cavity.validate().unwrap();
    let filled = boolean(&cavity, &inner, "union").unwrap();
    assert!(filled.bodies[0].inner_shells.is_empty());
    assert_eq!(bounds(&filled), ([0.; 3], [4.; 3]));
    filled.validate().unwrap();

    let untouched = boolean(
        &cavity,
        &cuboid([1.25; 3], [2.75; 3]).unwrap(),
        "difference",
    )
    .unwrap();
    assert_eq!(untouched.bodies[0].inner_shells.len(), 1);
    untouched.validate().unwrap();
}

#[test]
fn difference_can_split_a_body_and_chained_booleans_accept_it() {
    let stock = cuboid([0., 0., 0.], [3., 1., 1.]).unwrap();
    let splitter = cuboid([1., -1., -1.], [2., 2., 2.]).unwrap();
    let split = boolean(&stock, &splitter, "difference").unwrap();
    assert_eq!(split.bodies.len(), 2);
    assert_eq!(split.shells.len(), 2);
    split.validate().unwrap();

    let cap = cuboid([0., 0., 0.], [1.5, 1., 1.]).unwrap();
    let result = boolean(&split, &cap, "intersection").unwrap();
    assert_eq!(result.bodies.len(), 1);
    assert_eq!(bounds(&result), ([0.; 3], [1., 1., 1.]));
}

#[test]
fn box_chamfer_and_faceted_fillet_are_manifold() {
    let model = cuboid([0.; 3], [10.; 3]).unwrap();
    let chamfered = chamfer(&model, 0, 1.).unwrap();
    assert_eq!(chamfered.faces.len(), 7);
    chamfered.validate().unwrap();
    let filleted = fillet(&model, 0, 1., 8).unwrap();
    assert_eq!(filleted.faces.len(), 13);
    filleted.validate().unwrap();
}

#[test]
fn convex_profile_extrusion_and_connected_edge_chains_are_manifold() {
    let wedge = extrude_polygon(&[[0., 0.], [4., 0.], [0., 3.]], -1., 2.).unwrap();
    assert_eq!(
        (
            wedge.vertices.len(),
            wedge.edges.len(),
            wedge.faces.len(),
            wedge.bodies.len()
        ),
        (6, 9, 5, 1)
    );
    wedge.validate().unwrap();
    let slanted = wedge
        .edges
        .iter()
        .position(|edge| {
            let a = wedge.vertices[edge.vertices[0]].point;
            let b = wedge.vertices[edge.vertices[1]].point;
            (a[0] - b[0]).abs() > 1e-6 && (a[1] - b[1]).abs() > 1e-6
        })
        .unwrap();
    chamfer(&wedge, slanted, 0.25).unwrap().validate().unwrap();
    fillet(&wedge, slanted, 0.25, 6)
        .unwrap()
        .validate()
        .unwrap();

    let model = cuboid([0.; 3], [10.; 3]).unwrap();
    let connected = model.edges[0]
        .vertices
        .iter()
        .find_map(|vertex| {
            (1..model.edges.len()).find(|edge| model.edges[*edge].vertices.contains(vertex))
        })
        .unwrap();
    let chamfered = chamfer_edges(&model, &[0, connected], 1.).unwrap();
    let filleted = fillet_edges(&model, &[0, connected], 1., 4).unwrap();
    assert!(chamfered.faces.len() > 7);
    assert!(filleted.faces.len() > chamfered.faces.len());
    chamfered.validate().unwrap();
    filleted.validate().unwrap();
    assert_eq!(
        chamfer_edges(&model, &[0, 6], 1.).unwrap_err().code,
        UNSUPPORTED
    );
}

#[test]
fn edge_operations_reject_invalid_size_and_selection() {
    let model = cuboid([0.; 3], [1.; 3]).unwrap();
    assert_eq!(
        fillet(&model, 0, 0., 8).unwrap_err().code,
        "BREP_INVALID_SIZE"
    );
    assert_eq!(
        fillet(&model, 99, 0.1, 8).unwrap_err().code,
        "BREP_INVALID_SELECTION"
    );
    assert_eq!(
        fillet(&model, 0, 0.1, 64).unwrap_err().code,
        "BREP_RESOURCE_LIMIT"
    );
}
