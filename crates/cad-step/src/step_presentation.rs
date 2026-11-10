//! AP214 presentation metadata retained alongside the direct B-rep projection.
//!
//! Styles are kept at their authored scope.  Consumers must use
//! [`StepPresentation::face_style`] rather than flattening body/product styles
//! into faces: doing so would lose the STEP face > body > product override rule.

use crate::step_part21::{Entity, Value, call, components, list, number, one_ref};
use brep_core::{Model, TopoKind};
use nurbs_core::Result;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepColour {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

impl StepColour {
    pub fn new(red: f64, green: f64, blue: f64) -> Result<Self> {
        let colour = Self { red, green, blue };
        if ![red, green, blue]
            .into_iter()
            .all(|channel| channel.is_finite() && (0. ..=1.).contains(&channel))
        {
            return Err(super::refuse("STEP RGB channels must be finite and in 0..=1"));
        }
        Ok(colour)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepPresentationStyle {
    pub colour: Option<StepColour>,
    pub layers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepPresentationUnsupported {
    pub code: &'static str,
    pub entity: usize,
    pub detail: String,
}

/// Presentation bound to the same stable topology indices as `Model`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepPresentation {
    pub face_styles: Vec<Option<StepPresentationStyle>>,
    pub body_styles: Vec<Option<StepPresentationStyle>>,
    pub product_styles: BTreeMap<usize, StepPresentationStyle>,
    pub body_products: Vec<Option<usize>>,
    pub product_parents: BTreeMap<usize, usize>,
    pub unsupported: Vec<StepPresentationUnsupported>,
}

impl StepPresentation {
    pub fn empty(model: &Model) -> Self {
        Self {
            face_styles: vec![None; model.faces.len()],
            body_styles: vec![None; model.bodies.len()],
            body_products: vec![None; model.bodies.len()],
            ..Self::default()
        }
    }

    pub fn body_style(&self, body: usize) -> Option<StepPresentationStyle> {
        let product = self.body_products.get(body).copied().flatten();
        let body_style = self.body_styles.get(body).and_then(|style| style.clone());
        let product_style = product.and_then(|id| self.inherited_product_style(id));
        let colour = body_style
            .as_ref()
            .and_then(|style| style.colour)
            .or_else(|| product_style.as_ref().and_then(|style| style.colour));
        let layers = [&body_style, &product_style]
            .into_iter()
            .flatten()
            .find(|style| !style.layers.is_empty())
            .map(|style| style.layers.clone())
            .unwrap_or_default();
        (colour.is_some() || !layers.is_empty()).then_some(StepPresentationStyle { colour, layers })
    }

    /// Resolves colour and layers independently, so a face colour override does
    /// not accidentally discard an authored body or product layer assignment.
    pub fn face_style(&self, face: usize, body: usize) -> Option<StepPresentationStyle> {
        let product = self.body_products.get(body).copied().flatten();
        let face = self.face_styles.get(face).and_then(|style| style.clone());
        let body = self.body_styles.get(body).and_then(|style| style.clone());
        let product = product.and_then(|id| self.inherited_product_style(id));
        let colour = face
            .as_ref()
            .and_then(|style| style.colour)
            .or_else(|| body.as_ref().and_then(|style| style.colour))
            .or_else(|| product.as_ref().and_then(|style| style.colour));
        let layers = [&face, &body, &product]
            .into_iter()
            .flatten()
            .find(|style| !style.layers.is_empty())
            .map(|style| style.layers.clone())
            .unwrap_or_default();
        (colour.is_some() || !layers.is_empty()).then_some(StepPresentationStyle { colour, layers })
    }

    fn inherited_product_style(&self, product: usize) -> Option<StepPresentationStyle> {
        let mut current = Some(product);
        let mut visited = BTreeSet::new();
        let mut colour = None;
        let mut layers = Vec::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                break;
            }
            if let Some(style) = self.product_styles.get(&id) {
                colour = colour.or(style.colour);
                if layers.is_empty() {
                    layers = style.layers.clone();
                }
            }
            current = self.product_parents.get(&id).copied();
        }
        (colour.is_some() || !layers.is_empty()).then_some(StepPresentationStyle { colour, layers })
    }

    pub fn validate_for_model(&self, model: &Model) -> Result<()> {
        if self.face_styles.len() != model.faces.len()
            || self.body_styles.len() != model.bodies.len()
            || self.body_products.len() != model.bodies.len()
        {
            return Err(super::refuse(
                "STEP presentation topology dimensions do not match the model",
            ));
        }
        for style in self
            .face_styles
            .iter()
            .chain(&self.body_styles)
            .flatten()
            .chain(self.product_styles.values())
        {
            if let Some(colour) = style.colour {
                StepColour::new(colour.red, colour.green, colour.blue)?;
            }
            if style.layers.len() > 256
                || style
                    .layers
                    .iter()
                    .any(|layer| layer.len() > 1024 || layer.contains(['\r', '\n']))
            {
                return Err(super::refuse("Invalid STEP presentation layer metadata"));
            }
        }
        Ok(())
    }
}

fn predefined_colour(name: &str) -> Option<StepColour> {
    let rgb = match name.to_ascii_lowercase().as_str() {
        "black" => [0., 0., 0.],
        "white" => [1., 1., 1.],
        "red" => [1., 0., 0.],
        "green" => [0., 1., 0.],
        "blue" => [0., 0., 1.],
        "yellow" => [1., 1., 0.],
        "magenta" => [1., 0., 1.],
        "cyan" => [0., 1., 1.],
        _ => return None,
    };
    Some(StepColour { red: rgb[0], green: rgb[1], blue: rgb[2] })
}

fn colour(entities: &BTreeMap<usize, Entity>, id: usize, depth: usize) -> Result<Option<StepColour>> {
    if depth > 16 {
        return Err(super::refuse("STEP presentation style chain exceeds 16 links"));
    }
    let (kind, args): (&str, &[Value]) = if matches!(entities[&id].value, Value::List(_)) {
        ("COMPLEX", &[])
    } else {
        call(entities, id)?
    };
    match kind {
        "COLOUR_RGB" if args.len() == 4 => Ok(Some(StepColour::new(
            number(&args[1], "red colour channel")?,
            number(&args[2], "green colour channel")?,
            number(&args[3], "blue colour channel")?,
        )?)),
        "DRAUGHTING_PRE_DEFINED_COLOUR" if args.len() == 1 => {
            if let Value::String(name) = &args[0] {
                Ok(predefined_colour(name))
            } else {
                Err(super::refuse("Malformed predefined STEP colour"))
            }
        }
        "FILL_AREA_STYLE_COLOUR" if args.len() == 2 => {
            colour(entities, one_ref(&args[1], "fill colour")?, depth + 1)
        }
        "FILL_AREA_STYLE" if args.len() == 2 => {
            for item in list(&args[1], "fill area styles")? {
                if let Some(colour) = colour(entities, one_ref(item, "fill style")?, depth + 1)? {
                    return Ok(Some(colour));
                }
            }
            Ok(None)
        }
        "SURFACE_STYLE_FILL_AREA" if args.len() == 1 => {
            colour(entities, one_ref(&args[0], "surface fill style")?, depth + 1)
        }
        "SURFACE_SIDE_STYLE" if args.len() == 2 => {
            for item in list(&args[1], "surface side styles")? {
                if let Some(colour) = colour(entities, one_ref(item, "side style")?, depth + 1)? {
                    return Ok(Some(colour));
                }
            }
            Ok(None)
        }
        "SURFACE_STYLE_USAGE" if args.len() == 2 => {
            colour(entities, one_ref(&args[1], "surface style usage")?, depth + 1)
        }
        "PRESENTATION_STYLE_BY_CONTEXT" if !args.is_empty() => {
            colour(entities, one_ref(&args[0], "contextual presentation style")?, depth + 1)
        }
        "COMPLEX" => {
            for (kind, args) in components(entities, id)? {
                match (kind, args) {
                    ("FILL_AREA_STYLE_COLOUR", [_, colour_ref]) => {
                        return colour(entities, one_ref(colour_ref, "complex fill colour")?, depth + 1);
                    }
                    ("COLOUR_RGB", [_, red, green, blue]) => {
                        return Ok(Some(StepColour::new(
                            number(red, "red colour channel")?,
                            number(green, "green colour channel")?,
                            number(blue, "blue colour channel")?,
                        )?));
                    }
                    ("SURFACE_STYLE_USAGE", [_, style]) | ("PRESENTATION_STYLE_BY_CONTEXT", [style, ..]) => {
                        return colour(entities, one_ref(style, "complex surface style")?, depth + 1);
                    }
                    ("SURFACE_STYLE_FILL_AREA", [style]) => {
                        return colour(entities, one_ref(style, "complex fill area")?, depth + 1);
                    }
                    ("FILL_AREA_STYLE", [_, Value::List(styles)]) | ("SURFACE_SIDE_STYLE", [_, Value::List(styles)]) => {
                        for style in styles {
                            if let Some(colour) = colour(entities, one_ref(style, "complex side style")?, depth + 1)? {
                                return Ok(Some(colour));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn styled_target(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<Option<usize>> {
    let (kind, args) = call(entities, id)?;
    if kind == "STYLED_ITEM" && args.len() == 3 {
        Ok(Some(one_ref(&args[2], "styled representation item")?))
    } else {
        Ok(None)
    }
}

fn representation_products(
    entities: &BTreeMap<usize, Entity>,
) -> Result<(BTreeMap<usize, usize>, BTreeMap<usize, usize>, BTreeMap<usize, usize>)> {
    let mut definition_product = BTreeMap::new();
    for entity in entities.values() {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PRODUCT_DEFINITION"
            && args.len() == 4
        {
            let formation = one_ref(&args[2], "product definition formation")?;
            let (_, formation_args) = call(entities, formation)?;
            if formation_args.len() >= 3 {
                definition_product.insert(entity_id(entities, entity), one_ref(&formation_args[2], "product")?);
            }
        }
    }
    let mut representation_product = BTreeMap::new();
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
            let definition = one_ref(&shape_args[2], "shape product definition")?;
            let Some(product) = definition_product.get(&definition).copied() else { continue };
            let representation = one_ref(&args[1], "shape representation")?;
            representation_product.insert(representation, product);
            let (_, rep_args) = call(entities, representation)?;
            if rep_args.len() == 3 {
                for item in list(&rep_args[1], "representation items")? {
                    if let Value::Ref(body) = item
                        && matches!(call(entities, *body)?.0, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS" | "SHELL_BASED_SURFACE_MODEL")
                    {
                        body_product.insert(*body, product);
                    }
                }
            }
        }
    }
    Ok((representation_product, body_product, definition_product))
}

fn entity_id(entities: &BTreeMap<usize, Entity>, sought: &Entity) -> usize {
    // Called once per product definition; using the pointer avoids adding an
    // `id` field to the shared parser's public-within-crate Entity contract.
    entities
        .iter()
        .find_map(|(id, entity)| std::ptr::eq(entity, sought).then_some(*id))
        .unwrap_or(0)
}

pub(crate) fn parse(
    entities: &BTreeMap<usize, Entity>,
    mapping: &[(TopoKind, usize, usize)],
    body_products: &[Option<usize>],
    product_parents: BTreeMap<usize, usize>,
) -> Result<StepPresentation> {
    let face_count = mapping.iter().filter(|(kind, _, _)| *kind == TopoKind::Face).count();
    let body_count = mapping.iter().filter(|(kind, _, _)| *kind == TopoKind::Body).count();
    let face_indices: BTreeMap<usize, usize> = mapping
        .iter()
        .filter_map(|(kind, index, entity)| (*kind == TopoKind::Face).then_some((*entity, *index)))
        .collect();
    let body_indices: BTreeMap<usize, usize> = mapping
        .iter()
        .filter_map(|(kind, index, entity)| (*kind == TopoKind::Body).then_some((*entity, *index)))
        .collect();
    let (representation_products, _, definition_products) = representation_products(entities)?;
    let mut result = StepPresentation {
        face_styles: vec![None; face_count],
        body_styles: vec![None; body_count],
        body_products: body_products.to_vec(),
        product_parents,
        ..StepPresentation::default()
    };
    let mut styled_colours = BTreeMap::<usize, Option<StepColour>>::new();
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "STYLED_ITEM"
            && args.len() == 3
        {
            let mut found = None;
            for style in list(&args[1], "styled item styles")? {
                let style = one_ref(style, "styled item style")?;
                if let Some(colour) = colour(entities, style, 0)? {
                    found = Some(colour);
                    break;
                }
            }
            if found.is_none() {
                result.unsupported.push(StepPresentationUnsupported {
                    code: "STEP_PRESENTATION_STYLE_UNSUPPORTED",
                    entity: id,
                    detail: "No COLOUR_RGB/fill_area_style_colour chain was retained".into(),
                });
            }
            styled_colours.insert(id, found);
            let target = one_ref(&args[2], "styled representation item")?;
            let style = StepPresentationStyle { colour: found, layers: vec![] };
            if let Some(face) = face_indices.get(&target) {
                result.face_styles[*face] = Some(style);
            } else if let Some(body) = body_indices.get(&target) {
                result.body_styles[*body] = Some(style);
            } else if let Some(product) = representation_products.get(&target).copied().or_else(|| {
                (call(entities, target).ok()?.0 == "PRODUCT_DEFINITION")
                    .then(|| definition_products.get(&target).copied())
                    .flatten()
            }) {
                result.product_styles.insert(product, style);
            } else if call(entities, target)?.0 == "PRODUCT" {
                result.product_styles.insert(target, style);
            } else {
                result.unsupported.push(StepPresentationUnsupported {
                    code: "STEP_PRESENTATION_TARGET_UNSUPPORTED",
                    entity: id,
                    detail: format!("Styled target #{target} is not a retained face, body, or product"),
                });
            }
        }
    }
    for (&id, entity) in entities {
        if let Value::Call(kind, args) = &entity.value
            && kind == "PRESENTATION_LAYER_ASSIGNMENT"
            && args.len() == 3
        {
            let Value::String(layer) = &args[0] else {
                return Err(super::refuse("Presentation layer name must be a string"));
            };
            for assigned in list(&args[2], "presentation layer items")? {
                let assigned = one_ref(assigned, "presentation layer item")?;
                let target = styled_target(entities, assigned)?.unwrap_or(assigned);
                let styled = styled_colours.get(&assigned).copied().flatten();
                let style = StepPresentationStyle { colour: styled, layers: vec![layer.clone()] };
                if let Some(product) = representation_products.get(&target).copied() {
                    let product_style = result.product_styles.entry(product).or_default();
                    if !product_style.layers.contains(layer) {
                        product_style.layers.push(layer.clone());
                    }
                    continue;
                }
                let slot = if let Some(face) = face_indices.get(&target) {
                    &mut result.face_styles[*face]
                } else if let Some(body) = body_indices.get(&target) {
                    &mut result.body_styles[*body]
                } else {
                    result.unsupported.push(StepPresentationUnsupported {
                        code: "STEP_PRESENTATION_LAYER_TARGET_UNSUPPORTED",
                        entity: id,
                        detail: format!("Layer target #{target} is outside the retained product graph"),
                    });
                    continue;
                };
                if slot.is_none() {
                    *slot = Some(style);
                }
                let slot = slot.as_mut().unwrap();
                if !slot.layers.contains(layer) {
                    slot.layers.push(layer.clone());
                }
            }
        }
    }
    for styles in result.face_styles.iter_mut().chain(&mut result.body_styles) {
        if let Some(style) = styles {
            style.layers.sort();
            style.layers.dedup();
        }
    }
    Ok(result)
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn real(value: f64) -> String {
    format!("{value:.17e}").replace('e', "E")
}

pub(crate) fn emit(
    writer: &mut crate::step_v3_export::Writer,
    presentation: &StepPresentation,
    faces: &[usize],
    bodies: &[usize],
    representation: usize,
) -> Result<()> {
    for (target, style) in faces
        .iter()
        .zip(&presentation.face_styles)
        .filter_map(|(target, style)| style.as_ref().map(|style| (*target, style)))
        .chain(bodies.iter().zip(&presentation.body_styles).filter_map(|(target, style)| {
            style.as_ref().map(|style| (*target, style))
        }))
        .chain(presentation.product_styles.values().map(|style| (representation, style)))
    {
        let colour = if let Some(colour) = style.colour {
            let colour = writer.emit(format!(
                "COLOUR_RGB('',{},{},{})",
                real(colour.red),
                real(colour.green),
                real(colour.blue)
            ));
            let fill_colour = writer.emit(format!("FILL_AREA_STYLE_COLOUR('',#{colour})"));
            let fill = writer.emit(format!("FILL_AREA_STYLE('',(#{fill_colour}))"));
            let surface_fill = writer.emit(format!("SURFACE_STYLE_FILL_AREA(#{fill})"));
            let side = writer.emit(format!("SURFACE_SIDE_STYLE('',(#{surface_fill}))"));
            Some(writer.emit(format!("SURFACE_STYLE_USAGE('',#{side})")))
        } else {
            None
        };
        let styled = colour.map(|colour| {
            writer.emit(format!("STYLED_ITEM('',(#{colour}),#{target})"))
        });
        let layer_target = styled.unwrap_or(target);
        for layer in &style.layers {
            writer.emit(format!(
                "PRESENTATION_LAYER_ASSIGNMENT({},'',(#{layer_target}))",
                quote(layer)
            ));
        }
    }
    Ok(())
}
