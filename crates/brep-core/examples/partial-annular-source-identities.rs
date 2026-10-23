use brep_core::{analytic_features::build_partial_annular_preview, tube};

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&directory).unwrap();
    let source = tube(20., 5., 6.).unwrap();
    let mut rows = Vec::new();
    for edge in 0..source.edges.len() {
        let Ok(result) = build_partial_annular_preview(&source, edge, 1.25) else {
            continue;
        };
        let unchanged_vertices=source.vertices.iter().enumerate().filter_map(|(i,v)| {
            result.vertices.iter().position(|target|target.point==v.point).map(|j|
                value_codec::json!({"source":i,"target":j,"retained":source.1.vertices[i]==result.1.vertices[j]}))
        }).collect::<Vec<_>>();
        let counts = value_codec::json!({
            "vertices": source.1.vertices.iter().filter(|id|result.1.vertices.contains(id)).count(),
            "edges": source.1.edges.iter().filter(|id|result.1.edges.contains(id)).count(),
            "faces": source.1.faces.iter().filter(|id|result.1.faces.contains(id)).count(),
            "bodies": source.1.bodies.iter().filter(|id|result.1.bodies.contains(id)).count(),
        });
        rows.push(value_codec::json!({"selectedEdge":edge,"retainedCounts":counts,
            "exactUnchangedVertices":unchanged_vertices,"namingComplete":result.persistent_naming_complete(),
            "changeSetValid":result.1.change_set.validate().is_ok(),"changeSet":result.1.change_set}));
    }
    let report = value_codec::json!({"schema":"cad-partial-annular-source-identities/1",
        "scope":"Observed identity retention on four source quarter-arcs; no complete source ownership certificate.","cases":rows});
    std::fs::write(
        format!("{directory}/report.json"),
        value_codec::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("Recorded identity ownership for {} source arcs", rows.len());
}
