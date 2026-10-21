//! Audit the exact-identity gate separately from tolerance-qualified partitioning.
use brep_core::{boundary_agreement, edge_partition};
use nurbs_core::surface::Surface;
fn holed_support(x: f64) -> nurbs_core::Result<brep_core::Model> {
    let ring = |points: &[[f64; 2]]| -> nurbs_core::Result<Vec<nurbs_core::curve::Curve>> {
        (0..points.len())
            .map(|i| {
                nurbs_core::curve::Curve::from_polyline(vec![
                    vec![points[i][0], points[i][1], x],
                    vec![
                        points[(i + 1) % points.len()][0],
                        points[(i + 1) % points.len()][1],
                        x,
                    ],
                ])
            })
            .collect()
    };
    let loops = vec![
        ring(&[[-20., -20.], [20., -20.], [20., 20.], [-20., 20.]])?,
        ring(&[[4., 4.], [4., 6.], [6., 6.], [6., 4.]])?,
    ];
    let mut model = brep_core::planar_cap::build_sheet(&loops, x, 1e-6, 100000)?;
    let rotate = |p: &mut [f64]| {
        let q = p.to_vec();
        p[0] = q[2];
        p[1] = q[0];
        p[2] = q[1];
    };
    for v in &mut model.vertices {
        rotate(&mut v.point);
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            rotate(p);
        }
    }
    for f in &mut model.faces {
        for row in &mut f.surface.control_points {
            for p in row {
                rotate(p);
            }
        }
    }
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path)?;
    }
    let mut parts = Vec::new();
    let model = brep_core::cuboid([0.; 3], [10., 8., 12.])?;
    let baseline = boundary_agreement::verify_exact(&model, 1000000)?;
    assert!(baseline.all_equal && baseline.all_joins_exact);
    let edge = model
        .edges
        .iter()
        .position(|e| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            a[1] == 0. && b[1] == 0. && a[2] == 0. && b[2] == 0. && a[0] != b[0]
        })
        .unwrap();
    for (case_name, x, mode) in [
        ("root-cut-x-1", 1., 0),
        ("root-cut-x-5", 5., 0),
        ("root-cut-curved", 1., 1),
        ("root-cut-trimmed-face", 1., 2),
    ] {
        let mut plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![x, -20., -20.], vec![x, -20., 20.]],
                vec![vec![x, 20., -20.], vec![x, 20., 20.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let out = if mode == 2 {
            let cut = holed_support(x)?;
            let admitted =
                edge_partition::partition_at_face(&model, edge, &cut, 0, 0, 1e-8, 100000)?;
            println!(
                "  trimmed_face_coverage={} roots={} eligible={} work={}",
                admitted.coverage.complete,
                admitted.coverage.roots.len(),
                admitted.coverage.eligible.len(),
                admitted.result.partition.cells
            );
            admitted.result
        } else if mode == 1 {
            plane.control_points[1][1][0] += 0.2;
            edge_partition::partition_at_surface(&model, edge, &plane, 0, 10000)?
        } else {
            edge_partition::partition_at_plane(&model, edge, &plane, 0, 10000)?
        };
        let exact = boundary_agreement::verify_exact(&out.partition.model, 1000000)?;
        println!(
            "case={case_name} x={x} root={:?} vertex_error_upper={} exact_lifts={} exact_joins={} work={}",
            out.root_parameter,
            out.root_position_error_upper,
            exact.all_equal,
            exact.all_joins_exact,
            exact.work
        );
        let contacts = brep_core::face_contacts::Limits {
            pairs: 10000,
            cells: 100000,
            domain_cells: 1000000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10000,
        };
        let limits = brep_core::volume_validity::Limits {
            boundary: brep_core::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 10000,
                trim_cells: 100000,
                trim_domain_cells: 1000000,
                spans: 1024,
                contacts,
            },
            nesting_pairs: 100,
            nesting_cells: 100000,
            nesting_domain_cells: 1000000,
            orientation_cells: 100000,
            orientation_domain_cells: 1000000,
            orientation_spans: 100,
        };
        let volume = brep_core::volume_validity::inspect(&out.partition.model, 1e-8, limits)?;
        println!(
            "  fresh_volume_proven={} exact_boundary={} trim_valid={} embedding_proven={}",
            volume.proven,
            volume.boundary.agreement.all_equal,
            volume.boundary.trim.all_valid,
            volume.boundary.intersections.absence_proven
        );
        assert!(
            volume.proven,
            "Control cuboid must retain fresh global qualification"
        );
        if let Some(path) = &output {
            use sha2::{Digest, Sha256};
            let name = case_name;
            let file = format!("{name}.step");
            let (step, _, _) = brep_core::export_step_v5(&out.partition.model)?;
            let digest = Sha256::digest(step.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            std::fs::write(path.join(&file), &step)?;
            std::fs::write(
                path.join(format!("{name}-model.json")),
                value_codec::to_string(&out.partition.model)?,
            )?;
            parts.push(value_codec::json!({"name":name,"file":file,"sha256":digest,
                "expected":{"boundsMm":[[0.,0.,0.],[10.,8.,12.]],"volumeMm3":960.,"solids":1,"vertices":9,"edges":13}}));
            std::fs::write(
                path.join("manifest.json"),
                value_codec::to_string_pretty(&value_codec::json!({
                    "schema":"cad-roadmap-step/1","toleranceMm":1e-6,"relativeVolumeTolerance":1e-10,"parts":parts
                }))?,
            )?;
        }
        for u in exact.uses.iter().filter(|u| {
            !u.decision
                .as_ref()
                .is_some_and(|d| d.outcome == cad_predicates::BezierIdentity::Equal)
        }) {
            println!(
                "  unresolved exact lift: face={} wire={} coedge={} edge={} decision={:?}",
                u.face, u.wire, u.coedge, u.edge, u.decision
            );
        }
    }
    Ok(())
}
