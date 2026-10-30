use super::*;
#[test]
fn skewed_station_hulls_meet_only_at_owned_vertex() {
    let section = |z: f64, t: f64| {
        vec![
            [
                [[0.5, 0.], [0.5, 0.5], [0., 0.5]],
                [[0., 0.5], [-0.5, 0.5], [-0.5, 0.]],
                [[-0.5, 0.], [-0.5, -0.5], [0., -0.5]],
                [[0., -0.5], [0.5, -0.5], [0.5, 0.]],
            ]
            .into_iter()
            .map(|p| nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p
                    .into_iter()
                    .map(|[x, y]| vec![x - t * y, y + t * x, z])
                    .collect(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .collect(),
        ]
    };
    let model =
        crate::rational_section_loft(&[section(0., 0.), section(5., 0.25), section(10., 0.5)])
            .unwrap();
    let c = certify(&model, [0, 7]).expect("skewed common station vertex");
    assert!(c.vertex.is_some());
    assert!(c.edges.is_empty());
    let mut changed = model.clone();
    changed.vertices[c.vertex.unwrap()].point[0] =
        changed.vertices[c.vertex.unwrap()].point[0].next_up();
    assert!(certify(&changed, [0, 7]).is_none());
}
#[test]
fn quintic_station_vertex_support_uses_bounded_exact_plane_proposals() {
    let section = |z: f64, t: f64| {
        vec![
            [
                [[0.5, 0.], [0.5, 0.5], [0., 0.5]],
                [[0., 0.5], [-0.5, 0.5], [-0.5, 0.]],
                [[-0.5, 0.], [-0.5, -0.5], [0., -0.5]],
                [[0., -0.5], [0.5, -0.5], [0.5, 0.]],
            ]
            .into_iter()
            .map(|p| nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p
                    .into_iter()
                    .map(|[x, y]| vec![x - t * y, y + t * x, z])
                    .collect(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .collect(),
        ]
    };
    for degree in [5, 13] {
        let mut model = crate::rational_section_loft(&[
            section(0., 0.),
            section(5., 0.25),
            section(10., 0.5),
        ])
        .unwrap();
        for face in &mut model.faces[..8] {
            face.surface = nurbs_core::surface_edit::elevate(
                &face.surface,
                nurbs_core::surface::Axis::V,
                degree,
            )
            .unwrap();
        }
        // Dyadic nonsingular placement makes all world-axis hulls overlap.
        let transform = |p: &mut [f64]| {
            let [x, y, z] = [p[0], p[1], p[2]];
            p[0] = x + 0.125 * z;
            p[1] = y + 0.25 * z;
            p[2] = z + 0.25 * x + 0.125 * y;
        };
        for face in &mut model.faces {
            for p in face.surface.control_points.iter_mut().flatten() {
                transform(p);
            }
        }
        for edge in &mut model.edges {
            for p in &mut edge.curve.control_points {
                transform(p);
            }
        }
        for vertex in &mut model.vertices {
            transform(&mut vertex.point);
        }
        let original = model.clone();
        assert_eq!(
            model.faces[0]
                .surface
                .control_points
                .iter()
                .flatten()
                .count(),
            3 * (degree + 1)
        );
        let certificate =
            certify(&model, [0, 7]).expect("actual retained high-degree vertex support");
        let vertex = certificate.vertex.unwrap();
        assert!(certificate.edges.is_empty());
        let mut broken = model.clone();
        broken.vertices[vertex].point[0] = broken.vertices[vertex].point[0].next_up();
        assert!(certify(&broken, [0, 7]).is_none());
        let mut overlap = model.clone();
        overlap.faces[7].surface.control_points[1][2] =
            model.faces[0].surface.control_points[1][2].clone();
        assert!(certify(&overlap, [0, 7]).is_none());
        assert_eq!(model, original);
    }
}
#[test]
fn retained_hollow_sweep_vertex_contacts_require_authored_vertex_ownership() {
    let section = |z| {
        // Author exact quadrant endpoints rather than rounded trig values.
        let ring = |r: f64| {
            [
                [[r, 0.], [r, r], [0., r]],
                [[0., r], [-r, r], [-r, 0.]],
                [[-r, 0.], [-r, -r], [0., -r]],
                [[0., -r], [r, -r], [r, 0.]],
            ]
            .into_iter()
            .map(|p| nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.into_iter().map(|xy| vec![xy[0], xy[1], z]).collect(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .collect::<Vec<_>>()
        };
        let outer = ring(0.5);
        let inner = ring(0.2)
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
        vec![outer, inner]
    };
    let model =
        crate::rational_section_loft(&[section(0.), section(5.), section(10.)]).unwrap();
    let before = model.clone();
    let walls = model.faces.len() - 2;
    let mut vertices = Vec::new();
    for a in 0..walls {
        for b in a + 1..walls {
            if let Some(c) = certify(&model, [a, b]) {
                if c.vertex.is_some() {
                    vertices.push(c);
                }
            }
        }
    }
    assert!(
        !vertices.is_empty(),
        "actual diagonal wall panels must exercise vertex-only contact"
    );
    for c in vertices {
        let vertex = c.vertex.unwrap();
        let mut broken = model.clone();
        // Keep all geometry identical but give one face independent edges
        // and vertices. Coordinate coincidence cannot authorize contact.
        let f = &model.faces[c.faces[1]];
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for i in 0..broken.loops[wire].coedges.len() {
                let mut edge = model.edges[broken.loops[wire].coedges[i].edge].clone();
                for v in &mut edge.vertices {
                    let point = model.vertices[*v].clone();
                    *v = broken.vertices.len();
                    broken.vertices.push(point);
                }
                broken.loops[wire].coedges[i].edge = broken.edges.len();
                broken.edges.push(edge);
            }
        }
        assert!(certify(&broken, c.faces).is_none(), "{c:?}");
        let mut displaced = model.clone();
        displaced.vertices[vertex].point[0] = displaced.vertices[vertex].point[0].next_up();
        assert!(
            certify(&displaced, c.faces).is_none(),
            "one ULP invalidates exact ownership"
        );
    }
    assert_eq!(model, before);
}
#[test]
fn coincident_geometry_without_shared_topology_is_not_authorized() {
    let m = crate::operations::boolean(
        &crate::cuboid([0.; 3], [10.; 3]).unwrap(),
        &crate::cuboid([2.; 3], [8.; 3]).unwrap(),
        "difference",
    )
    .unwrap();
    let mut examples = [None, None];
    for a in 0..m.faces.len() {
        for b in a + 1..m.faces.len() {
            if let Some(c) = certify(&m, [a, b]) {
                let k = usize::from(c.vertex.is_some());
                if examples[k].is_none() {
                    examples[k] = Some(c);
                }
            }
        }
    }
    for c in examples.into_iter().map(Option::unwrap) {
        let mut broken = m.clone();
        let face = &m.faces[c.faces[1]];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for i in 0..broken.loops[wire].coedges.len() {
                let old = broken.loops[wire].coedges[i].edge;
                let mut edge = broken.edges[old].clone();
                for v in &mut edge.vertices {
                    let vertex = broken.vertices[*v].clone();
                    *v = broken.vertices.len();
                    broken.vertices.push(vertex);
                }
                broken.loops[wire].coedges[i].edge = broken.edges.len();
                broken.edges.push(edge);
            }
        }
        assert!(certify(&broken, c.faces).is_none());
    }
}
#[test]
fn overlapping_face_interiors_and_microscopic_gaps_are_not_shared_boundaries() {
    let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    assert!(certify(&m, [0, 0]).is_none());
    let c = (0..6)
        .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
        .find_map(|p| certify(&m, p))
        .unwrap();
    let axis = (0..3)
        .find(|&k| c.hull_intersection[k][0] < c.hull_intersection[k][1])
        .unwrap();
    for e in c.edges {
        for p in &mut m.edges[e].curve.control_points {
            p[(axis + 1) % 3] += 1e-12;
        }
    }
    assert!(certify(&m, c.faces).is_none());
}
