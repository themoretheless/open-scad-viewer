//! Compare the actual non-cfg(test) production library against exported reference extraction.
#![cfg(feature = "gpu")]
use sdf_core::{Acceleration, Field, Grid, polygonize_accelerated};
#[test]
#[ignore = "requires explicit reference mesh directory and Metal GPU"]
fn production_matches_reference_bits() {
    let root = std::path::PathBuf::from(std::env::var("SDF_REFERENCE_DIR").unwrap());
    let sphere = || Field::Sphere {
        center: [0.; 3],
        radius: 1.,
    };
    let tetra = || {
        Field::from_triangles(
            geometry_ops::Triangles {
                positions: vec![1., 1., 1., -1., -1., 1., -1., 1., -1., 1., -1., -1.],
                indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
            },
            true,
        )
        .unwrap()
    };
    for (name, field, cells, mode) in [
        ("sphere-gpu-64", sphere(), 64, Acceleration::Gpu),
        ("sphere-auto-64", sphere(), 64, Acceleration::Auto),
        ("tetra-auto-48", tetra(), 48, Acceleration::Auto),
        ("tetra-auto-49", tetra(), 49, Acceleration::Auto),
        ("tetra-auto-64", tetra(), 64, Acceleration::Auto),
        ("negative-gpu-8", sphere(), 8, Acceleration::Gpu),
        ("negative-cpu-8", sphere(), 8, Acceleration::Cpu),
    ] {
        let mesh = polygonize_accelerated(
            &field,
            &Grid {
                min: [-2.; 3],
                max: [2.; 3],
                cells: [cells; 3],
            },
            mode,
        )
        .unwrap();
        let mut bytes = Vec::new();
        bytes.extend((mesh.positions.len() as u64).to_le_bytes());
        bytes.extend((mesh.indices.len() as u64).to_le_bytes());
        for p in mesh.positions {
            bytes.extend(p.to_bits().to_le_bytes());
        }
        for i in mesh.indices {
            bytes.extend((i as u64).to_le_bytes());
        }
        assert_eq!(
            bytes,
            std::fs::read(root.join(format!("{name}.bin"))).unwrap(),
            "{name}"
        );
    }
}
