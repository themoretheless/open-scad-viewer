use super::count_components;

fn reference_components(triangles: &[[usize; 3]]) -> usize {
    let valid = |t: &[usize; 3]| t[0] != t[1] && t[1] != t[2] && t[0] != t[2];
    let mut seen = vec![false; triangles.len()];
    let mut count = 0;
    for start in 0..triangles.len() {
        if seen[start] {
            continue;
        }
        count += 1;
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(a) = stack.pop() {
            if !valid(&triangles[a]) {
                continue;
            }
            for b in 0..triangles.len() {
                if !seen[b]
                    && valid(&triangles[b])
                    && triangles[a]
                        .iter()
                        .filter(|v| triangles[b].contains(v))
                        .count()
                        >= 2
                {
                    seen[b] = true;
                    stack.push(b);
                }
            }
        }
    }
    count
}

#[test]
fn component_count_matches_independent_face_adjacency_traversal() {
    let mut state = 0x7a31_c259_u32;
    for length in 0..96 {
        let mut triangles: Vec<[usize; 3]> = (0..length)
            .map(|_| {
                std::array::from_fn(|_| {
                    state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                    (state >> 16) as usize % 23
                })
            })
            .collect();
        assert_eq!(
            count_components(&triangles),
            reference_components(&triangles)
        );
        triangles.reverse();
        assert_eq!(
            count_components(&triangles),
            reference_components(&triangles)
        );
    }
    // Vertex-only contact does not join faces; degenerate faces stay isolated.
    let triangles = [[0, 1, 2], [2, 3, 4], [1, 2, 5], [0, 0, 1], [8, 9, 10]];
    assert_eq!(count_components(&triangles), 4);
    assert_eq!(count_components(&[[4, 4, 4]; 5]), 5);
}
