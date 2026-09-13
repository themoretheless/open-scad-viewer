// Copyright 2026 the OpenSCAD Viewer Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bind non-scaling strokes in shared paint resources to their painted instance.

use std::collections::HashSet;
use std::sync::Arc;

use crate::tiny_skia_path::Transform;
use crate::{Group, ImageKind, Mask, Node, NonEmptyString, Paint, Path, Pattern, Tree, filter};

/// Failure to resolve instance-dependent non-scaling stroke coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonScalingContextError {
    /// Traversal, copying, or nesting would exceed the supplied work budget.
    LimitExceeded,
    /// A painted instance has a non-finite or non-invertible transform.
    InvalidTransform,
}

impl std::fmt::Display for NonScalingContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::LimitExceeded => {
                "SVG non-scaling stroke context expansion exceeds its work budget"
            }
            Self::InvalidTransform => "SVG non-scaling stroke context has an invalid transform",
        })
    }
}

impl std::error::Error for NonScalingContextError {}

type Result<T> = std::result::Result<T, NonScalingContextError>;
type Resource = (u8, usize);

fn key<T>(kind: u8, value: &Arc<T>) -> Resource {
    (kind, Arc::as_ptr(value) as usize)
}

struct Admission {
    remaining: usize,
    affected: HashSet<Resource>,
    ids: HashSet<String>,
}

impl Admission {
    fn charge(&mut self, amount: usize) -> Result<()> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or(NonScalingContextError::LimitExceeded)?;
        Ok(())
    }

    fn id(&mut self, value: &str) -> Result<()> {
        self.charge(1 + value.len())?;
        if !value.is_empty() && !self.ids.contains(value) {
            self.ids.insert(value.to_owned());
        }
        Ok(())
    }

    fn group(&mut self, group: &Group, depth: usize) -> Result<bool> {
        if depth > 128 {
            return Err(NonScalingContextError::LimitExceeded);
        }
        self.id(&group.id)?;
        let mut affected = false;
        if let Some(mask) = &group.mask {
            affected |= self.mask(mask, depth + 1)?;
        }
        if let Some(clip) = &group.clip_path {
            self.clip(clip, depth + 1)?;
        }
        for filter in &group.filters {
            affected |= self.filter(filter, depth + 1)?;
        }
        for node in &group.children {
            self.charge(1)?;
            affected |= match node {
                Node::Group(group) => self.group(group, depth + 1)?,
                Node::Path(path) => {
                    self.id(&path.id)?;
                    self.charge(path.data.points().len())?;
                    let mut affected = false;
                    if let Some(fill) = &path.fill {
                        affected |= self.paint(&fill.paint, depth + 1)?;
                    }
                    if let Some(stroke) = &path.stroke {
                        self.charge(stroke.dasharray.as_ref().map_or(0, Vec::len))?;
                        affected |= stroke.is_non_scaling();
                        affected |= self.paint(&stroke.paint, depth + 1)?;
                    }
                    affected
                }
                Node::Text(text) => {
                    self.id(&text.id)?;
                    // Resource clones retain rendered outlines, not text layout caches.
                    self.group(&text.flattened, depth + 1)?
                }
                Node::Image(image) => {
                    self.id(&image.id)?;
                    if let ImageKind::SVG(tree) = &image.kind {
                        self.group(&tree.root, depth + 1)?
                    } else {
                        false
                    }
                }
            };
        }
        Ok(affected)
    }

    fn paint(&mut self, paint: &Paint, depth: usize) -> Result<bool> {
        match paint {
            Paint::Pattern(pattern) => {
                self.id(pattern.id())?;
                let affected = self.group(&pattern.root, depth + 1)?;
                if affected {
                    self.affected.insert(key(0, pattern));
                }
                Ok(affected)
            }
            Paint::LinearGradient(gradient) => {
                self.id(gradient.id())?;
                Ok(false)
            }
            Paint::RadialGradient(gradient) => {
                self.id(gradient.id())?;
                Ok(false)
            }
            Paint::Color(_) => Ok(false),
        }
    }

    fn mask(&mut self, mask: &Arc<Mask>, depth: usize) -> Result<bool> {
        self.id(mask.id())?;
        let mut affected = self.group(&mask.root, depth + 1)?;
        if let Some(nested) = &mask.mask {
            affected |= self.mask(nested, depth + 1)?;
        }
        if affected {
            self.affected.insert(key(1, mask));
        }
        Ok(affected)
    }

    fn clip(&mut self, clip: &Arc<crate::ClipPath>, depth: usize) -> Result<()> {
        self.id(clip.id())?;
        // Clip strokes are not painted, but their IDs still occupy the namespace.
        self.group(&clip.root, depth + 1)?;
        if let Some(nested) = &clip.clip_path {
            self.clip(nested, depth + 1)?;
        }
        Ok(())
    }

    fn filter(&mut self, filter: &Arc<filter::Filter>, depth: usize) -> Result<bool> {
        self.id(filter.id())?;
        let mut affected = false;
        for primitive in &filter.primitives {
            self.charge(1 + primitive.result.len())?;
            self.filter_inputs(&primitive.kind)?;
            match &primitive.kind {
                filter::Kind::Image(image) => affected |= self.group(&image.root, depth + 1)?,
                filter::Kind::ConvolveMatrix(matrix) => self.charge(matrix.matrix.data().len())?,
                filter::Kind::ComponentTransfer(transfer) => {
                    for function in [
                        &transfer.func_r,
                        &transfer.func_g,
                        &transfer.func_b,
                        &transfer.func_a,
                    ] {
                        if let filter::TransferFunction::Table(values)
                        | filter::TransferFunction::Discrete(values) = function
                        {
                            self.charge(values.len())?;
                        }
                    }
                }
                filter::Kind::Merge(merge) => self.charge(merge.inputs.len())?,
                _ => {}
            }
        }
        if affected {
            self.affected.insert(key(2, filter));
        }
        Ok(affected)
    }

    fn input(&mut self, input: &filter::Input) -> Result<()> {
        self.charge(1)?;
        if let filter::Input::Reference(name) = input {
            self.charge(name.len())?;
        }
        Ok(())
    }

    fn filter_inputs(&mut self, kind: &filter::Kind) -> Result<()> {
        match kind {
            filter::Kind::Blend(value) => {
                self.input(&value.input1)?;
                self.input(&value.input2)?;
            }
            filter::Kind::Composite(value) => {
                self.input(&value.input1)?;
                self.input(&value.input2)?;
            }
            filter::Kind::DisplacementMap(value) => {
                self.input(&value.input1)?;
                self.input(&value.input2)?;
            }
            filter::Kind::ColorMatrix(value) => {
                self.input(&value.input)?;
                if let filter::ColorMatrixKind::Matrix(values) = &value.kind {
                    self.charge(values.len())?;
                }
            }
            filter::Kind::ComponentTransfer(value) => self.input(&value.input)?,
            filter::Kind::ConvolveMatrix(value) => self.input(&value.input)?,
            filter::Kind::DiffuseLighting(value) => self.input(&value.input)?,
            filter::Kind::DropShadow(value) => self.input(&value.input)?,
            filter::Kind::GaussianBlur(value) => self.input(&value.input)?,
            filter::Kind::Morphology(value) => self.input(&value.input)?,
            filter::Kind::Offset(value) => self.input(&value.input)?,
            filter::Kind::SpecularLighting(value) => self.input(&value.input)?,
            filter::Kind::Tile(value) => self.input(&value.input)?,
            filter::Kind::Merge(value) => {
                for input in &value.inputs {
                    self.input(input)?;
                }
            }
            filter::Kind::Flood(_) | filter::Kind::Image(_) | filter::Kind::Turbulence(_) => {}
        }
        Ok(())
    }
}

