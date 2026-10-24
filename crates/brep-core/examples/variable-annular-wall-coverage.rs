//! Full original face-pair wall audit; unresolved coverage forbids safety claims.
use brep_core::{
    boundary_embedding, face_contacts, material_segment, material_wall, material_wall_coverage,
    volume_validity,
};
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            let before = value_codec::to_string(&model).unwrap();
            let theta: f64 = 0.65 * direction;
            let r = material_wall_coverage::inspect(
                &model,
                [0., 0., 3.],
                [25. * theta.cos(), 25. * theta.sin(), 0.],
                1e-5,
                1e-8,
                1e-4,
                limits(),
            )
            .unwrap();
            assert_eq!(value_codec::to_string(&model).unwrap(), before);
            assert_eq!(r.total_pairs, 496);
            assert!(r.enumeration_complete);
            assert_eq!(r.pairs.len(), 496);
            cases.push(value_codec::json!({"radii":radii,"direction":direction,"totalPairs":r.total_pairs,"enumerationComplete":r.enumeration_complete,"lowerBoundMm":r.lower_bound_mm,"intervalMm":r.interval_mm,"converged":r.converged,"reason":r.reason,"candidateProven":r.candidate.chord.proven,"candidateReason":r.candidate.chord.reason,"volumeProven":r.candidate.chord.validity.proven,"candidateAligned":r.candidate.aligned,"cells":r.cells,"domainCells":r.domain_cells,"pairs":r.pairs.iter().map(|p|value_codec::json!({"faces":p.faces,"lowerBoundMm":p.lower_bound_mm,"reason":p.reason})).collect::<Vec<_>>()}));
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-wall-coverage/1","scope":"Full original face-pair enumeration including self pairs; unresolved pairs retain conservative bounds; no global thickness claim unless converged", "cases":cases})).unwrap()).unwrap();
}

pub(crate) fn limits() -> material_wall_coverage::Limits {
    material_wall_coverage::Limits {
        wall: material_wall::Limits {
            material: material_segment::Limits {
                volume: volume_validity::Limits {
                    boundary: boundary_embedding::Limits {
                        exact_work: 1000000,
                        trim_pairs: 10000,
                        trim_cells: 4096,
                        trim_domain_cells: 65536,
                        spans: 4096,
                        contacts: face_contacts::Limits {
                            pairs: 10000,
                            cells: 10000,
                            domain_cells: 100000,
                            cells_per_pair: 256,
                            domain_cells_per_pair: 4096,
                        },
                    },
                    nesting_pairs: 100,
                    nesting_cells: 4096,
                    nesting_domain_cells: 65536,
                    orientation_cells: 4096,
                    orientation_domain_cells: 65536,
                    orientation_spans: 4096,
                },
                point_cells: 4096,
                point_domain_cells: 65536,
                segment_cells: 4096,
                segment_domain_cells: 65536,
            },
            distance_cells: 4096,
            distance_domain_cells: 65536,
            normal_spans: 4096,
        },
        pairs: 10000,
        plane_controls: 10000,
        normal_spans: 10000,
    }
}
