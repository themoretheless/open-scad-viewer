//! File-based repair regressions for deterministic STL/OFF defect fixtures.

use manifold_core::{RepairMode, repair_with_mode, validate_indexed_mesh};

fn parse_off(source: &str) -> (Vec<f64>, Vec<usize>) {
    let tokens: Vec<&str> = source
        .lines()
        .map(|line| line.split('#').next().unwrap())
        .flat_map(str::split_whitespace)
        .collect();
    assert_eq!(tokens.first(), Some(&"OFF"));
    let vertex_count: usize = tokens[1].parse().unwrap();
    let face_count: usize = tokens[2].parse().unwrap();
    let mut cursor = 4;
    let mut positions = Vec::with_capacity(vertex_count * 3);
    for _ in 0..vertex_count {
        for _ in 0..3 {
            positions.push(tokens[cursor].parse().unwrap());
            cursor += 1;
        }
    }
    let mut indices = Vec::new();
    for _ in 0..face_count {
        let count: usize = tokens[cursor].parse().unwrap();
        cursor += 1;
        assert!(count >= 3);
        let first: usize = tokens[cursor].parse().unwrap();
        let mut previous: usize = tokens[cursor + 1].parse().unwrap();
        for offset in 2..count {
            let next: usize = tokens[cursor + offset].parse().unwrap();
            indices.extend_from_slice(&[first, previous, next]);
            previous = next;
        }
        cursor += count;
    }
    assert_eq!(cursor, tokens.len());
    (positions, indices)
}

fn parse_stl(source: &str) -> (Vec<f64>, Vec<usize>) {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for line in source.lines().map(str::trim) {
        let Some(coordinates) = line.strip_prefix("vertex ") else {
            continue;
        };
        let values: Vec<f64> = coordinates
            .split_whitespace()
            .map(|value| value.parse().unwrap())
            .collect();
        assert_eq!(values.len(), 3);
        indices.push(positions.len() / 3);
        positions.extend_from_slice(&values);
    }
    assert_eq!(indices.len() % 3, 0);
    assert!(!indices.is_empty());
    (positions, indices)
}

struct Expected {
    // Boundary, non-manifold, orientation, degenerate and pinched input defects.
    before: [usize; 5],
    vertices: usize,
    triangles: usize,
    // Weld, removal, flip, split, filled-loop and filled-triangle counts.
    stats: [usize; 6],
    // The same defect counts after repair, followed by component count.
    after: [usize; 6],
}

#[test]
fn file_corpus_matches_recorded_repair_outcomes() {
    let fixtures = [
        (
            "unwelded-cube",
            include_str!("fixtures/repair/unwelded-cube.stl"),
            RepairMode::Conservative,
            Expected {
                before: [36, 0, 0, 0, 0],
                vertices: 8,
                triangles: 12,
                stats: [28, 0, 0, 0, 0, 0],
                after: [0, 0, 0, 0, 0, 1],
            },
        ),
        (
            "degenerate-quad",
            include_str!("fixtures/repair/degenerate-quad.off"),
            RepairMode::Conservative,
            Expected {
                before: [4, 0, 0, 1, 0],
                vertices: 4,
                triangles: 2,
                stats: [0, 1, 0, 0, 0, 0],
                after: [4, 0, 0, 0, 0, 1],
            },
        ),
        (
            "flipped-quad",
            include_str!("fixtures/repair/flipped-quad.off"),
            RepairMode::Conservative,
            Expected {
                before: [4, 0, 1, 0, 0],
                vertices: 4,
                triangles: 2,
                stats: [0, 0, 1, 0, 0, 0],
                after: [4, 0, 0, 0, 0, 1],
            },
        ),
        (
            "open-box",
            include_str!("fixtures/repair/open-box.off"),
            RepairMode::Full,
            Expected {
                before: [4, 0, 0, 0, 0],
                vertices: 8,
                triangles: 12,
                stats: [0, 0, 0, 0, 1, 2],
                after: [0, 0, 0, 0, 0, 1],
            },
        ),
        (
            "triple-edge",
            include_str!("fixtures/repair/triple-edge.off"),
            RepairMode::Full,
            Expected {
                before: [6, 1, 0, 0, 0],
                vertices: 9,
                triangles: 5,
                stats: [0, 0, 0, 4, 2, 2],
                after: [3, 0, 0, 0, 0, 3],
            },
        ),
        (
            "pinched-fans",
            include_str!("fixtures/repair/pinched-fans.off"),
            RepairMode::Full,
            Expected {
                before: [4, 0, 0, 0, 1],
                vertices: 8,
                triangles: 8,
                stats: [0, 0, 0, 1, 1, 2],
                after: [0, 0, 0, 0, 0, 2],
            },
        ),
    ];
    for (name, source, mode, expected) in fixtures {
        let (positions, indices) = if source.starts_with("OFF") {
            parse_off(source)
        } else {
            parse_stl(source)
        };
        validate_indexed_mesh(&positions, &indices).unwrap();
        let before = manifold_core::check(&positions, &indices);
        assert_eq!(
            [
                before.boundary_edges.len(),
                before.non_manifold_edges.len(),
                before.orientation_edges.len(),
                before.degenerate_triangles.len(),
                before.non_manifold_vertices.len(),
            ],
            expected.before,
            "{name}: input defects changed"
        );
        // Fresh hash seeds must not change shell ownership, loop selection,
        // repair counters or the resulting topology.
        for iteration in 0..32 {
            let out = repair_with_mode(&positions, &indices, 0.0, mode);
            assert_eq!(out.positions.len() / 3, expected.vertices, "{name}");
            assert_eq!(out.indices.len() / 3, expected.triangles, "{name}");
            assert_eq!(
                [
                    out.report.welded_vertices,
                    out.report.removed_degenerate_triangles,
                    out.report.flipped_triangles,
                    out.report.split_vertices,
                    out.report.filled_holes,
                    out.report.filled_triangles,
                ],
                expected.stats,
                "{name}, iteration {iteration}: repair statistics changed"
            );
            let residual = &out.report.residual;
            assert_eq!(
                [
                    residual.boundary_edges.len(),
                    residual.non_manifold_edges.len(),
                    residual.orientation_edges.len(),
                    residual.degenerate_triangles.len(),
                    residual.non_manifold_vertices.len(),
                    residual.component_count,
                ],
                expected.after,
                "{name}, iteration {iteration}: residual topology changed"
            );
            assert!(residual.isolated_vertices.is_empty(), "{name}");
            assert_eq!(
                out.is_manifold(),
                expected.after[..5].iter().all(|&count| count == 0),
                "{name}, iteration {iteration}: manifold contract changed"
            );
        }
    }
}
