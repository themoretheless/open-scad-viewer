//! Bounded octahedral triangle soups for signed axial-force visualization.
use crate::{Result, fail as error};
pub fn build(
    nodes: &[[f64; 3]],
    members: &[[usize; 2]],
    forces: &[f64],
    marker: f64,
) -> Result<[Vec<f32>; 3]> {
    if !marker.is_finite()
        || marker <= 0.
        || nodes.is_empty()
        || nodes.len() > 125
        || members.is_empty()
        || members.len() > 400
        || forces.len() != members.len()
        || forces
            .iter()
            .chain(nodes.iter().flatten())
            .any(|v| !v.is_finite())
    {
        return Err(error("Invalid axial force field."));
    }
    let mut groups: [Vec<f32>; 3] = std::array::from_fn(|_| Vec::new());
    let faces = [
        [0, 2, 4],
        [0, 4, 3],
        [0, 3, 5],
        [0, 5, 2],
        [1, 4, 2],
        [1, 3, 4],
        [1, 5, 3],
        [1, 2, 5],
    ];
    for (pair, force) in members.iter().zip(forces) {
        let [a, b] = *pair;
        if a == b || a >= nodes.len() || b >= nodes.len() {
            return Err(error("Invalid axial force member."));
        }
        let mid = std::array::from_fn::<_, 3, _>(|k| nodes[a][k] / 2. + nodes[b][k] / 2.);
        let r = marker / 2.;
        if mid.iter().any(|v| (*v + r) as f32 == (*v - r) as f32) {
            return Err(error(
                "Marker size is too small at these display coordinates; increase the marker size.",
            ));
        }
        let p = [
            [mid[0] + r, mid[1], mid[2]],
            [mid[0] - r, mid[1], mid[2]],
            [mid[0], mid[1] + r, mid[2]],
            [mid[0], mid[1] - r, mid[2]],
            [mid[0], mid[1], mid[2] + r],
            [mid[0], mid[1], mid[2] - r],
        ];
        let group = &mut groups[if *force < 0. {
            0
        } else if *force > 0. {
            2
        } else {
            1
        }];
        for face in faces {
            for vertex in face {
                group.extend(p[vertex].map(|x| x as f32));
            }
        }
    }
    Ok(groups)
}
