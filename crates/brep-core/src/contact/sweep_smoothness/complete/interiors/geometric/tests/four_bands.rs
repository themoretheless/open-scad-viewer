use super::*;

fn four_periodic_bands() -> crate::Model {
    use crate::{Body, Coedge, Curve, Edge, Face, FaceUse, Loop, Shell, Vertex};
    use nurbs_core::{editing::{exact_periodic_surface::open_exact_periodic_surface_axis,
        exact_surface_knot_insertion::split_tensor_surface}, surface::Axis};
    let native = periodic_chart(true);
    let opened = open_exact_periodic_surface_axis(&native, Axis::V, 1_000_000)
        .unwrap().surface.unwrap();
    let split = |s: &Surface, t| split_tensor_surface(s, Axis::V, t, 1_000_000)
        .unwrap().parts.unwrap();
    let [lo, hi] = split(&opened, 2.);
    let [a,b] = split(&lo, 1.);
    let [c,d] = split(&hi, 3.);
    let bands = [a,b,c,d];
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    for (i, s) in bands.iter().enumerate() {
        let p = &s.control_points[1][0];
        vertices.push(Vertex { point: [p[0],p[1],p[2]] });
        edges.push(Edge {
            curve: Curve {
                degree: s.degree_u, knots: s.knots_u.clone(), periodic: true,
                control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
                weights: s.weights.iter().map(|r| r[0]).collect(),
            },
            vertices: [i,i], degenerate: false,
        });
    }
    let mut loops = Vec::new();
    for (i, s) in bands.iter().enumerate() {
        let next = (i+1)%4;
        let [lo,hi] = [i as f64, i as f64 + 1.];
        edges.push(Edge {
            curve: Curve {
                degree: s.degree_v, knots: s.knots_v.clone(), periodic: false,
                control_points: s.control_points[1].clone(), weights: s.weights[1].clone(),
            },
            vertices: [i,next], degenerate: false,
        });
        loops.push(Loop { coedges: [
            (i,false,[0.,lo],[4.,lo]), (i+4,false,[4.,lo],[4.,hi]),
            (next,true,[4.,hi],[0.,hi]), (i+4,true,[0.,hi],[0.,lo]),
        ].map(|(edge, rev, a, b)| Coedge::new(edge, rev, crate::line(a.to_vec(), b.to_vec()))).to_vec() });
    }
    let mut model = crate::Model(brep_topology::Model {
        vertices, edges, loops,
        faces: bands.into_iter().enumerate().map(|(outer,surface)| Face { surface, outer, holes: vec![] }).collect(),
        shells: vec![Shell { closed: true, faces: [2,0,3,1].map(|face| FaceUse { face, reversed: false }).to_vec() }],
        bodies: vec![Body { outer_shell: 0, inner_shells: vec![] }], tolerance_mm: 1e-8,
    }, crate::TopologyIds::default());
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}

fn four_band_policy() -> crate::closed_solid::AdmissionPolicy {
    crate::closed_solid::AdmissionPolicy {
        tolerance_uv: 1e-8,
        max_linear_cells: 100000,
        geometry: crate::volume_validity::Limits {
            boundary: crate::boundary_embedding::Limits {
                exact_work: 1000000, trim_pairs: 1000, trim_cells: 10000,
                trim_domain_cells: 100000, spans: 100,
                contacts: crate::face_contacts::Limits {
                    pairs: 100, cells: 10000, domain_cells: 100000,
                    cells_per_pair: 16, domain_cells_per_pair: 1000,
                },
            },
            nesting_pairs: 100, nesting_cells: 10000, nesting_domain_cells: 100000,
            orientation_cells: 100000, orientation_domain_cells: 100000,
            orientation_spans: 1000,
        },
    }
}

#[test]
fn exact_four_band_shell_separates_nonadjacent_periodic_faces() {
    let model = four_periodic_bands();
    use crate::feature_ops::region_pattern::qualification::periodic_charts::radial_incidence;
    let mut remaining = 10000;
    let contacts = radial_incidence::certify_cycle_contacts(&model, &[2,0,3,1], &mut remaining, &mut 0, 100)
        .unwrap().unwrap();
    let mut disjoint = contacts.disjoint;
    disjoint.sort();
    assert_eq!(disjoint, [[0,2],[1,3]]);
    assert_eq!(contacts.shared.len(),4);
    for contact in contacts.shared {
        assert_eq!(contact.edges.len(),1);
    }
    let policy = four_band_policy();
    let mut limited_policy = policy;
    limited_policy.geometry.boundary.contacts.pairs = 5;
    let limited = crate::closed_solid::qualify(model.clone(), limited_policy).unwrap();
    let limited_pairs = &limited.geometry().boundary.intersections.pairs;
    assert_eq!(limited_pairs.next_pair, Some([2, 3]));
    assert!(!limited_pairs.all_pairs_classified);
    assert!(limited.solid().is_none());

    let admission = crate::closed_solid::qualify(model, policy).unwrap();
    let geometry = admission.geometry();
    let disjoint = geometry.boundary.intersections.pairs.pairs.iter()
        .filter(|pair| pair.radial_disjoint)
        .map(|pair| {
            assert_eq!(pair.reason, "pair-disjoint");
            assert!(!pair.hull_disjoint && pair.result.is_none());
            pair.faces
        }).collect::<Vec<_>>();
    assert_eq!(disjoint, [[0, 2], [1, 3]]);
    assert!(geometry.boundary.proven);
    assert_eq!(geometry.orientations[0].outward, Some(true));
    let solid = admission.into_solid().expect("four radial bands own an oriented closed solid");
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    for (point, inside) in [([4., 0.125, 0.125], true), ([0.; 3], false)] {
        assert_eq!(solid.classify_point(point, &directions, 100000, 100000).unwrap().parity, Some(inside));
    }
}

#[test]
fn four_band_solid_preserves_material_under_exact_affine_slice_frames() {
    let source = four_periodic_bands();
    for height_row in [[0., 0., 1., 0.], [0.25, 0.5, 0.125, -1.]] {
        let transform = [
            [1., 0., 0.5, 2.],
            [0., 1., -2., -3.],
            height_row,
            [0., 0., 0., 1.],
        ];
        let model = crate::transform::affine(&source, transform).unwrap();
        let continuity = super::super::super::super::inspect(&model, super::super::super::super::Limits {
            order: 2, exact_work: 16_000_000, agreement_work: 1_000_000, regularity_cells: 100_000,
        }).unwrap();
        assert!(continuity.g2_certified, "{height_row:?}: {:?}", continuity.unresolved_edges);
        let admission = crate::closed_solid::qualify(model, four_band_policy()).unwrap();
        let geometry = admission.geometry();
        assert!(geometry.boundary.proven, "{height_row:?}");
        assert_eq!(geometry.orientations[0].outward, Some(true));
        assert_eq!(geometry.boundary.intersections.pairs.pairs.iter()
            .filter(|p| p.radial_disjoint).count(), 2);
        let solid = admission.into_solid().expect("affine radial shell owns material");
        let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
        for (sample, inside) in [([4., 0.125, 0.125, 1.], true), ([0., 0., 0., 1.], false)] {
            let point = std::array::from_fn(|i| (0..4).map(|j| transform[i][j] * sample[j]).sum());
            assert_eq!(solid.classify_point(point, &directions, 100000, 100000).unwrap().parity,
                Some(inside), "{point:?}");
        }
    }
}
