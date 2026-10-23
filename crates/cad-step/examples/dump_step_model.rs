//! Prints the value_codec JSON of a STEP v9 import. brep-core's own tests read
//! such fixtures instead of depending on this crate.
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: dump_step_model <file.step>");
    let text = std::fs::read_to_string(&path).expect("read STEP file");
    let (model, _, _) = cad_step::import_step_v9(&text).expect("import STEP v9");
    println!(
        "{}",
        value_codec::to_string(&model).expect("serialize model")
    );
}