struct Resolver {
    affected: HashSet<Resource>,
    ids: HashSet<String>,
    next_id: usize,
}

impl Resolver {
    fn id(&mut self) -> NonEmptyString {
        loop {
            let id = format!("__usvg_non_scaling_{}", self.next_id);
            self.next_id += 1;
            if self.ids.insert(id.clone()) {
                return NonEmptyString::new(id).unwrap();
            }
        }
    }

    // `host` already includes this group's transform. A resource root is rendered
    // through render_nodes, so only actual child-group transforms are composed.
    fn group(&mut self, group: &mut Group, host: Transform) -> Result<()> {
        group.abs_transform = host;
        if let Some(mask) = &mut group.mask {
            self.mask(mask, host)?;
        }
        for filter in &mut group.filters {
            self.filter(filter, host)?;
        }
        for node in &mut group.children {
            match node {
                Node::Group(child) => self.group(child, host.pre_concat(child.transform))?,
                Node::Path(path) => {
                    if let Some(fill) = &mut path.fill {
                        self.paint(&mut fill.paint, host)?;
                    }
                    if let Some(stroke) = &mut path.stroke {
                        self.paint(&mut stroke.paint, host)?;
                    }
                    **path = Path::new(
                        path.id.clone(),
                        path.visible,
                        path.fill.clone(),
                        path.stroke.clone(),
                        path.paint_order,
                        path.rendering_mode,
                        path.data.clone(),
                        host,
                    )
                    .ok_or(NonScalingContextError::InvalidTransform)?;
                }
                Node::Text(text) => {
                    text.abs_transform = host;
                    let text_host = host.pre_concat(text.flattened.transform);
                    self.group(&mut text.flattened, text_host)?;
                    text.abs_bounding_box = text
                        .bounding_box
                        .transform(host)
                        .ok_or(NonScalingContextError::InvalidTransform)?;
                    text.stroke_bounding_box = text.flattened.stroke_bounding_box;
                    text.abs_stroke_bounding_box = text.flattened.abs_stroke_bounding_box;
                }
                Node::Image(image) => {
                    image.abs_transform = host;
                    image.abs_bounding_box = image
                        .size
                        .to_non_zero_rect(0., 0.)
                        .transform(host)
                        .ok_or(NonScalingContextError::InvalidTransform)?;
                    if let ImageKind::SVG(tree) = &mut image.kind {
                        // An embedded SVG image establishes its own outermost viewport.
                        self.group(&mut tree.root, Transform::identity())?;
                        refresh_definitions(tree);
                    }
                }
            }
        }
        // Empty groups legitimately have no non-zero layer box.
        if !group.children.is_empty() && group.calculate_bounding_boxes().is_none() {
            return Err(NonScalingContextError::InvalidTransform);
        }
        Ok(())
    }

