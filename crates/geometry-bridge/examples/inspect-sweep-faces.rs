//! Inspect independent whole-face proof work without enumerating contact pairs.
use std::io::{self, Read};
fn main() {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text).unwrap();
    let model: brep_core::Model = value_codec::from_str(&text).unwrap();
    let budget: usize = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "20000".into())
        .parse()
        .unwrap();
    let report = brep_core::face_injectivity::inspect_with_linear(&model, 1000, budget).unwrap();
    println!(
        "all={} spans={} linear_cells={}",
        report.all_faces_injective, report.spans, report.linear_cells
    );
    for face in &report.faces {
        if !face.absence_proven() || face.linear.as_ref().is_some_and(|r| r.cells > 64) {
            println!(
                "face={} contraction={:?} linear={:?}",
                face.face,
                face.result.as_ref().map(|r| (r.reason, r.spans)),
                face.linear
                    .as_ref()
                    .map(|r| (r.certified, r.cells, r.projection, r.reason))
            );
        }
    }
}
