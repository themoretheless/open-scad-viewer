//! Deterministic load fixtures shared by the manifold-ops benchmark examples.
#![allow(dead_code)]

use std::collections::HashMap;

/// Unit icosphere with `subdivisions` midpoint refinements: `20 * 4^n`
/// triangles, `10 * 4^n + 2` vertices, closed, outward, genus 0.
pub fn icosphere(subdivisions: usize) -> (Vec<f64>, Vec<usize>) {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;
    #[rustfmt::skip]
    let mut positions: Vec<f64> = vec![
        -1.,  t, 0.,  1.,  t, 0., -1., -t, 0.,  1., -t, 0.,
         0., -1.,  t,  0.,  1.,  t,  0., -1., -t,  0.,  1., -t,
         t,  0., -1.,  t,  0.,  1., -t,  0., -1., -t,  0.,  1.,
    ];
    #[rustfmt::skip]
    let mut indices: Vec<usize> = vec![
        0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11,
        1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7, 1, 8,
        3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9,
        4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9, 8, 1,
    ];
    let unit = |v: [f64; 3]| {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / len, v[1] / len, v[2] / len]
    };
    for v in positions.as_chunks_mut::<3>().0 {
        *v = unit(*v);
    }
    for _ in 0..subdivisions {
        let mut midpoint: HashMap<(usize, usize), usize> =
            HashMap::with_capacity(indices.len() / 2);
        let mut next = Vec::with_capacity(indices.len() * 4);
        let mut mid = |a: usize, b: usize, positions: &mut Vec<f64>| -> usize {
            *midpoint.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let id = positions.len() / 3;
                let m = unit([
                    (positions[a * 3] + positions[b * 3]) / 2.,
                    (positions[a * 3 + 1] + positions[b * 3 + 1]) / 2.,
                    (positions[a * 3 + 2] + positions[b * 3 + 2]) / 2.,
                ]);
                positions.extend_from_slice(&m);
                id
            })
        };
        for &[a, b, c] in indices.as_chunks::<3>().0 {
            let ab = mid(a, b, &mut positions);
            let bc = mid(b, c, &mut positions);
            let ca = mid(c, a, &mut positions);
            next.extend_from_slice(&[a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca]);
        }
        indices = next;
    }
    (positions, indices)
}

/// Scale then translate a mesh in place.
pub fn place(positions: &mut [f64], scale: f64, offset: [f64; 3]) {
    for v in positions.as_chunks_mut::<3>().0 {
        *v = std::array::from_fn(|k| v[k] * scale + offset[k]);
    }
}

/// Face soup: every triangle carries its own copy of its three corners.
pub fn unwelded(positions: &[f64], indices: &[usize]) -> (Vec<f64>, Vec<usize>) {
    let mut p = Vec::with_capacity(indices.len() * 3);
    let mut i = Vec::with_capacity(indices.len());
    for &corner in indices {
        i.push(p.len() / 3);
        p.extend_from_slice(&positions[corner * 3..corner * 3 + 3]);
    }
    (p, i)
}