    fn paint(&mut self, paint: &mut Paint, host: Transform) -> Result<()> {
        let Paint::Pattern(original) = paint else {
            return Ok(());
        };
        if !self.affected.contains(&key(0, original)) {
            return Ok(());
        }
        let mut pattern = Pattern {
            id: self.id(),
            units: original.units,
            content_units: original.content_units,
            transform: original.transform,
            rect: original.rect,
            view_box: original.view_box,
            root: clone_group(&original.root),
        };
        self.group(&mut pattern.root, host.pre_concat(pattern.transform))?;
        *original = Arc::new(pattern);
        Ok(())
    }

    fn mask(&mut self, original: &mut Arc<Mask>, host: Transform) -> Result<()> {
        if !self.affected.contains(&key(1, original)) {
            return Ok(());
        }
        let mut mask = Mask {
            id: self.id(),
            rect: original.rect,
            kind: original.kind,
            mask: original.mask.clone(),
            root: clone_group(&original.root),
        };
        self.group(&mut mask.root, host)?;
        if let Some(nested) = &mut mask.mask {
            self.mask(nested, host)?;
        }
        *original = Arc::new(mask);
        Ok(())
    }

    fn filter(&mut self, original: &mut Arc<filter::Filter>, host: Transform) -> Result<()> {
        if !self.affected.contains(&key(2, original)) {
            return Ok(());
        }
        let mut primitives = Vec::with_capacity(original.primitives.len());
        for primitive in &original.primitives {
            let kind = if let filter::Kind::Image(image) = &primitive.kind {
                let mut root = clone_group(&image.root);
                // feImage is rendered into the filter's axis-aligned intermediate
                // canvas; translations do not affect stroke construction.
                let (sx, sy) = host.get_scale();
                self.group(&mut root, Transform::from_scale(sx, sy))?;
                filter::Kind::Image(filter::Image { root })
            } else {
                primitive.kind.clone()
            };
            primitives.push(filter::Primitive {
                rect: primitive.rect,
                color_interpolation: primitive.color_interpolation,
                result: primitive.result.clone(),
                kind,
            });
        }
        *original = Arc::new(filter::Filter {
            id: self.id(),
            rect: original.rect,
            primitives,
        });
        Ok(())
    }
}

// Clone only rendered state. Path points and raster bytes remain shared; text
// layout metadata is discarded in favour of its existing outlined group. IDs on
// cloned descendants are removed because selectors/references are already bound.
fn clone_group(group: &Group) -> Group {
    let children = group
        .children
        .iter()
        .map(|node| match node {
            Node::Group(child) => Node::Group(Box::new(clone_group(child))),
            Node::Path(path) => {
                let mut path = path.as_ref().clone();
                path.id.clear();
                Node::Path(Box::new(path))
            }
            Node::Text(text) => Node::Group(Box::new(clone_group(&text.flattened))),
            Node::Image(image) => {
                let kind = if let ImageKind::SVG(tree) = &image.kind {
                    ImageKind::SVG(Tree {
                        size: tree.size,
                        root: clone_group(&tree.root),
                        linear_gradients: vec![],
                        radial_gradients: vec![],
                        patterns: vec![],
                        clip_paths: vec![],
                        masks: vec![],
                        filters: vec![],
                        #[cfg(feature = "text")]
                        fontdb: tree.fontdb.clone(),
                    })
                } else {
                    image.kind.clone()
                };
                Node::Image(Box::new(crate::Image {
                    id: String::new(),
                    visible: image.visible,
                    size: image.size,
                    rendering_mode: image.rendering_mode,
                    kind,
                    abs_transform: image.abs_transform,
                    abs_bounding_box: image.abs_bounding_box,
                }))
            }
        })
        .collect();
    Group {
        id: String::new(),
        transform: group.transform,
        abs_transform: group.abs_transform,
        opacity: group.opacity,
        blend_mode: group.blend_mode,
        isolate: group.isolate,
        clip_path: group.clip_path.clone(),
        is_context_element: group.is_context_element,
        mask: group.mask.clone(),
        filters: group.filters.clone(),
        bounding_box: group.bounding_box,
        abs_bounding_box: group.abs_bounding_box,
        stroke_bounding_box: group.stroke_bounding_box,
        abs_stroke_bounding_box: group.abs_stroke_bounding_box,
        layer_bounding_box: group.layer_bounding_box,
        abs_layer_bounding_box: group.abs_layer_bounding_box,
        children,
    }
}

