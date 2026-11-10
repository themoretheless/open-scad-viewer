//! boundary hull contact algorithm: propose_vertex_plane.
use super::*;
pub fn propose_vertex_plane(point: [f64; 3], poles: &[Vec<&Vec<f64>>; 2]) -> Option<[Vec<f64>; 2]> {
    let mut directions = Vec::new();
    for (face, ps) in poles.iter().enumerate() {
        for p in ps {
            if p.as_slice() == point {
                continue;
            }
            let mut d: [f64; 3] = std::array::from_fn(|k| p[k] - point[k]);
            let length = d.iter().map(|x| x * x).sum::<f64>().sqrt();
            if !length.is_finite() || length <= 0. {
                return None;
            }
            for x in &mut d {
                *x /= length;
                if face == 1 {
                    *x = -*x;
                }
            }
            directions.push(d);
        }
    }
    let mut normal = [0.; 3];
    for _ in 0..256 {
        let mut settled = true;
        for d in &directions {
            let dot = (0..3).map(|k| normal[k] * d[k]).sum::<f64>();
            if dot < 1. {
                settled = false;
                for k in 0..3 {
                    normal[k] += (1. - dot) * d[k];
                }
            }
        }
        if settled {
            break;
        }
    }
    let axis = (0..3).max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))?;
    if !normal[axis].is_finite() || normal[axis] == 0. {
        return None;
    }
    let free = (0..3).filter(|&k| k != axis).collect::<Vec<_>>();
    let anchors = std::array::from_fn(|i| {
        let mut p = point.to_vec();
        p[free[i]] += 1.;
        p[axis] -= normal[free[i]] / normal[axis];
        p
    });
    anchors
        .iter()
        .flatten()
        .all(|x| x.is_finite())
        .then_some(anchors)
}
