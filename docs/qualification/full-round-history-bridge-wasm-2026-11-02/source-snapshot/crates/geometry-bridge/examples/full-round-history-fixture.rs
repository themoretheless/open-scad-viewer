//! Native frozen-history fixture for cross-runtime replay qualification.
use brep_core::feature_ops::{
    full_round_curved::admitted::history::{Budget, History, codec},
    full_round_exact,
};
use value_codec::json;
fn main() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let mut history = History::new(
        full_round_exact(&source, [5, 4, 3], Default::default()).unwrap(),
        Budget::default(),
    )
    .unwrap();
    let changed = brep_core::transform::affine(
        &source,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1.25, 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    history.rebase_source(&changed).unwrap();
    history.undo().unwrap();
    let work = history.work();
    let output = json!({"format":"native-full-round-history-fixture/1","history":codec::export(&history).unwrap(),
        "work":{"pairProjections":work.pair_projections,"atoms":work.atoms,"rayCells":work.ray_cells}});
    println!("{}", value_codec::to_string(&output).unwrap());
}
