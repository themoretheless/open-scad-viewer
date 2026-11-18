use brep_core::Model;
use brep_core::direct_edit::complete_incremental_rebuild;
use nurbs_core::foundation::guards::Budget;
use nurbs_core::surface::Surface;
use std::time::Instant;

fn translated_cube(offset: [f64; 3]) -> Model {
    let mut model = brep_core::cuboid([0.; 3], [1., 1., 1.]).unwrap();
    for vertex in &mut model.vertices {
        for axis in 0..3 {
            vertex.point[axis] += offset[axis];
        }
    }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points {
            for axis in 0..3 {
                point[axis] += offset[axis];
            }
        }
    }
    for face in &mut model.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            for axis in 0..3 {
                point[axis] += offset[axis];
            }
        }
    }
    model
}

fn append(target: &mut Model, part: &Model) {
    let [v, e, l, f, s] = [
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
    ];
    target.vertices.extend(part.vertices.iter().cloned());
    target
        .edges
        .extend(part.edges.iter().cloned().map(|mut edge| {
            edge.vertices = edge.vertices.map(|vertex| vertex + v);
            edge
        }));
    target
        .loops
        .extend(part.loops.iter().cloned().map(|mut wire| {
            for coedge in &mut wire.coedges {
                coedge.edge += e;
            }
            wire
        }));
    target
        .faces
        .extend(part.faces.iter().cloned().map(|mut face| {
            face.outer += l;
            for hole in &mut face.holes {
                *hole += l;
            }
            face
        }));
    target
        .shells
        .extend(part.shells.iter().cloned().map(|mut shell| {
            for face in &mut shell.faces {
                face.face += f;
            }
            shell
        }));
    target
        .bodies
        .extend(part.bodies.iter().cloned().map(|mut body| {
            body.outer_shell += s;
            for shell in &mut body.inner_shells {
                *shell += s;
            }
            body
        }));
    target.1.vertices.extend(&part.1.vertices);
    target.1.edges.extend(&part.1.edges);
    target.1.loops.extend(&part.1.loops);
    target.1.faces.extend(&part.1.faces);
    target.1.shells.extend(&part.1.shells);
    target.1.bodies.extend(&part.1.bodies);
}

fn separated_cuboids(count: usize) -> Model {
    let mut model = Model::empty(1e-6).unwrap();
    for index in 0..count {
        append(&mut model, &translated_cube([3. * index as f64, 0., 0.]));
    }
    model.rebuild_topology_ids();
    model
}

fn perturb_interior(face: &mut brep_core::Face) {
    let old = &face.surface;
    let mut surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![vec![]; 3]; 3],
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    for (row, points) in surface.control_points.iter_mut().enumerate() {
        for (column, point) in points.iter_mut().enumerate() {
            let u = row as f64 / 2.;
            let v = column as f64 / 2.;
            *point = old.evaluate(u, v).unwrap().point.to_vec();
            if row == 1 && column == 1 {
                point[2] += 1e-4;
            }
        }
    }
    face.surface = surface;
}

#[test]
#[ignore = "explicit 10000-face incremental-rebuild performance benchmark"]
fn incremental_rebuild_10000_faces_reports_actual_rebuilt_ratio() {
    let source = separated_cuboids(1667); // 10,002 closed faces, 1,667 bodies.
    assert!(source.faces.len() >= 10_000);
    let seed = 1; // The +Z face of the first separated cuboid.
    let mut staged = source.clone();
    perturb_interior(&mut staged.faces[seed]);
    let budget = Budget::new(50_000_000, 8, 300_000).unwrap();
    let started = Instant::now();
    let report = complete_incremental_rebuild(&source, staged, &[seed], &budget).unwrap();
    let elapsed = started.elapsed();
    let ratio = report.rebuilt_faces as f64 / source.faces.len() as f64;
    println!(
        "source_faces={} rebuilt_faces={} reused_faces={} rebuilt_ratio={:.8} elapsed_ms={} dirty_faces={} full_validation_fallback={}",
        source.faces.len(),
        report.rebuilt_faces,
        report.reused_faces,
        ratio,
        elapsed.as_secs_f64() * 1000.,
        report.dirty.len(),
        report.validation.full_validation_fallback,
    );
    assert_eq!(report.rebuilt_faces, 1);
    assert_eq!(report.reused_faces, source.faces.len() - 1);
    assert!(ratio < 0.01, "actual rebuilt ratio {ratio} is not below 1%");
    assert!(report.validation.publishable());
    assert_eq!(report.model.1, source.1);
}