// The ordinary collect_* methods use linear duplicate searches. Context cloning
// can create many definitions, so rebuilding these caches uses pointer sets.
#[derive(Default)]
struct Definitions {
    seen: HashSet<Resource>,
    linear: Vec<Arc<crate::LinearGradient>>,
    radial: Vec<Arc<crate::RadialGradient>>,
    patterns: Vec<Arc<Pattern>>,
    clips: Vec<Arc<crate::ClipPath>>,
    masks: Vec<Arc<Mask>>,
    filters: Vec<Arc<filter::Filter>>,
}

impl Definitions {
    fn group(&mut self, group: &Group) {
        if let Some(mask) = &group.mask {
            self.mask(mask);
        }
        if let Some(clip) = &group.clip_path {
            self.clip(clip);
        }
        for filter in &group.filters {
            if self.seen.insert(key(2, filter)) {
                self.filters.push(filter.clone());
                for primitive in &filter.primitives {
                    if let filter::Kind::Image(image) = &primitive.kind {
                        self.group(&image.root);
                    }
                }
            }
        }
        for node in &group.children {
            match node {
                Node::Group(group) => self.group(group),
                Node::Path(path) => {
                    if let Some(fill) = &path.fill {
                        self.paint(&fill.paint);
                    }
                    if let Some(stroke) = &path.stroke {
                        self.paint(&stroke.paint);
                    }
                }
                Node::Text(text) => self.group(&text.flattened),
                Node::Image(image) => {
                    if let ImageKind::SVG(tree) = &image.kind {
                        self.group(&tree.root);
                    }
                }
            }
        }
    }

    fn paint(&mut self, paint: &Paint) {
        match paint {
            Paint::Pattern(pattern) if self.seen.insert(key(0, pattern)) => {
                self.patterns.push(pattern.clone());
                self.group(&pattern.root);
            }
            Paint::LinearGradient(gradient) if self.seen.insert(key(3, gradient)) => {
                self.linear.push(gradient.clone())
            }
            Paint::RadialGradient(gradient) if self.seen.insert(key(4, gradient)) => {
                self.radial.push(gradient.clone())
            }
            _ => {}
        }
    }

    fn mask(&mut self, mask: &Arc<Mask>) {
        if self.seen.insert(key(1, mask)) {
            self.masks.push(mask.clone());
            self.group(&mask.root);
            if let Some(nested) = &mask.mask {
                self.mask(nested);
            }
        }
    }

    fn clip(&mut self, clip: &Arc<crate::ClipPath>) {
        if self.seen.insert(key(5, clip)) {
            self.clips.push(clip.clone());
            self.group(&clip.root);
            if let Some(nested) = &clip.clip_path {
                self.clip(nested);
            }
        }
    }
}

fn refresh_definitions(tree: &mut Tree) {
    let mut definitions = Definitions::default();
    definitions.group(&tree.root);
    tree.linear_gradients = definitions.linear;
    tree.radial_gradients = definitions.radial;
    tree.patterns = definitions.patterns;
    tree.clip_paths = definitions.clips;
    tree.masks = definitions.masks;
    tree.filters = definitions.filters;
}

impl Tree {
    /// Resolves non-scaling strokes against each painted instance's outer SVG viewport.
    ///
    /// Call after parsing and before rendering or writing. Shared patterns, masks,
    /// and filter images containing these strokes receive independent resources.
    /// Rendered text in cloned resources is retained as outlines. Embedded SVG
    /// images retain their independent viewport.
    ///
    /// `max_work` bounds expanded nodes, path points, identifiers and variable-size
    /// filter data across all resource references. Admission and the depth limit
    /// are checked before any tree mutation or resource clone.
    pub fn resolve_non_scaling_contexts(&mut self, max_work: usize) -> Result<()> {
        let mut admission = Admission {
            remaining: max_work,
            affected: HashSet::new(),
            ids: HashSet::new(),
        };
        if !admission.group(&self.root, 0)? {
            return Ok(());
        }
        let mut resolver = Resolver {
            affected: admission.affected,
            ids: admission.ids,
            next_id: 0,
        };
        resolver.group(&mut self.root, Transform::identity())?;
        refresh_definitions(self);
        Ok(())
    }
}
