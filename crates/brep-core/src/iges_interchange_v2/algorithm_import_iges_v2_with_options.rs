//! iges interchange v2 algorithm: import_iges_v2_with_options.
use super::*;
pub fn import_iges_v2_with_options(
    text: &str,
    options: IgesV2ImportOptions,
) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    let (mut entities, declared_unit, resolution) = parse(text)?;
    let entity_count = entities.len();
    let discarded = filter_discarded_entities(&mut entities)?;
    let applied_unit = options.unit_override.unwrap_or(declared_unit);
    let scale = applied_unit.scale_to_mm();
    let merge_tolerance_mm = resolution * scale;
    if !merge_tolerance_mm.is_finite() || merge_tolerance_mm <= 0. {
        return Err(refuse("Invalid scaled IGES composite merge tolerance"));
    }
    const GEOMETRIC: &[usize] = &[
        100, 102, 110, 124, 126, 128, 141, 142, 143, 186, 502, 504, 508, 510, 514,
    ];
    const METADATA: &[usize] = &[314, 402, 406];
    let mut ignored = BTreeSet::new();
    for e in entities.values() {
        if !GEOMETRIC.contains(&e.directory.ty) && !METADATA.contains(&e.directory.ty) {
            return Err(refuse(format!(
                "IGES entity {} is outside the finite direct subset",
                e.directory.ty
            )));
        }
        if METADATA.contains(&e.directory.ty) {
            ignored.insert(format!("entity-{}", e.directory.ty));
        }
        if e.directory.color != 0 {
            ignored.insert("directory-color".into());
        }
        if !e.directory.label.is_empty() {
            ignored.insert("directory-label".into());
        }
        if e.directory.transform != 0 {
            return Err(refuse(
                "Per-entity IGES transformations are typed-refused; bake one rigid placement into geometry",
            ));
        }
    }
    let builder = Builder {
        e: &entities,
        scale,
        merge_tolerance_mm,
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        vertex_de: vec![],
        edge_de: vec![],
        loop_de: vec![],
        face_de: vec![],
        shell_de: vec![],
        body_de: vec![],
    };
    let (mut model, map) = builder.build()?;
    let identity = restore_identity(&entities, &map, &mut model)?;
    let topology_count = map.len();
    let warning = unit_mismatch_warning(&model, declared_unit, applied_unit);
    Ok((
        model,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "strict_80_column_sgdp_t",
                "direct_126_128_502_504_508_510_514_186",
                "shared_topology_and_pcurves",
                "multi_body_and_cavity",
                "no_aabb_constructor_or_mesh",
            ],
        },
        IgesV2Report {
            identity,
            ignored_metadata: ignored.into_iter().collect(),
            entity_count,
            topology_count,
            units: IgesV2UnitReport {
                declared_unit,
                applied_unit,
                declared_flag: declared_unit.flag(),
                declared_scale_to_mm: declared_unit.scale_to_mm(),
                applied_scale_to_mm: applied_unit.scale_to_mm(),
                overridden: applied_unit != declared_unit,
                warning,
            },
            discarded,
        },
    ))
}
