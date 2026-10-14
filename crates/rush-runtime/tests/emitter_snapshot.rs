//! Exact (bit-for-bit) snapshot of compiler output for every emitter-parity
//! fixture plus extra documents covering node kinds the parity fixture lacks
//! (mechanical generators, evaluate, assert, match, error paths).
//! Each case stores the FNV-1a 64 hash and byte length of the serialized
//! result or error. Regenerate with `UPDATE_EMITTER_SNAPSHOT=1`.
use value_codec::{Value, json};

fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn doc(root: &str, nodes: Value) -> Value {
    json!({"language":"modelgraph/1","units":"mm","parameters":[{"id":"n","value":3},{"id":"teeth","value":14}],"root":root,"nodes":nodes})
}

fn extra_cases() -> Vec<(String, Value)> {
    let geometry_fn = json!({"id":"shape_fn","kind":"geometry","parameters":["r"],"root":"inner","nodes":[{"id":"inner","op":"sphere","radius":{"local":"r"}}]});
    let mut evaluate = doc(
        "ev",
        json!([{"id":"ev","op":"evaluate","value":{"op":"geometry","function":"shape_fn","args":{"r":2}}}]),
    );
    evaluate["functions"] = json!([geometry_fn]);
    vec![
        (
            "gear".into(),
            doc(
                "g",
                json!([{"id":"g","op":"gear","teeth":12,"module":1,"thickness":5}]),
            ),
        ),
        (
            "gear_nested".into(),
            doc(
                "t",
                json!([{"id":"t","op":"translate","vector":[1,0,0],"input":"g"},{"id":"g","op":"gear","teeth":{"param":"teeth"},"module":1,"thickness":5,"helix_angle":10}]),
            ),
        ),
        (
            "thread".into(),
            doc(
                "t",
                json!([{"id":"t","op":"thread","diameter":8,"pitch":1.25,"length":6}]),
            ),
        ),
        (
            "planetary_gears".into(),
            doc(
                "p",
                json!([{"id":"p","op":"planetary_gears","sun_teeth":24,"planet_teeth":24,"planet_count":3,"module":2,"thickness":8}]),
            ),
        ),
        (
            "planetary_gears_nested".into(),
            doc(
                "u",
                json!([{"id":"u","op":"union","inputs":["p"]},{"id":"p","op":"planetary_gears","sun_teeth":24,"planet_teeth":24,"planet_count":3,"module":2,"thickness":8}]),
            ),
        ),
        (
            "planetary_spinner".into(),
            doc(
                "s",
                json!([{"id":"s","op":"planetary_spinner","inner_radius":30.845,"outer_radius":40,"bore":8,"gap":0.3,"height":10,"helix_angle":30}]),
            ),
        ),
        (
            "planetary_spinner_bad".into(),
            doc(
                "s",
                json!([{"id":"s","op":"planetary_spinner","inner_radius":30.845,"outer_radius":20,"bore":8,"gap":0.3,"height":10,"helix_angle":30}]),
            ),
        ),
        ("evaluate".into(), evaluate),
        (
            "assert".into(),
            doc(
                "a",
                json!([{"id":"a","op":"assert","input":"b","constraints":[]},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
        (
            "match".into(),
            doc(
                "m",
                json!([{"id":"m","op":"match","value":{"param":"n"},"arms":[{"pattern":{"kind":"literal","value":3},"input":"b"},{"pattern":{"kind":"wildcard"},"input":"c"}]},{"id":"b","op":"box","size":[1,2,3]},{"id":"c","op":"sphere","radius":1}]),
            ),
        ),
        (
            "profile_mismatch".into(),
            doc(
                "e",
                json!([{"id":"e","op":"extrude","height":2,"input":"b"},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
        (
            "bad_box".into(),
            doc("b", json!([{"id":"b","op":"box","size":[1,0,3]}])),
        ),
        (
            "bad_circle".into(),
            doc(
                "e",
                json!([{"id":"e","op":"extrude","height":2,"input":"c"},{"id":"c","op":"circle","radius":-1}]),
            ),
        ),
        (
            "bad_cylinder_h".into(),
            doc(
                "c",
                json!([{"id":"c","op":"cylinder","radius":1,"height":0}]),
            ),
        ),
        (
            "bad_scale".into(),
            doc(
                "s",
                json!([{"id":"s","op":"scale","vector":[1,0,1],"input":"b"},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
        (
            "bad_mirror".into(),
            doc(
                "s",
                json!([{"id":"s","op":"mirror","normal":[0,0,0],"input":"b"},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
        (
            "bad_map".into(),
            doc(
                "s",
                json!([{"id":"s","op":"map","count":0,"index":"i","input":"b"},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
        (
            "bad_torus".into(),
            doc(
                "t",
                json!([{"id":"t","op":"torus","major_radius":1,"minor_radius":2}]),
            ),
        ),
        (
            "bad_cone".into(),
            doc(
                "t",
                json!([{"id":"t","op":"cone","radius_bottom":0,"radius_top":0,"height":2}]),
            ),
        ),
        (
            "bad_advanced_extrude".into(),
            doc(
                "e",
                json!([{"id":"e","op":"advanced_extrude","height":2,"twist":4000,"top_scale":[1,1],"slices":4,"input":"c"},{"id":"c","op":"circle","radius":1}]),
            ),
        ),
        (
            "bad_revolve".into(),
            doc(
                "e",
                json!([{"id":"e","op":"revolve","angle":400,"input":"c"},{"id":"c","op":"circle","radius":1}]),
            ),
        ),
        (
            "bad_pattern".into(),
            doc(
                "p",
                json!([{"id":"p","op":"linear_pattern","count":2,"step":[1e7,0,0],"input":"b"},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
        (
            "bad_affine".into(),
            doc(
                "a",
                json!([{"id":"a","op":"affine","rows":[[0,0,0,0],[0,1,0,0],[0,0,1,0]],"input":"b"},{"id":"b","op":"box","size":[1,2,3]}]),
            ),
        ),
    ]
}

#[test]
fn emitter_output_is_bit_identical_to_snapshot() {
    let cases: Value = value_codec::from_str(include_str!("fixtures/emitter-parity.json")).unwrap();
    let mut all: Vec<(String, Value)> = cases
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                format!("parity/{}", c["name"].as_str().unwrap()),
                c["document"].clone(),
            )
        })
        .collect();
    all.extend(
        extra_cases()
            .into_iter()
            .map(|(n, d)| (format!("extra/{n}"), d)),
    );
    let mut actual = Vec::new();
    for (name, document) in all {
        let out = match rush_runtime::compile(document) {
            Ok(result) => format!("ok:{}", value_codec::to_string(&result).unwrap()),
            Err(error) => format!("err:{}", value_codec::to_string(&error).unwrap()),
        };
        let kind = &out[..out.find(':').unwrap()];
        actual.push(format!(
            "{name}\t{kind}\t{}\t{}",
            out.len(),
            fnv(out.as_bytes())
        ));
    }
    let actual = actual.join("\n") + "\n";
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/emitter-snapshot.tsv"
    );
    if std::env::var_os("UPDATE_EMITTER_SNAPSHOT").is_some() {
        std::fs::write(path, &actual).unwrap();
    }
    // Emitted geometry goes through platform libm trig, whose last-bit results differ between
    // glibc, macOS and MSVC. The snapshot was recorded on x86_64 Linux (the CI `rust` job), so
    // only that target compares it; elsewhere every case still runs and must not panic.
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let expected = std::fs::read_to_string(path)
        .expect("snapshot missing; run with UPDATE_EMITTER_SNAPSHOT=1");
    for (a, e) in actual.lines().zip(expected.lines()) {
        assert_eq!(a, e, "emitter snapshot mismatch");
    }
    assert_eq!(actual.lines().count(), expected.lines().count());
}
