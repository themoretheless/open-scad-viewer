//! The CAD Rush frontend must preserve the complete declarative authoring graph.
use std::path::Path;

fn sources(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() { sources(&path, files); }
        else if matches!(path.extension().and_then(|s| s.to_str()), Some("r" | "mg")) {files.push(path);}
    }
}
#[test]
fn rush_and_modelgraph_headers_produce_identical_graphs_and_controls() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rush");
    let mut files = Vec::new();
    sources(&root, &mut files);
    files.sort();
    let mut successful = 0;
    assert!(!files.is_empty());
    for path in files {
        let source = std::fs::read_to_string(&path).unwrap();
        let body = source.strip_prefix("// @rush/1").or_else(||source.strip_prefix("// @modelgraph-text/1")).unwrap_or(&source);
        // Keep byte offsets identical so source maps must match too.
        let rush = format!("{:<21}{body}", "// @rush/1");
        let graph = format!("// @modelgraph-text/1{body}");
        let rush = modelgraph_text::compile(&rush);
        let graph = modelgraph_text::compile(&graph);
        match (rush, graph) {
            (Ok(rush), Ok(graph)) => { for (key,value) in rush.as_object().unwrap() { assert_eq!(Some(value), graph.get(key), "{} field {key}",path.display()); } successful += 1; }
            (Err(_), Err(_)) => (),
            _ => panic!("Rush/ModelGraph validity differs for {}",path.display()),
        }
    }
    assert!(successful >= 10, "Only {successful} models compiled");
    eprintln!("Identical authoring graphs, controls and source maps for {successful} models");
}

#[test]
fn named_parts_keep_ids_when_independent_parts_are_inserted_or_reordered() {
    let base = "// @rush/1\nparam size = 10mm range 1mm..20mm\nbody = box([size,size,size]).translate([1mm,0mm,0mm])\nshow body";
    let expected = modelgraph_text::compile_document(base).unwrap();
    let root = expected["root"].as_str().unwrap();
    assert!(root.starts_with('r'));
    for source in [base.replace("body =", "unrelated = sphere(2mm)\nbody ="), base.replace("show body", "unrelated = sphere(2mm)\nshow body")] {
        let changed = modelgraph_text::compile_document(&source).unwrap();
        assert_eq!(changed["root"], expected["root"]);
        for node in expected["nodes"].as_array().unwrap() {
            let id = node["id"].as_str().unwrap();
            assert_eq!(Some(node), changed["nodes"].as_array().unwrap().iter().find(|n|n["id"].as_str()==Some(id)));
        }
    }
    let legacy = modelgraph_text::compile_document(&base.replace("// @rush/1","// @modelgraph-text/1")).unwrap();
    assert!(legacy["root"].as_str().unwrap().starts_with('n'));
}

#[test]
fn parameter_source_edit_preserves_comments_units_and_existing_part_ids() {
    let source = "// @rush/1\n// size comment\nparam size = 10mm range 1mm..20mm // retained\nbody = box([size,size,size])\nshow body";
    let before = modelgraph_text::compile_document(source).unwrap();
    let parameter = &before["customizer"][0];
    let start = parameter["valueStart"].as_u64().unwrap() as usize;
    let end = parameter["valueEnd"].as_u64().unwrap() as usize;
    let mut edited = source.to_owned();
    edited.replace_range(start..end, "12");
    assert!(edited.contains("12mm range 1mm..20mm // retained"));
    assert!(edited.contains("// size comment"));
    let after = modelgraph_text::compile_document(&edited).unwrap();
    assert_eq!(after["root"], before["root"]);
    assert_eq!(after["customizer"][0]["value"].as_f64(),Some(12.));
}

#[test]
fn explicit_ids_survive_binding_rename_and_reject_duplicate_identity() {
    let source = "// @rush/1\nbody @id(\"stable-body\") = box([10mm,10mm,10mm])\nshow body";
    let before = modelgraph_text::compile_document(source).unwrap();
    let renamed = modelgraph_text::compile_document(&source.replace("body @", "part @").replace("show body", "show part")).unwrap();
    assert_eq!(before["root"],renamed["root"]);
    assert_eq!(before["nodes"],renamed["nodes"]);
    assert!(modelgraph_text::compile_document(&source.replace("show body","other @id(\"stable-body\") = sphere(1mm)\nshow body")).is_err());
    for id in ["", "bad/id", "white space"] {
        assert!(modelgraph_text::compile_document(&source.replace("stable-body",id)).is_err());
    }
}

#[test]
fn declarative_example_combines_editable_inputs_procedural_function_and_output() {
    let source=include_str!("../../../examples/rush/declarative-identity.r");
    let document=modelgraph_text::compile_document(source).unwrap();
    assert_eq!(document["customizer"].as_array().unwrap().len(),2);
    assert!(document["root"].as_str().unwrap().starts_with('r'));
}
