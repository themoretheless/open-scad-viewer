use raster_core::shaders::{
    DEEP_MESH_WGSL, EDGE_WGSL, GRID_WGSL, LINE_WGSL, MESH_MATCAP_WGSL, MESH_PBR_WGSL, MESH_TOON_WGSL,
    MESH_UNLIT_WGSL, MESH_WGSL, SELECTION_OVERLAY_WGSL,
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
        ("mesh_pbr", MESH_PBR_WGSL),
        ("mesh_matcap", MESH_MATCAP_WGSL),
        ("mesh_toon", MESH_TOON_WGSL),
        ("mesh_unlit", MESH_UNLIT_WGSL),
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
    validate("instanced_mesh_pbr", &instanced_object_shader(MESH_PBR_WGSL, VertexOutput::V));
    validate("instanced_mesh_matcap", &instanced_object_shader(MESH_MATCAP_WGSL, VertexOutput::V));
    validate("instanced_mesh_toon", &instanced_object_shader(MESH_TOON_WGSL, VertexOutput::V));
    validate("instanced_mesh_unlit", &instanced_object_shader(MESH_UNLIT_WGSL, VertexOutput::V));
    validate("instanced_deep_mesh", &instanced_object_shader(DEEP_MESH_WGSL, VertexOutput::V));
    validate("instanced_edge", &instanced_object_shader(EDGE_WGSL, VertexOutput::EdgeV));
}

#[test]
fn uniform_layout_matches_the_wgsl_contract() {
    assert_eq!(OBJECT_UNIFORM_FLOATS, 56);
    assert_eq!(OBJECT_UNIFORM_BYTES, 224);
    assert_eq!(STYLE_FLOAT_OFFSET, 36);
    assert_eq!(MORPH_FLOAT_OFFSET, 40);
    assert_eq!(SCENE_UNIFORM_FLOATS, 72);
    assert_eq!(SCENE_UNIFORM_BYTES, 288);

    let object = ObjectUniform {
        model: [1.0; 16],
        nmat: [2.0; 16],
        color: [3.0; 4],
        style: [4.0; 4],
        morph: [5.0; 4],
        base_color: [6.0; 3],
        metallic: 0.25,
        emissive: [7.0; 3],
        roughness: 0.5,
        material_id: 3.0,
    };
    let mut floats = [0.0f32; 56];
    object.write_f32(&mut floats);
    assert_eq!(floats[0], 1.0);
    assert_eq!(floats[16], 2.0);
    assert_eq!(floats[32], 3.0);
    assert_eq!(floats[STYLE_FLOAT_OFFSET], 4.0);
    assert_eq!(floats[MORPH_FLOAT_OFFSET], 5.0);
    // Material tail starts at float 44; legacy offsets above are unchanged.
    assert_eq!(floats[44], 6.0);
    assert_eq!(floats[46], 6.0);
    assert_eq!(floats[47], 0.25); // metallic
    assert_eq!(floats[48], 7.0);
    assert_eq!(floats[50], 7.0);
    assert_eq!(floats[51], 0.5); // roughness
    assert_eq!(floats[52], 3.0); // material_id
    assert_eq!(floats[55], 0.0); // padding

    let default_object = ObjectUniform::default();
    assert_eq!(default_object.base_color, [1.0, 1.0, 1.0]);
    assert_eq!(default_object.metallic, 0.0);
    assert_eq!(default_object.emissive, [0.0, 0.0, 0.0]);
    assert_eq!(default_object.roughness, 0.7);
    assert_eq!(default_object.material_id, 0.0);

    let scene = SceneUniform::new([9.0; 16], [8.0; 4]);
    let mut scene_floats = [0.0f32; 72];
    scene.write_f32(&mut scene_floats);
    assert_eq!(scene_floats[0], 9.0);
    assert_eq!(scene_floats[16], 8.0);
    assert_eq!(scene_floats[20], 0.55); // default light
    assert_eq!(scene_floats[36], 0.0); // inverse_vp starts at float 36
    // Theme tail starts at float 52; defaults reproduce the previously
    // hard-coded shader colors exactly.
    assert_eq!(scene_floats[52..55], [1.0, 0.52, 0.06]); // selection
    assert_eq!(scene_floats[56..59], [0.12, 0.78, 1.0]); // hover
    assert_eq!(scene_floats[60..63], [0.025, 0.03, 0.04]); // edge
    assert_eq!(scene_floats[64..67], [1.0, 0.42, 0.06]); // xray
    assert_eq!(scene_floats[68..71], [0.42, 0.42, 0.42]); // grid
    assert_eq!(scene_floats[55], 0.0); // padding
    assert_eq!(scene_floats[71], 0.0);
}

#[test]
fn immediate_variant_serves_style_from_immediate_address_space() {
    for source in [MESH_WGSL, MESH_PBR_WGSL, MESH_MATCAP_WGSL, MESH_TOON_WGSL, MESH_UNLIT_WGSL, DEEP_MESH_WGSL, EDGE_WGSL] {
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

#[test]
fn generated_ts_matches_the_wgsl_sources() {
    // The browser shader library must consume exactly these WGSL texts; run
    // the `wgsl_export` bin to regenerate after editing any .wgsl file.
    let path = raster_core::codegen::generated_ts_path();
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("generated sources missing at {}: {error}", path.display()));
    assert_eq!(
        on_disk,
        raster_core::codegen::generate_ts_sources(),
        "{} is stale; regenerate with the `wgsl_export` bin",
        path.display()
    );
}

#[test]
fn generated_variant_goldens_match_the_rust_variants() {
    // The golden module pins the TypeScript variants.ts to these exact
    // outputs of the Rust transforms.
    let path = raster_core::codegen::variant_goldens_path();
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("variant goldens missing at {}: {error}", path.display()));
    assert_eq!(
        on_disk,
        raster_core::codegen::generate_variant_goldens_ts(),
        "{} is stale; regenerate with the `wgsl_export` bin",
        path.display()
    );
}
