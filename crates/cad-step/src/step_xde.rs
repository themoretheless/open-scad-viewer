//! Typed STEP XDE product attributes and validation-property diagnostics.
//! Density is normalized to kg/m³; validation area/volume use the kernel's
//! mm²/mm³ conventions regardless of the source Part 21 units.

use brep_core::analysis::mass_properties;
use crate::step_part21::{Entity, Value, call, components, list, number, one_ref, string_value};
use brep_core::{Model, TopoKind};
use nurbs_core::Result;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepValidationProperties {
    pub volume_mm3: Option<f64>,
    pub surface_area_mm2: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepProductMetadata {
    pub entity: usize,
    pub id: String,
    pub name: String,
    pub description: String,
    pub shape_name: Option<String>,
    pub applied_names: Vec<String>,
    pub material: Option<String>,
    pub density_kg_per_m3: Option<f64>,
    pub validation: StepValidationProperties,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepXdeUnsupported {
    pub code: &'static str,
    pub entity: usize,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepXdeMetadata {
    pub products: BTreeMap<usize, StepProductMetadata>,
    pub body_products: Vec<Option<usize>>,
    pub product_parents: BTreeMap<usize, usize>,
    pub unsupported: Vec<StepXdeUnsupported>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepValidationKind {
    Volume,
    SurfaceArea,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepValidationDiagnostic {
    pub code: &'static str,
    pub product: usize,
    pub property: StepValidationKind,
    pub declared: f64,
    pub computed: f64,
    pub absolute_error: f64,
    pub relative_error: f64,
    pub tolerance: f64,
}

impl StepXdeMetadata {
    pub fn validate_for_model(&self, model: &Model) -> Result<()> {
        if self.body_products.len() != model.bodies.len() {
            return Err(super::refuse("XDE body/product mapping does not match the model"));
        }
        for product in self.products.values() {
            if product.name.len() > 4096
                || product.id.len() > 1024
                || product.material.as_ref().is_some_and(|name| name.len() > 4096)
                || product.density_kg_per_m3.is_some_and(|density| {
                    !density.is_finite() || density <= 0.
                })
                || [product.validation.volume_mm3, product.validation.surface_area_mm2]
                    .into_iter()
                    .flatten()
                    .any(|value| !value.is_finite() || value < 0.)
            {
                return Err(super::refuse("Invalid typed STEP XDE product metadata"));
            }
        }
        Ok(())
    }

    /// Compares authored validation properties with independently integrated
    /// mass properties.  A scoped single-product model is required: silently
    /// comparing an assembly total with a component property would be wrong.
    pub fn validation_diagnostics(
        &self,
        model: &Model,
        absolute_tolerance: f64,
        relative_tolerance: f64,
    ) -> Result<Vec<StepValidationDiagnostic>> {
        if !absolute_tolerance.is_finite()
            || absolute_tolerance < 0.
            || !relative_tolerance.is_finite()
            || relative_tolerance < 0.
        {
            return Err(super::refuse("Validation tolerances must be finite and nonnegative"));
        }
        if !self.products.values().any(|product| {
            product.validation.volume_mm3.is_some() || product.validation.surface_area_mm2.is_some()
        }) {
            return Ok(Vec::new());
        }
        if self.products.len() != 1 || model.bodies.is_empty() {
            return Err(super::refuse(
                "Validation diagnostics require one scoped XDE product and a non-empty model",
            ));
        }
        let (&product_id, product) = self.products.iter().next().unwrap();
        let computed = mass_properties(model, model.tolerance_mm, 200_000)?;
        let mut diagnostics = Vec::new();
        for (property, declared, computed) in [
            (
                StepValidationKind::Volume,
                product.validation.volume_mm3,
                computed.signed_volume_mm3.abs(),
            ),
            (
                StepValidationKind::SurfaceArea,
                product.validation.surface_area_mm2,
                computed.surface_area_mm2,
            ),
        ] {
            if let Some(declared) = declared {
                let absolute_error = (declared - computed).abs();
                let relative_error = absolute_error / computed.abs().max(f64::MIN_POSITIVE);
                let tolerance = absolute_tolerance.max(relative_tolerance * computed.abs());
                if absolute_error > tolerance {
                    diagnostics.push(StepValidationDiagnostic {
                        code: "STEP_XDE_VALIDATION_MISMATCH",
                        product: product_id,
                        property,
                        declared,
                        computed,
                        absolute_error,
                        relative_error,
                        tolerance,
                    });
                }
            }
        }
        Ok(diagnostics)
    }
}

/// Decodes the ISO 10303-21 escapes used by product/name attributes.  Unknown
/// control directives are refused instead of leaking a misleading display name.
pub fn decode_step_string(input: &str) -> Result<String> {
    let mut output = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('\\') {
        output.push_str(&rest[..start]);
        let escape = &rest[start..];
        if let Some(tail) = escape.strip_prefix("\\X2\\") {
            let end = tail.find("\\X0\\").ok_or_else(|| super::refuse("Unterminated STEP \\X2\\ escape"))?;
            let hex = &tail[..end];
            if hex.len() % 4 != 0 {
                return Err(super::refuse("Malformed STEP UTF-16 escape"));
            }
            let units = (0..hex.len())
                .step_by(4)
                .map(|offset| {
                    u16::from_str_radix(&hex[offset..offset + 4], 16)
                        .map_err(|_| super::refuse("Malformed STEP UTF-16 code unit"))
                })
                .collect::<Result<Vec<_>>>()?;
            for character in char::decode_utf16(units) {
                output.push(character.map_err(|_| super::refuse("Invalid STEP UTF-16 surrogate"))?);
            }
            rest = &tail[end + 4..];
        } else if let Some(tail) = escape.strip_prefix("\\X\\") {
            if tail.len() < 2 {
                return Err(super::refuse("Truncated STEP \\X\\ escape"));
            }
            let byte = u8::from_str_radix(&tail[..2], 16)
                .map_err(|_| super::refuse("Malformed STEP one-byte escape"))?;
            output.push(char::from(byte));
            rest = &tail[2..];
        } else if escape.starts_with("\\S\\") {
            let tail = &escape[3..];
            let character = tail.chars().next().ok_or_else(|| super::refuse("Truncated STEP \\S\\ escape"))?;
            output.push(character);
            rest = &tail[character.len_utf8()..];
        } else if escape.starts_with("\\N\\") || escape.starts_with("\\F\\") {
            rest = &escape[3..];
        } else {
            return Err(super::refuse("Unsupported STEP string control directive"));
        }
    }
    output.push_str(rest);
    Ok(output)
}

pub fn encode_step_string(input: &str) -> String {
    let mut output = String::new();
    let mut utf16 = Vec::new();
    let flush = |output: &mut String, units: &mut Vec<u16>| {
        if !units.is_empty() {
            output.push_str("\\X2\\");
            for unit in units.drain(..) {
                output.push_str(&format!("{unit:04X}"));
            }
            output.push_str("\\X0\\");
        }
    };
    for character in input.chars() {
        if character.is_ascii() {
            flush(&mut output, &mut utf16);
            if character == '\'' {
                output.push('\'');
            } else if !character.is_control() {
                output.push(character);
            }
        } else {
            utf16.extend(character.encode_utf16(&mut [0; 2]).iter().copied());
        }
    }
    flush(&mut output, &mut utf16);
    output
}

fn product_graph(
    entities: &BTreeMap<usize, Entity>,
) -> Result<(
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
)> {
    let mut definition_product = BTreeMap::new();
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PRODUCT_DEFINITION"
            && args.len() == 4
        {
            let formation = one_ref(&args[2], "product definition formation")?;
            let (_, args) = call(entities, formation)?;
            if args.len() >= 3 {
                definition_product.insert(id, one_ref(&args[2], "formation product")?);
            }
        }
    }
    let mut shape_product = BTreeMap::new();
    let mut body_product = BTreeMap::new();
    for entity in entities.values() {
        if let Value::Call(kind, args) = &entity.value
            && kind == "SHAPE_DEFINITION_REPRESENTATION"
            && args.len() == 2
        {
            let shape = one_ref(&args[0], "product definition shape")?;
            let (_, shape_args) = call(entities, shape)?;
            if shape_args.len() != 3 {
                continue;
            }
            let definition = one_ref(&shape_args[2], "shape definition")?;
            let Some(product) = definition_product.get(&definition).copied() else { continue };
            shape_product.insert(shape, product);
            let representation = one_ref(&args[1], "shape representation")?;
            let (_, rep_args) = call(entities, representation)?;
            if rep_args.len() == 3 {
                for item in list(&rep_args[1], "shape representation items")? {
                    if let Value::Ref(body) = item
                        && matches!(call(entities, *body)?.0, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS" | "SHELL_BASED_SURFACE_MODEL")
                    {
                        body_product.insert(*body, product);
                    }
                }
            }
        }
    }
    Ok((definition_product, shape_product, body_product))
}

fn prefix_scale(prefix: &str) -> Option<f64> {
    Some(match prefix {
        "" => 1.,
        "MILLI" => 1e-3,
        "CENTI" => 1e-2,
        "DECI" => 1e-1,
        "KILO" => 1e3,
        "MEGA" => 1e6,
        "MICRO" => 1e-6,
        _ => return None,
    })
}

fn si_unit(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<Option<(f64, String)>> {
    for (kind, args) in components(entities, id)? {
        if kind == "SI_UNIT" && args.len() == 2 {
            let prefix = match &args[0] {
                Value::Enum(prefix) => prefix.clone(),
                Value::Omitted => String::new(),
                _ => return Err(super::refuse("Malformed SI unit prefix")),
            };
            let Value::Enum(name) = &args[1] else {
                return Err(super::refuse("Malformed SI unit name"));
            };
            let scale = prefix_scale(&prefix).ok_or_else(|| super::refuse("Unsupported SI prefix"))?;
            return Ok(Some((scale, name.clone())));
        }
    }
    Ok(None)
}

fn area_volume_factor(
    entities: &BTreeMap<usize, Entity>,
    unit: usize,
    power: i32,
) -> Result<Option<f64>> {
    let Some((scale, name)) = si_unit(entities, unit)? else { return Ok(None) };
    match name.as_str() {
        "METRE" => Ok(Some((scale * 1000.).powi(power))),
        "SQUARE_METRE" if power == 2 => Ok(Some((scale * 1000.).powi(2))),
        "CUBIC_METRE" if power == 3 => Ok(Some((scale * 1000.).powi(3))),
        _ => Ok(None),
    }
}

fn density_factor(entities: &BTreeMap<usize, Entity>, unit: usize) -> Result<Option<f64>> {
    for (kind, args) in components(entities, unit)? {
        if kind == "DERIVED_UNIT" && !args.is_empty() {
            let mut factor = 1.;
            let mut mass = false;
            let mut length_power = 0;
            for element in list(&args[0], "density derived unit elements")? {
                let element = one_ref(element, "derived unit element")?;
                let (_, args) = call(entities, element)?;
                if args.len() != 2 {
                    return Err(super::refuse("Malformed derived unit element"));
                }
                let base = one_ref(&args[0], "derived unit base")?;
                let exponent = number(&args[1], "derived unit exponent")?;
                let Some((scale, name)) = si_unit(entities, base)? else { return Ok(None) };
                match name.as_str() {
                    "GRAM" => {
                        factor *= (scale * 1e-3).powf(exponent);
                        mass = true;
                    }
                    "METRE" => {
                        factor *= scale.powf(exponent);
                        length_power = exponent as i32;
                    }
                    _ => return Ok(None),
                }
            }
            if mass && length_power == -3 {
                return Ok(Some(factor));
            }
        }
    }
    Ok(None)
}

fn measure_value(value: &Value) -> Result<Option<(&str, f64)>> {
    if let Value::Call(kind, args) = value
        && args.len() == 1
        && matches!(kind.as_str(), "VOLUME_MEASURE" | "AREA_MEASURE" | "LENGTH_MEASURE" | "DENSITY_MEASURE" | "MASS_MEASURE")
    {
        Ok(Some((kind, number(&args[0], "measure value")?)))
    } else {
        Ok(None)
    }
}

pub(crate) fn parse(
    entities: &BTreeMap<usize, Entity>,
    mapping: &[(TopoKind, usize, usize)],
) -> Result<StepXdeMetadata> {
    let body_indices: BTreeMap<usize, usize> = mapping
        .iter()
        .filter_map(|(kind, index, entity)| (*kind == TopoKind::Body).then_some((*entity, *index)))
        .collect();
    let (definition_product, shape_product, source_body_product) = product_graph(entities)?;
    let mut result = StepXdeMetadata {
        body_products: vec![None; body_indices.len()],
        ..StepXdeMetadata::default()
    };
    for (source, target) in body_indices {
        result.body_products[target] = source_body_product.get(&source).copied();
    }
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PRODUCT"
            && args.len() == 4
        {
            result.products.insert(id, StepProductMetadata {
                entity: id,
                id: decode_step_string(string_value(&args[0], "product id")?)?,
                name: decode_step_string(string_value(&args[1], "product name")?)?,
                description: decode_step_string(string_value(&args[2], "product description")?)?,
                ..StepProductMetadata::default()
            });
        }
    }
    for (&shape, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PRODUCT_DEFINITION_SHAPE"
            && args.len() == 3
        {
            if let Some(product) = shape_product.get(&shape).and_then(|id| result.products.get_mut(id)) {
                let name = decode_step_string(string_value(&args[0], "shape name")?)?;
                if !name.is_empty() {
                    product.shape_name = Some(name);
                }
            }
        }
    }
    for entity in entities.values() {
        if let Value::Call(kind, args) = &entity.value
            && kind == "NEXT_ASSEMBLY_USAGE_OCCURRENCE"
            && args.len() >= 5
        {
            let parent = definition_product.get(&one_ref(&args[3], "parent definition")?).copied();
            let child = definition_product.get(&one_ref(&args[4], "child definition")?).copied();
            if let (Some(parent), Some(child)) = (parent, child) {
                result.product_parents.insert(child, parent);
            }
        }
    }
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "MATERIAL_DESIGNATION"
            && args.len() == 2
        {
            let material = decode_step_string(string_value(&args[0], "material designation")?)?;
            let definitions: Vec<usize> = match &args[1] {
                Value::Ref(id) => vec![*id],
                Value::List(items) => items.iter().map(|item| one_ref(item, "material definition")).collect::<Result<_>>()?,
                _ => return Err(super::refuse("Malformed material designation target")),
            };
            let mut assigned = false;
            for definition in definitions {
                if let Some(product) = definition_product
                    .get(&definition)
                    .or_else(|| shape_product.get(&definition))
                    .copied()
                    .or(Some(definition).filter(|id| result.products.contains_key(id))) {
                    if let Some(metadata) = result.products.get_mut(&product) {
                        metadata.material = Some(material.clone());
                        assigned = true;
                    }
                }
            }
            if !assigned {
                result.unsupported.push(StepXdeUnsupported {
                    code: "STEP_XDE_MATERIAL_TARGET_UNSUPPORTED",
                    entity: id,
                    detail: "Material designation is not attached to a retained product".into(),
                });
            }
        }
    }
    let mut properties = BTreeMap::<usize, (String, usize)>::new();
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PROPERTY_DEFINITION"
            && args.len() == 3
        {
            let name = decode_step_string(string_value(&args[0], "property name")?)?.to_ascii_lowercase();
            let definition = one_ref(&args[2], "property definition target")?;
            properties.insert(id, (name, definition));
        }
    }
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PROPERTY_DEFINITION_REPRESENTATION"
            && args.len() == 2
        {
            let property = one_ref(&args[0], "represented property")?;
            let Some((property_name, definition)) = properties.get(&property) else { continue };
            let Some(product) = definition_product
                .get(definition)
                .or_else(|| shape_product.get(definition))
                .copied()
                .or(Some(*definition).filter(|id| result.products.contains_key(id))) else {
                result.unsupported.push(StepXdeUnsupported {
                    code: "STEP_XDE_PROPERTY_TARGET_UNSUPPORTED",
                    entity: id,
                    detail: format!("Property #{property} has no retained product target"),
                });
                continue;
            };
            let representation = one_ref(&args[1], "property representation")?;
            let (_, rep_args) = call(entities, representation)?;
            if rep_args.len() != 3 {
                return Err(super::refuse("Malformed property representation"));
            }
            for item in list(&rep_args[1], "property representation items")? {
                let item = one_ref(item, "property representation item")?;
                let (item_kind, item_args) = call(entities, item)?;
                match item_kind {
                    "DESCRIPTIVE_REPRESENTATION_ITEM" if item_args.len() >= 2 && property_name.contains("material") => {
                        if let Some(metadata) = result.products.get_mut(&product) {
                            metadata.material = Some(decode_step_string(string_value(&item_args[1], "material description")?)?);
                        }
                    }
                    "MEASURE_REPRESENTATION_ITEM" if item_args.len() == 3 => {
                        let item_name = decode_step_string(string_value(&item_args[0], "measure item name")?)?.to_ascii_lowercase();
                        let Some((measure_kind, value)) = measure_value(&item_args[1])? else {
                            result.unsupported.push(StepXdeUnsupported {
                                code: "STEP_XDE_MEASURE_UNSUPPORTED",
                                entity: item,
                                detail: "Only typed area, volume, and density measures are retained".into(),
                            });
                            continue;
                        };
                        let unit = one_ref(&item_args[2], "measure unit")?;
                        let normalized_name = format!("{property_name} {item_name}");
                        let is_volume = measure_kind == "VOLUME_MEASURE" || normalized_name.contains("volume");
                        let is_area = measure_kind == "AREA_MEASURE" || normalized_name.contains("area");
                        let is_density = measure_kind == "DENSITY_MEASURE" || normalized_name.contains("density");
                        let normalized = if is_density {
                            density_factor(entities, unit)?.map(|factor| value * factor)
                        } else if is_volume {
                            area_volume_factor(entities, unit, 3)?.map(|factor| value * factor)
                        } else if is_area {
                            area_volume_factor(entities, unit, 2)?.map(|factor| value * factor)
                        } else {
                            None
                        };
                        if let Some(value) = normalized {
                            if let Some(metadata) = result.products.get_mut(&product) {
                                if is_density {
                                    metadata.density_kg_per_m3 = Some(value);
                                } else if is_volume {
                                    metadata.validation.volume_mm3 = Some(value);
                                } else {
                                    metadata.validation.surface_area_mm2 = Some(value);
                                }
                            }
                        } else {
                            result.unsupported.push(StepXdeUnsupported {
                                code: "STEP_XDE_UNIT_UNSUPPORTED",
                                entity: item,
                                detail: format!("No typed conversion for {measure_kind} unit #{unit}"),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    for entity in entities.values() {
        if let Value::Call(kind, args) = &entity.value
            && kind == "APPLIED_NAME_ASSIGNMENT"
            && args.len() == 3
        {
            let assigned_name = match &args[0] {
                Value::String(name) => decode_step_string(name)?,
                Value::Ref(attribute) => {
                    let (_, args) = call(entities, *attribute)?;
                    let value = args.iter().find_map(|arg| match arg { Value::String(value) => Some(value), _ => None })
                        .ok_or_else(|| super::refuse("Name attribute has no string"))?;
                    decode_step_string(value)?
                }
                _ => return Err(super::refuse("Malformed applied name assignment")),
            };
            for target in list(&args[2], "applied name targets")? {
                let target = one_ref(target, "applied name target")?;
                let product = definition_product
                    .get(&target)
                    .or_else(|| shape_product.get(&target))
                    .copied()
                    .or(Some(target).filter(|id| result.products.contains_key(id)));
                if let Some(metadata) = product.and_then(|id| result.products.get_mut(&id)) {
                    if !metadata.applied_names.contains(&assigned_name) {
                        metadata.applied_names.push(assigned_name.clone());
                    }
                }
            }
        }
    }
    Ok(result)
}

fn quote(value: &str) -> String {
    format!("'{}'", encode_step_string(value).replace('\'', "''"))
}

fn real(value: f64) -> String {
    format!("{value:.17e}").replace('e', "E")
}

pub(crate) fn emit(
    writer: &mut crate::step_v3_export::Writer,
    xde: &StepXdeMetadata,
    definition: usize,
    context: usize,
) -> Result<()> {
    if xde.products.len() > 1 {
        return Err(super::refuse(
            "Typed semantic export currently supports one XDE product; use /10 for retained assemblies",
        ));
    }
    let Some(product) = xde.products.values().next() else { return Ok(()) };
    if let Some(material) = &product.material {
        writer.emit(format!("MATERIAL_DESIGNATION({},#{definition})", quote(material)));
    }
    if let Some(density) = product.density_kg_per_m3 {
        let kg = writer.emit("(MASS_UNIT()NAMED_UNIT(*)SI_UNIT(.KILO.,.GRAM.))".into());
        let metre = writer.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.))".into());
        let mass_element = writer.emit(format!("DERIVED_UNIT_ELEMENT(#{kg},1.)"));
        let length_element = writer.emit(format!("DERIVED_UNIT_ELEMENT(#{metre},-3.)"));
        let unit = writer.emit(format!(
            "(DENSITY_UNIT()DERIVED_UNIT((#{mass_element},#{length_element}))NAMED_UNIT(*))"
        ));
        let measure = writer.emit(format!(
            "MEASURE_REPRESENTATION_ITEM('density',DENSITY_MEASURE({}),#{unit})",
            real(density)
        ));
        let representation = writer.emit(format!("REPRESENTATION('',(#{measure}),#{context})"));
        let property = writer.emit(format!("PROPERTY_DEFINITION('density','',#{definition})"));
        writer.emit(format!("PROPERTY_DEFINITION_REPRESENTATION(#{property},#{representation})"));
    }
    let mut measures = Vec::new();
    if let Some(volume) = product.validation.volume_mm3 {
        let unit = writer.emit(format!("(NAMED_UNIT(*)SI_UNIT($,.CUBIC_METRE.)VOLUME_UNIT())"));
        measures.push(writer.emit(format!(
            "MEASURE_REPRESENTATION_ITEM('volume',VOLUME_MEASURE({}),#{unit})",
            real(volume * 1e-9)
        )));
    }
    if let Some(area) = product.validation.surface_area_mm2 {
        let unit = writer.emit(format!("(NAMED_UNIT(*)SI_UNIT($,.SQUARE_METRE.)AREA_UNIT())"));
        measures.push(writer.emit(format!(
            "MEASURE_REPRESENTATION_ITEM('surface area',AREA_MEASURE({}),#{unit})",
            real(area * 1e-6)
        )));
    }
    if !measures.is_empty() {
        let items = measures.iter().map(|id| format!("#{id}")).collect::<Vec<_>>().join(",");
        let representation = writer.emit(format!("REPRESENTATION('',({items}),#{context})"));
        let property = writer.emit(format!("PROPERTY_DEFINITION('validation property','',#{definition})"));
        writer.emit(format!("PROPERTY_DEFINITION_REPRESENTATION(#{property},#{representation})"));
    }
    Ok(())
}
