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
    a: BoxSpec,
    b: BoxSpec,
}

#[derive(Deserialize)]
struct BoxSpec {
    origin: [f64; 3],
    size: [f64; 3],
}

impl BoxSpec {
    fn mesh(&self) -> (Vec<f64>, Vec<usize>) {
        assert!(self.origin.iter().chain(&self.size).all(|v| v.is_finite()));
        assert!(self.size.iter().all(|&v| v > 0.0));
        let [x, y, z] = self.origin;
        let [sx, sy, sz] = self.size;
        #[rustfmt::skip]
        let positions = vec![
            x, y, z, x + sx, y, z, x + sx, y + sy, z, x, y + sy, z,
            x, y, z + sz, x + sx, y, z + sz, x + sx, y + sy, z + sz, x, y + sy, z + sz,
        ];
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
