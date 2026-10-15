//! Viewport octahedron markers for ranked lattice weak spots.
use super::*;

const FACES: [[usize; 3]; 8] = [
    [0, 2, 4], [0, 4, 3], [0, 3, 5], [0, 5, 2],
    [1, 4, 2], [1, 3, 4], [1, 5, 3], [1, 2, 5],
];

/// Triangle soup (9 floats per triangle) of an octahedron of half-size `s` at `at`.
pub(super) fn octahedron(at: [f64; 3], s: f64) -> Vec<f64> {
    let [x, y, z] = at;
    let v = [[x, y + s, z], [x, y - s, z], [x + s, y, z], [x - s, y, z], [x, y, z + s], [x, y, z - s]];
    FACES.iter().flat_map(|f| f.iter().flat_map(|&i| v[i])).collect()
}

/// `cad_lattice_weak_spot_markers`: `{spots:[{at, score}], markerMm}` → one
/// triangle soup per spot, scaled by score (0.75..1.25 × marker size).
pub fn weak_spot_markers(v: Value) -> Result<Value> {
    let marker: f64 = field(&v, "markerMm")?;
    if !marker.is_finite() || marker <= 0. {
        return Err(input("Marker size must be positive."));
    }
    let size = marker.max(0.4);
    let spots = v["spots"]
        .as_array()
        .ok_or_else(|| input("spots must be an array."))?;
    if spots.len() > 64 {
        return Err(input("At most 64 weak-spot markers."));
    }
    let mut out = Vec::with_capacity(spots.len());
    for spot in spots {
        let at: [f64; 3] = field(spot, "at")?;
        let score: f64 = field(spot, "score")?;
        if !at.iter().all(|x| x.is_finite()) || !score.is_finite() {
            return Err(input("Weak-spot markers need finite positions and scores."));
        }
        out.push(octahedron(at, size * (0.75 + 0.5 * score.clamp(0., 1.))));
    }
    encode(out)
}
