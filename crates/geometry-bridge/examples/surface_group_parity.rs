//! Native conformance harness; not a production transport or WASM benchmark.
use geometry_bridge::mesh_surface_groups::surface_group_ids;
use std::io::{self, BufRead, Write};
use value_codec::{Value, from_str, from_value, json};

fn main() {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    for line in io::stdin().lock().lines() {
        let input: Value = from_str(&line.unwrap()).unwrap();
        let vertices: Vec<f32> = from_value(input["vertices"].clone()).unwrap();
        let indices: Vec<u32> = from_value(input["indices"].clone()).unwrap();
        let angle = input["angle"].as_f64().unwrap();
        let result = match surface_group_ids(6, &vertices, &indices, angle) {
            Ok(ids) => json!({"ids": ids}),
            Err(error) => json!({"error": error.message}),
        };
        writeln!(output, "{result}").unwrap();
    }
}
