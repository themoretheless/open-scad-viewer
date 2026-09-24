use raster_core::shaders::{
    DEEP_MESH_WGSL, EDGE_WGSL, GRID_WGSL, LINE_WGSL, MESH_WGSL, SELECTION_OVERLAY_WGSL,
};
use raster_core::uniform::{
    MORPH_FLOAT_OFFSET, OBJECT_UNIFORM_BYTES, OBJECT_UNIFORM_FLOATS, SCENE_UNIFORM_BYTES,
    SCENE_UNIFORM_FLOATS, STYLE_FLOAT_OFFSET, ObjectUniform, SceneUniform,
};
use raster_core::variants::{VertexOutput, immediate_object_shader, instanced_object_shader};

fn validate(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|error| panic!("{name}: WGSL parse failed: {error}"));
    let mut validator =
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all());
    validator
        .validate(&module)
        .unwrap_or_else(|error| panic!("{name}: WGSL validation failed: {error}"));
}

#[test]
fn shipped_shaders_validate_with_naga() {
    for (name, source) in [
        ("mesh", MESH_WGSL),
        ("deep_mesh", DEEP_MESH_WGSL),
        ("edge", EDGE_WGSL),
        ("line", LINE_WGSL),
        ("grid", GRID_WGSL),
        ("selection_overlay", SELECTION_OVERLAY_WGSL),
    ] {
        validate(name, source);
    }
}

#[test]
fn instanced_variants_validate_with_naga() {
    validate("instanced_mesh", &instanced_object_shader(MESH_WGSL, VertexOutput::V));
    validate("instanced_edge", &instanced_object_shader(EDGE_WGSL, VertexOutput::EdgeV));
}

#[test]
fn uniform_layout_matches_the_wgsl_contract() {
    assert_eq!(OBJECT_UNIFORM_FLOATS, 44);
    assert_eq!(OBJECT_UNIFORM_BYTES, 176);
    assert_eq!(STYLE_FLOAT_OFFSET, 36);
    assert_eq!(MORPH_FLOAT_OFFSET, 40);
    assert_eq!(SCENE_UNIFORM_FLOATS, 52);
    assert_eq!(SCENE_UNIFORM_BYTES, 208);

    let object = ObjectUniform {
        model: [1.0; 16],
        nmat: [2.0; 16],
        color: [3.0; 4],
        style: [4.0; 4],
        morph: [5.0; 4],
    };
    let mut floats = [0.0f32; 44];
    object.write_f32(&mut floats);
    assert_eq!(floats[0], 1.0);
    assert_eq!(floats[16], 2.0);
    assert_eq!(floats[32], 3.0);
    assert_eq!(floats[STYLE_FLOAT_OFFSET], 4.0);
    assert_eq!(floats[MORPH_FLOAT_OFFSET], 5.0);

    let scene = SceneUniform::new([9.0; 16], [8.0; 4]);
    let mut scene_floats = [0.0f32; 52];
    scene.write_f32(&mut scene_floats);
    assert_eq!(scene_floats[0], 9.0);
    assert_eq!(scene_floats[16], 8.0);
    assert_eq!(scene_floats[20], 0.55); // default light
    assert_eq!(scene_floats[36], 0.0); // inverse_vp starts at float 36
}

#[test]
fn immediate_variant_serves_style_from_immediate_address_space() {
    for source in [MESH_WGSL, DEEP_MESH_WGSL, EDGE_WGSL] {
        let variant = immediate_object_shader(source);
        assert!(variant.starts_with("requires immediate_address_space;"));
        assert!(variant.contains("var<immediate> im_style: vec4f;"));
        assert!(!variant.contains("ob.style"));
        assert!(variant.contains("objectStyle()"));
    }
}

#[test]
fn instanced_variant_reads_per_instance_records() {
    for (source, output) in [(MESH_WGSL, VertexOutput::V), (EDGE_WGSL, VertexOutput::EdgeV)] {
        let variant = instanced_object_shader(source, output);
        assert!(variant.contains("var<storage, read> objects: array<Obj>;"));
        assert!(variant.contains("@builtin(instance_index) instance: u32"));
        assert!(variant.contains("let ob = objects[instance];"));
        assert!(variant.contains("let ob = objects[v.instance];"));
        assert!(!variant.contains("var<uniform> ob: Obj;"));
    }
}

#[test]
#[should_panic(expected = "missing object uniform")]
fn immediate_variant_panics_without_object_uniform() {
    immediate_object_shader("fn nope() {}");
}

#[test]
#[should_panic(expected = "Instanced shader contract changed")]
fn instanced_variant_panics_on_drifted_source() {
    instanced_object_shader("fn nope() {}", VertexOutput::V);
}
