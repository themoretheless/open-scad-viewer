//! JSON adapter for the isolated manifold-3d differential sidecar.

use manifold_ops::{BooleanOp, boolean_manifold};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    a: Solid,
    b: Solid,
}

/// Either an axis-aligned box (optionally rotated in degrees about X, then
/// Y, then Z around the world origin, then translated to `origin`, matching
/// manifold-3d `cube(size).rotate(r).translate(origin)`), or an explicit mesh.
#[derive(Deserialize)]
#[serde(untagged)]
enum Solid {
    Mesh { mesh: MeshSpec },
    Box(BoxSpec),
}

#[derive(Deserialize)]
struct MeshSpec {
    positions: Vec<f64>,
    indices: Vec<usize>,
}

#[derive(Deserialize)]
struct BoxSpec {
    origin: [f64; 3],
    size: [f64; 3],
    #[serde(default)]
    rotate: Option<[f64; 3]>,
}

impl Solid {
    fn mesh(&self) -> (Vec<f64>, Vec<usize>) {
        match self {
            Solid::Mesh { mesh } => (mesh.positions.clone(), mesh.indices.clone()),
            Solid::Box(b) => b.mesh(),
        }
    }
}

/// Degree sine/cosine with exact quarter turns, like manifold's `sind`/`cosd`.
fn sincosd(deg: f64) -> (f64, f64) {
    let r = deg.rem_euclid(360.0);
    if r % 90.0 == 0.0 {
        return match (r / 90.0) as i32 {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        };
    }
    deg.to_radians().sin_cos()
}

impl BoxSpec {
    fn mesh(&self) -> (Vec<f64>, Vec<usize>) {
        assert!(self.origin.iter().chain(&self.size).all(|v| v.is_finite()));
        assert!(self.size.iter().all(|&v| v > 0.0));
        let [sx, sy, sz] = self.size;
        #[rustfmt::skip]
        let mut positions = vec![
            0.0, 0.0, 0.0, sx, 0.0, 0.0, sx, sy, 0.0, 0.0, sy, 0.0,
            0.0, 0.0, sz, sx, 0.0, sz, sx, sy, sz, 0.0, sy, sz,
        ];
        if let Some([rx, ry, rz]) = self.rotate {
            let (s, c) = sincosd(rx);
            let (t, d) = sincosd(ry);
            let (u, e) = sincosd(rz);
            for p in positions.chunks_mut(3) {
                let (x, y, z) = (p[0], p[1], p[2]);
                let (y, z) = (c * y - s * z, s * y + c * z);
                let (x, z) = (d * x + t * z, -t * x + d * z);
                let (x, y) = (e * x - u * y, u * x + e * y);
                p.copy_from_slice(&[x, y, z]);
            }
        }
        for p in positions.chunks_mut(3) {
            for k in 0..3 {
                p[k] += self.origin[k];
            }
        }
        #[rustfmt::skip]
        let indices = vec![
            0, 2, 1, 0, 3, 2,
            4, 5, 6, 4, 6, 7,
            0, 1, 5, 0, 5, 4,
            2, 3, 7, 2, 7, 6,
            1, 2, 6, 1, 6, 5,
            3, 0, 4, 3, 4, 7,
        ];
        (positions, indices)
    }
}

#[derive(Serialize)]
struct Report {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    results: Vec<ResultRecord>,
}

#[derive(Serialize)]
struct ResultRecord {
    id: String,
    op: String,
    error: Option<String>,
    volume: Option<f64>,
    surface_area: Option<f64>,
    genus: Option<i64>,
    repaired: Option<[bool; 2]>,
}

impl ResultRecord {
    fn error(id: &str, op: &str, error: String) -> Self {
        Self {
            id: id.into(),
            op: op.into(),
            error: Some(error),
            volume: None,
            surface_area: None,
            genus: None,
            repaired: None,
        }
    }
}

fn main() {
    let corpus: Corpus = serde_json::from_reader(std::io::stdin()).unwrap();
    let mut results = Vec::new();
    for case in &corpus.cases {
        let (ap, ai) = case.a.mesh();
        let (bp, bi) = case.b.mesh();
        for (name, op) in [
            ("union", BooleanOp::Union),
            ("intersection", BooleanOp::Intersection),
            ("difference", BooleanOp::Difference),
        ] {
            match boolean_manifold(&ap, &ai, &bp, &bi, op) {
                Ok(out) => {
                    let metrics = manifold_core::metrics(&out.positions, &out.indices);
                    // manifold-3d reports whole-mesh genus (1 - χ/2), not
                    // manifold-core's sum of independently decomposed genera.
                    // Its empty-solid convention consequently has genus 1.
                    let genus = if metrics.components.is_empty() {
                        Some(1)
                    } else if metrics.watertight && metrics.euler_characteristic % 2 == 0 {
                        Some(1 - metrics.euler_characteristic / 2)
                    } else {
                        None
                    };
                    results.push(ResultRecord {
                        id: case.id.clone(),
                        op: name.into(),
                        error: None,
                        volume: Some(out.volume()),
                        surface_area: Some(out.surface_area()),
                        genus,
                        repaired: Some([out.operand_repairs[0].is_some(), out.operand_repairs[1].is_some()]),
                    });
                }
                Err(error) => results.push(ResultRecord::error(&case.id, name, error.to_string())),
            }
        }
    }
    serde_json::to_writer(
        std::io::stdout(),
        &Report {
            schema_version: 1,
            results,
        },
    )
    .unwrap();
}
