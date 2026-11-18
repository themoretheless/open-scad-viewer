//! Native profile investigation: fixed contacts/radii, no whole-solid admission.
use brep_core::{face_curvature, region_replacement as r};
fn main() {
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let requested = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| r::EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect::<Vec<_>>();
    for fraction in [0.125, 0.175, 0.225, 0.275, 0.3] {
        let result = r::build_trihedral_strips_with_profile(r::TrihedralCorner { source: &source, shell: 0, vertex: 0, requested: &requested, runout_mm: 0.1, span_mm: 2. }, r::StripLimits {
                tolerances: [1e-7, 1e-5, 1e-4],
                tolerance_uv: 1e-8,
                max_geometry_work: 10000,
                max_cells: 300000,
                max_exact: 1000000,
                max_coefficients: 2000000,
            }, fraction);
        let strips = match result {
            Ok(s) => s,
            Err(e) => {
                println!("fraction={fraction} construction_refused={e:?}");
                continue;
            }
        };
        for strip in &strips.strips {
            let face = &strip.patch.model.faces[0];
            let mut uv = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
            for co in &strip.patch.model.loops[face.outer].coedges {
                for p in &co.pcurve.control_points {
                    for k in 0..2 {
                        uv[k][0] = uv[k][0].min(p[k]);
                        uv[k][1] = uv[k][1].max(p[k]);
                    }
                }
            }
            let [u, v] = uv;
            // Proposals for certified INSIDE lower-bound witnesses only.
            // Hints never replace complete coverage for a positive decision.
            let hints = (1..16)
                .map(|i| [u[0] + (u[1] - u[0]) * i as f64 / 16., (v[0] + v[1]) * 0.5])
                .collect::<Vec<_>>();
            let report = face_curvature::inspect(
                &strip.patch.model,
                0,
                10.,
                1e-8,
                face_curvature::Limits {
                    cells: 5000,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
                &hints,
            );
            match report {
                Ok(p) => println!(
                    "fraction={fraction} radius={} decision={:?} upper={:?} witness={:?} reason={}",
                    strip.radius_mm,
                    p.decision,
                    p.absolute_curvature_upper,
                    p.witness_curvature,
                    p.reason
                ),
                Err(e) => println!(
                    "fraction={fraction} radius={} field_refused={e:?}",
                    strip.radius_mm
                ),
            }
        }
    }
}
