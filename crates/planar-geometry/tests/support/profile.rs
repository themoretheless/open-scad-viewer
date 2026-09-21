use math_core::{cross2, sub2};
use planar_geometry::tessellation::FillMesh;
use std::collections::BTreeMap;

pub fn grid(side: usize, segments: usize) -> (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>) {
    let extent = 4. * side as f64 + 2.;
    let outer = vec![[0., 0.], [extent, 0.], [extent, extent], [0., extent]];
    let holes = (0..side * side)
        .map(|i| {
            let center = [3. + 4. * (i % side) as f64, 3. + 4. * (i / side) as f64];
            (0..segments)
                .map(|j| {
                    let angle = std::f64::consts::TAU * j as f64 / segments as f64;
                    [center[0] + angle.cos(), center[1] + angle.sin()]
                })
                .collect()
        })
        .collect();
    (outer, holes)
}

pub fn signed_area(ring: &[[f64; 2]]) -> f64 {
    (1..ring.len() - 1)
        .map(|i| cross2(sub2(ring[i], ring[0]), sub2(ring[i + 1], ring[0])) / 2.)
        .sum()
}

pub fn validate(mesh: &FillMesh, outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) {
    let key = |p: [f64; 2]| p.map(|v| if v == 0. { 0 } else { v.to_bits() });
    let input: BTreeMap<_, _> = outer
        .iter()
        .chain(holes.iter().flatten())
        .enumerate()
        .map(|(i, &p)| (key(p), i))
        .collect();
    let ids: Vec<_> = mesh
        .positions
        .iter()
        .map(|&p| *input.get(&key(p)).expect("new boundary coordinate"))
        .collect();
    let mut edges = BTreeMap::<(usize, usize), usize>::new();
    let mut area = 0.;
    for t in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = [
            mesh.positions[t[0] as usize],
            mesh.positions[t[1] as usize],
            mesh.positions[t[2] as usize],
        ];
        let triangle_area = cross2(sub2(b, a), sub2(c, a)) / 2.;
        assert!(triangle_area > 0., "nonpositive triangle {a:?} {b:?} {c:?}");
        area += triangle_area;
        for j in 0..3 {
            *edges
                .entry((ids[t[j] as usize], ids[t[(j + 1) % 3] as usize]))
                .or_default() += 1;
        }
    }
    let expected =
        signed_area(outer).abs() - holes.iter().map(|h| signed_area(h).abs()).sum::<f64>();
    assert!(
        (area - expected).abs() < expected * 1e-10,
        "area {area} != {expected}"
    );
    let mut boundary = BTreeMap::new();
    for (ring_index, ring) in std::iter::once(outer)
        .chain(holes.iter().map(Vec::as_slice))
        .enumerate()
    {
        let reverse = (signed_area(ring) > 0.) != (ring_index == 0);
        for i in 0..ring.len() {
            let (a, b) = (
                input[&key(ring[i])],
                input[&key(ring[(i + 1) % ring.len()])],
            );
            boundary.insert(if reverse { (b, a) } else { (a, b) }, 1);
        }
    }
    for (&edge, &count) in &boundary {
        assert_eq!(edges.get(&edge), Some(&count), "missing boundary {edge:?}");
    }
    for (&(a, b), &count) in &edges {
        assert_eq!(count, 1, "overlapping directed edge");
        if !boundary.contains_key(&(a, b)) {
            assert_eq!(edges.get(&(b, a)), Some(&1), "unpaired interior edge");
        } else {
            assert!(!edges.contains_key(&(b, a)), "filled hole/outside edge");
        }
    }
}
