//! STEP assembly definitions, occurrences, and presentation metadata.
use super::*;

/// Export one planet prototype and its placements as an AP242 product assembly.
pub fn export_planet_gear_occurrences_step(
    assembly: &brep_core::gear::PlanetGearOccurrences,
) -> Result<String> {
    let placements: Vec<_> = assembly.placements().iter().copied().map(|matrix| (0, matrix)).collect();
    let (text, _, _) = export_step_direct_impl(
        assembly.prototype(), true, true, STEP_INTERCHANGE_V9_CAPABILITY,
        None, Some(AssemblyExport { placements: &placements, groups: None, colours: None, names: None, definition_tolerances: &[assembly.prototype().tolerance_mm] }), false,
    )?;
    Ok(text)
}

/// Export a planetary gear assembly with separate sun, ring and planet
/// definitions. All planet occurrences reference the same STEP geometry.
pub fn export_planetary_gear_set_step(
    sun: &Model,
    ring: &Model,
    planets: &brep_core::gear::PlanetGearOccurrences,
) -> Result<String> {
    if planets.placements().len() > 254 {
        return Err(refuse("Planetary assembly exceeds the STEP limit of 256 occurrences"));
    }
    let identity = [
        [1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.],
    ];
    let placed = |definition, transform| crate::StepAssemblyOccurrence { definition, transform };
    let mut placements = vec![placed(0, identity), placed(1, identity)];
    placements.extend(planets.placements().iter().map(|&transform| placed(2, transform)));
    let assembly = crate::StepSharedAssembly::from_placed_solids(
        vec![sun.clone(), ring.clone(), planets.prototype().clone()], placements,
    )?;
    export_step_shared_assembly(&assembly)
}

/// Serialize current shared definitions with their nonsingular orthogonal placements.
pub fn export_step_shared_assembly(assembly: &crate::StepSharedAssembly) -> Result<String> {
    export_shared_products(assembly, None, None, None)
}

/// Name the assembly and each reusable part definition in definition-index order.
pub fn export_named_step_assembly(assembly: &crate::StepSharedAssembly, name: &str, parts: &[&str]) -> Result<String> {
    validate_part_names(assembly, name, parts)?;
    export_shared_products(assembly, Some((name, parts)), None, None)
}

fn validate_part_names(assembly: &crate::StepSharedAssembly, name: &str, parts: &[&str]) -> Result<()> {
    if parts.len() != assembly.definitions().len() || std::iter::once(name).chain(parts.iter().copied())
        .any(|s| s.is_empty() || s.chars().any(char::is_control)) {
        return Err(refuse("Expected an assembly name and one nonempty printable name per definition"));
    }
    Ok(())
}

/// Export a flat group hierarchy while retaining reusable local solid definitions.
pub fn export_grouped_step_assembly(
    assembly: &crate::StepSharedAssembly,
    groups: &[&str],
    parents: &[Option<usize>],
) -> Result<String> {
    export_grouped_with_styles(assembly, None, groups, parents, None)
}

/// Write occurrence-specific surface colours while sharing the underlying solid.
pub fn export_styled_step_assembly(
    assembly: &crate::StepSharedAssembly, groups: &[&str], parents: &[Option<usize>],
    colours: &[Option<StepOccurrenceColour<'_>>],
) -> Result<String> {
    export_grouped_with_styles(assembly, None, groups, parents, Some(colours))
}

/// Export product labels, one level of groups, and occurrence-specific colours
/// together. Scaled geometry variants retain the label of their source product.
pub fn export_named_styled_step_assembly(
    assembly: &crate::NamedStepAssembly, groups: &[&str], parents: &[Option<usize>],
    colours: &[Option<StepOccurrenceColour<'_>>],
) -> Result<String> {
    let parts: Vec<_> = assembly.definition_names.iter().map(String::as_str).collect();
    validate_part_names(&assembly.assembly, &assembly.name, &parts)?;
    export_grouped_with_styles(&assembly.assembly, Some((&assembly.name, &parts)), groups, parents, Some(colours))
}

fn export_grouped_with_styles(
    assembly: &crate::StepSharedAssembly, names: Option<(&str, &[&str])>, groups: &[&str], parents: &[Option<usize>],
    colours: Option<&[Option<StepOccurrenceColour<'_>>]>,
) -> Result<String> {
    if let Some(colours) = colours
        && (colours.len() != parents.len() || colours.iter().flatten().any(|style|
            style.name.chars().any(char::is_control) || style.rgb.iter().any(|x| !(0.0..=1.0).contains(x)))) {
            return Err(refuse("Expected one optional named RGB style per occurrence"));
        }
    if parents.len() != assembly.occurrences().len()
        || groups.len() > 256
        || groups.iter().any(|name| name.is_empty() || name.chars().any(char::is_control))
        || parents.iter().flatten().any(|&i| i >= groups.len())
    {
        return Err(refuse("Expected printable group names and a valid optional parent for every occurrence"));
    }
    if colours.is_some() && assembly.occurrences().iter().any(|item|
        !crate::step_interchange_v3::is_proper_rigid_placement(item.transform)) {
        let (baked, sources) = assembly.with_baked_scales()?;
        let labels = names.map(|(_, parts)| sources.iter().map(|&i| parts[i]).collect::<Vec<_>>());
        let baked_names = names.zip(labels.as_deref()).map(|((name, _), parts)| (name, parts));
        return export_shared_products(&baked, baked_names, Some((groups, parents)), colours);
    }
    export_shared_products(assembly, names, Some((groups, parents)), colours)
}

fn export_shared_products(assembly: &crate::StepSharedAssembly, names: Option<(&str, &[&str])>, groups: Option<(&[&str], &[Option<usize>])>, colours: Option<&[Option<StepOccurrenceColour<'_>>]>) -> Result<String> {
    let definitions = assembly.definitions();
    let occurrences = assembly.occurrences();
    if definitions.is_empty() || occurrences.is_empty() || occurrences.len() > 256 {
        return Err(refuse("STEP shared assembly requires 1 to 256 occurrences"));
    }
    if definitions.iter().any(|model| {
        model.bodies.len() != 1 || model.shells.iter().any(|shell| !shell.closed)
    }) {
        return Err(refuse("STEP shared assembly export requires one closed solid per definition"));
    }
    let placements: Vec<_> = occurrences.iter()
        .map(|occurrence| (occurrence.definition, occurrence.transform))
        .collect();
    let model = crate::compose_step_v7_occurrences(definitions)?;
    let tolerances: Vec<_> = definitions.iter().map(|model| model.tolerance_mm).collect();
    let (text, _, _) = export_step_direct_impl(
        &model, true, true, STEP_INTERCHANGE_V9_CAPABILITY, None, Some(AssemblyExport { placements: &placements, groups, colours, names, definition_tolerances: &tolerances }), false,
    )?;
    Ok(text)
}
