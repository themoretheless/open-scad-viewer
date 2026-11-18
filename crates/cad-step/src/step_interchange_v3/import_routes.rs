use super::*;

pub(super) const UNREACHABLE_ALLOWLIST: &[&str] = &[
    "SURFACE_STYLE_PARAMETER_LINE", "SURFACE_STYLE_TRANSPARENT", "TESSELLATED_SHAPE_REPRESENTATION",
    "CONCENTRICITY_TOLERANCE", "PRODUCT_DEFINITION_CONTEXT_ROLE", "SYMMETRY_TOLERANCE",
    "ROUNDNESS_TOLERANCE", "CYLINDRICITY_TOLERANCE", "APPLIED_DOCUMENT_REFERENCE",
    "TOTAL_RUNOUT_TOLERANCE", "OBJECT_ROLE", "LIMITS_AND_FITS", "DOCUMENT_PRODUCT_EQUIVALENCE",
    "ROLE_ASSOCIATION", "APPLICATION_CONTEXT", "APPLICATION_PROTOCOL_DEFINITION",
    "DEFAULT_MODEL_GEOMETRIC_VIEW", "MODEL_GEOMETRIC_VIEW", "CAMERA_MODEL_D3",
    "REPRESENTATION_MAP", "PRODUCT_CATEGORY_RELATIONSHIP", "PRODUCT", "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION", "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_RELATED_PRODUCT_CATEGORY", "PRODUCT_CATEGORY",
    "MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP", "MAPPED_ITEM", "REAL_REPRESENTATION_ITEM",
    "PLANE", "DRAUGHTING_MODEL", "VIEW_VOLUME", "DRAUGHTING_PRE_DEFINED_CURVE_FONT",
    "DIMENSIONAL_EXPONENTS", "PRODUCT_CONTEXT", "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_DEFINITION_SHAPE", "SHAPE_DEFINITION_REPRESENTATION",
    "ADVANCED_BREP_SHAPE_REPRESENTATION", "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "SHELL_BASED_SURFACE_MODEL", "COORDINATES_LIST", "TRIANGULATED_FACE", "TESSELLATED_FACE",
    "TESSELLATED_SHELL", "GEOMETRIC_MODEL_RELATIONSHIP", "GEOMETRIC_REPRESENTATION_CONTEXT",
    "GLOBAL_UNIT_ASSIGNED_CONTEXT", "SI_UNIT", "PRESENTATION_LAYER_ASSIGNMENT", "STYLED_ITEM",
    "COLOUR_RGB", "SURFACE_STYLE_USAGE", "SURFACE_SIDE_STYLE", "SURFACE_STYLE_FILL_AREA",
    "FILL_AREA_STYLE", "FILL_AREA_STYLE_COLOUR", "DRAUGHTING_PRE_DEFINED_COLOUR",
    "CAMERA_MODEL_D3_MULTI_CLIPPING", "PROPERTY_DEFINITION", "PROPERTY_DEFINITION_REPRESENTATION",
    "DESCRIPTIVE_REPRESENTATION_ITEM", "MEASURE_REPRESENTATION_ITEM", "PLANAR_BOX", "INVISIBILITY",
    "VALUE_REPRESENTATION_ITEM", "CURVE_STYLE", "DRAUGHTING_CALLOUT_RELATIONSHIP",
    "GENERAL_PROPERTY_ASSOCIATION", "PRESENTATION_STYLE_ASSIGNMENT", "DIMENSIONAL_LOCATION",
    "SURFACE_PROFILE_TOLERANCE", "LINE", "GENERAL_PROPERTY",
    "FEATURE_FOR_DATUM_TARGET_RELATIONSHIP", "SHAPE_ASPECT", "PRE_DEFINED_MARKER",
    "GEOMETRIC_TOLERANCE", "FLATNESS_TOLERANCE", "ANGULARITY_TOLERANCE", "TRIMMED_CURVE",
    "INTEGER_REPRESENTATION_ITEM", "PLACED_DATUM_TARGET_FEATURE", "ID_ATTRIBUTE", "POINT_STYLE",
    "ALL_AROUND_SHAPE_ASPECT", "TOLERANCE_ZONE_FORM", "COMPOSITE_CURVE_SEGMENT",
    "DRAUGHTING_MODEL_ITEM_ASSOCIATION", "GEOMETRIC_CURVE_SET", "GEOMETRIC_TOLERANCE_RELATIONSHIP",
    "COMPLEX_TRIANGULATED_SURFACE_SET", "DATUM_FEATURE", "ITEM_IDENTIFIED_REPRESENTATION_USAGE",
    "TOLERANCE_ZONE", "COMPOSITE_CURVE", "ANNOTATION_PLANE", "MEASURE_WITH_UNIT", "CIRCLE",
    "DATUM_REFERENCE_ELEMENT", "TESSELLATED_CURVE_SET", "VERTEX_POINT", "SHAPE_ASPECT_RELATIONSHIP",
    "ANGULAR_LOCATION", "PERPENDICULARITY_TOLERANCE", "DATUM_REFERENCE_COMPARTMENT",
    "PLANE_ANGLE_MEASURE_WITH_UNIT", "OVER_RIDING_STYLED_ITEM", "EDGE_CURVE",
    "PARALLELISM_TOLERANCE", "TESSELLATED_GEOMETRIC_SET", "COMPOSITE_GROUP_SHAPE_ASPECT", "DATUM",
    "DATUM_SYSTEM", "CONVERSION_BASED_UNIT", "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION",
    "COMPOUND_REPRESENTATION_ITEM", "ORIENTED_EDGE", "TESSELLATED_ANNOTATION_OCCURRENCE",
    "TOLERANCE_VALUE", "DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER", "EDGE_LOOP",
    "DRAUGHTING_CALLOUT", "PLUS_MINUS_TOLERANCE", "SHAPE_DIMENSION_REPRESENTATION",
    "ANNOTATION_PLACEHOLDER_OCCURRENCE", "FACE_OUTER_BOUND", "ANNOTATION_CURVE_OCCURRENCE",
    "MEASURE_QUALIFICATION", "GEOMETRIC_ITEM_SPECIFIC_USAGE",
    "DIMENSIONAL_CHARACTERISTIC_REPRESENTATION", "GEOMETRIC_SET", "ADVANCED_FACE", "POLYLINE",
    "COMPOSITE_SHAPE_ASPECT", "CYLINDRICAL_SURFACE", "DIMENSIONAL_SIZE",
    "VALUE_FORMAT_TYPE_QUALIFIER", "OPEN_SHELL",
    "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION", "TYPE_QUALIFIER", "COLOUR",
    "LENGTH_MEASURE_WITH_UNIT", "B_SPLINE_CURVE_WITH_KNOTS", "PRE_DEFINED_POINT_MARKER_SYMBOL",
    "CONSTRUCTIVE_GEOMETRY_REPRESENTATION", "CIRCULAR_RUNOUT_TOLERANCE", "MASS_MEASURE_WITH_UNIT",
    "CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP", "PROJECTED_ZONE_DEFINITION", "FACE_BOUND",
    "SURFACE_STYLE_BOUNDARY", "SURFACE_STYLE_RENDERING_WITH_PROPERTIES", "ANGULAR_SIZE",
    "STRAIGHTNESS_TOLERANCE", "PRODUCT_DEFINITION_CONTEXT_ASSOCIATION",
    "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION", "DATUM_TARGET", "VECTOR",
    "REPRESENTATION", "DERIVED_UNIT", "DERIVED_UNIT_ELEMENT", "DENSITY_UNIT", "MASS_UNIT",
    "LENGTH_UNIT", "AREA_UNIT", "VOLUME_UNIT", "NAMED_UNIT", "NAME_ATTRIBUTE",
    "MATERIAL_DESIGNATION", "APPLIED_NAME_ASSIGNMENT", "APPLIED_DESCRIPTION_TEXT_ASSIGNMENT",
    "PERSON", "ORGANIZATION", "PERSON_AND_ORGANIZATION", "PERSON_AND_ORGANIZATION_ROLE",
    "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT", "DATE_AND_TIME", "CALENDAR_DATE", "LOCAL_TIME",
    "COORDINATED_UNIVERSAL_TIME_OFFSET", "DATE_TIME_ROLE", "APPLIED_DATE_AND_TIME_ASSIGNMENT",
    "APPROVAL", "APPROVAL_STATUS", "APPROVAL_ROLE", "APPLIED_APPROVAL_ASSIGNMENT",
    "SECURITY_CLASSIFICATION", "SECURITY_CLASSIFICATION_LEVEL",
    "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT", "EXTERNAL_CLASS_LIBRARY",
    "ITEM_DEFINED_TRANSFORMATION", "AXIS2_PLACEMENT_3D", "CARTESIAN_POINT", "DIRECTION",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION", "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
];

const EXACT_BREP_REPRESENTATION: &str = "ADVANCED_BREP_SHAPE_REPRESENTATION";
const AMBIGUOUS_EXACT_ROOTS: &str = "Ambiguous multiple exact B-rep representation roots";
const NON_UNIFORM_OPERATOR: &str = "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM";

const REACHABLE_TYPES: &[&str] = &[
    "SHAPE_REPRESENTATION", "ADVANCED_BREP_SHAPE_REPRESENTATION", "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "MANIFOLD_SOLID_BREP", "BREP_WITH_VOIDS", "SHELL_BASED_SURFACE_MODEL", "CLOSED_SHELL", "OPEN_SHELL",
    "ORIENTED_CLOSED_SHELL", "ADVANCED_FACE", "FACE_OUTER_BOUND", "FACE_BOUND", "EDGE_LOOP", "VERTEX_LOOP",
    "ORIENTED_EDGE", "EDGE_CURVE", "VERTEX_POINT", "CARTESIAN_POINT", "DIRECTION", "VECTOR",
    "AXIS2_PLACEMENT_2D", "AXIS2_PLACEMENT_3D", "LINE", "CIRCLE", "ELLIPSE", "TRIMMED_CURVE",
    "B_SPLINE_CURVE_WITH_KNOTS", "PCURVE", "SURFACE_CURVE", "SEAM_CURVE", "DEFINITIONAL_REPRESENTATION",
    "PLANE", "CYLINDRICAL_SURFACE", "CONICAL_SURFACE", "SPHERICAL_SURFACE", "TOROIDAL_SURFACE",
    "B_SPLINE_SURFACE_WITH_KNOTS", "CURVE_BOUNDED_SURFACE", "GEOMETRIC_REPRESENTATION_CONTEXT",
    "GLOBAL_UNIT_ASSIGNED_CONTEXT", "SI_UNIT", "CONVERSION_BASED_UNIT",
    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT", "UNCERTAINTY_MEASURE_WITH_UNIT", "LENGTH_MEASURE_WITH_UNIT",
    "PLANE_ANGLE_MEASURE_WITH_UNIT", "MEASURE_WITH_UNIT", "DIMENSIONAL_EXPONENTS",
    "ITEM_DEFINED_TRANSFORMATION", "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
    "CARTESIAN_TRANSFORMATION_OPERATOR_3D", "GEOMETRIC_TOLERANCE",
    "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "POSITION_TOLERANCE", "TESSELLATED_GEOMETRIC_SET",
];

const EXTERNAL_REFERENCE_TYPES: &[&str] = &[
    "DOCUMENT_FILE", "DOCUMENT", "DOCUMENT_TYPE", "DOCUMENT_REPRESENTATION_TYPE",
    "EXTERNALLY_DEFINED_ITEM", "EXTERNAL_SOURCE", "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT",
];

const BASE_CERTIFICATE_NOTES: &[&str] = &[
    "direct_part21_topology", "line_circle_ellipse_trimmed_curve",
    "plane_and_rational_multispan_bspline_surface", "periodic_analytic_surfaces_typed_refused_parameterization",
    "shared_topology_senses_holes", "multi_body_multi_cavity", "si_and_positive_conversion_units",
    "reachable_rigid_placements_only", "independent_curve_pcurve_check", "bounded_reachable_graph",
];

type DirectImport = (Model, FeatureCertificate, StepV3Report, Option<StepSemanticMetadata>);

/// One direct STEP import: the parsed graph plus the capability switches,
/// with one method per import stage.
struct DirectImportRoute<'a> {
    entities: &'a BTreeMap<usize, Entity>,
    capability: &'static str,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
}

impl DirectImportRoute<'_> {
    fn is_v9_or_later(&self) -> bool {
        matches!(self.capability, STEP_INTERCHANGE_V9_CAPABILITY | STEP_INTERCHANGE_V10_CAPABILITY)
    }

    fn whole_domain_proofs(&self) -> bool {
        matches!(
            self.capability,
            STEP_INTERCHANGE_V7_CAPABILITY
                | STEP_INTERCHANGE_V8_CAPABILITY
                | STEP_INTERCHANGE_V9_CAPABILITY
                | STEP_INTERCHANGE_V10_CAPABILITY
        )
    }

    fn allow_open_shells(&self) -> bool {
        self.is_v9_or_later()
    }

    fn skips_non_uniform_operators(&self) -> bool {
        self.allow_degenerate && self.capability != STEP_INTERCHANGE_V8_CAPABILITY
    }

    fn is_type(&self, id: usize, ty: &str) -> bool {
        call(self.entities, id).is_ok_and(|(found, _)| found == ty)
    }

    fn build_options(&self, ap242_composition: bool) -> DirectBuildOptions {
        DirectBuildOptions {
            analytic_surfaces: self.analytic_surfaces,
            ap242_composition,
            allow_degenerate: self.allow_degenerate,
            whole_domain_proofs: self.whole_domain_proofs(),
            interior_point_selectors: self.is_v9_or_later(),
            allow_open_shells: self.allow_open_shells(),
            preserve_authored_charts: self.capability == STEP_INTERCHANGE_V6_CAPABILITY,
            repair_pcurves: self.capability == STEP_INTERCHANGE_V10_CAPABILITY,
        }
    }

    fn select_representations(&self, auxiliary: &mut BTreeSet<usize>) -> Result<Vec<usize>> {
        if !self.ap242_composition {
            return Ok(self
                .entities
                .keys()
                .copied()
                .filter(|id| self.is_type(*id, EXACT_BREP_REPRESENTATION))
                .collect());
        }
        let selected = self.shape_definition_selections()?;
        if self.capability != STEP_INTERCHANGE_V10_CAPABILITY {
            self.check_admitted_products(&selected)?;
            return Ok(selected);
        }
        self.select_exact_root(selected, auxiliary)
    }

    fn shape_definition_selections(&self) -> Result<Vec<usize>> {
        let mut selected = Vec::<usize>::new();
        for entity in self.entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "SHAPE_DEFINITION_REPRESENTATION"
                && let [_, representation] = args.as_slice()
            {
                selected.push(one_ref(representation, "selected shape representation")?);
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty() {
            return Err(refuse("AP242 import requires SHAPE_DEFINITION_REPRESENTATION selection"));
        }
        Ok(selected)
    }

    fn check_admitted_products(&self, selected: &[usize]) -> Result<()> {
        let allow_open_shells = self.allow_open_shells();
        for id in selected {
            let (ty, _) = call(self.entities, *id)?;
            if ty != EXACT_BREP_REPRESENTATION
                && !(allow_open_shells && ty == "SHAPE_REPRESENTATION")
                && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
            {
                return Err(refuse("Selected representation is outside the admitted direct B-rep products"));
            }
        }
        Ok(())
    }

    fn holds_direct_exact_body(&self, id: usize) -> bool {
        call(self.entities, id).is_ok_and(|(ty, args)| {
            ty == "SHAPE_REPRESENTATION"
                && args.get(1).is_some_and(|items| {
                    list(items, "exact representation items").is_ok_and(|items| {
                        items.iter().any(|item| {
                            one_ref(item, "exact representation item").is_ok_and(|item| {
                                call(self.entities, item)
                                    .is_ok_and(|(ty, _)| matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS"))
                            })
                        })
                    })
                })
        })
    }

    /// Representation relationships without a transformation operator.
    fn untransformed_edges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.entities.keys().filter_map(|relationship| {
            relationship_components(self.entities, *relationship)
                .ok()
                .flatten()
                .and_then(|(from, to, operator)| (operator == 0).then_some((from, to)))
        })
    }

    fn links_exact_representation(&self, id: usize) -> bool {
        self.untransformed_edges().any(|(from, to)| {
            (from == id || to == id) && self.is_type(if from == id { to } else { from }, EXACT_BREP_REPRESENTATION)
        })
    }

    fn select_exact_root(&self, selected: Vec<usize>, auxiliary: &mut BTreeSet<usize>) -> Result<Vec<usize>> {
        let exact = selected
            .iter()
            .copied()
            .filter(|id| self.is_type(*id, EXACT_BREP_REPRESENTATION))
            .collect::<Vec<_>>();
        let direct_exact = selected.iter().copied().filter(|id| self.holds_direct_exact_body(*id)).collect::<Vec<_>>();
        let containers = selected
            .iter()
            .copied()
            .filter(|id| !exact.contains(id) && !direct_exact.contains(id))
            .filter(|id| self.is_type(*id, "SHAPE_REPRESENTATION"))
            .filter(|id| self.links_exact_representation(*id))
            .collect::<Vec<_>>();
        if !exact.is_empty() || !containers.is_empty() {
            let root = self.container_root(&exact, &containers, auxiliary)?;
            auxiliary.extend(containers.iter().copied());
            auxiliary.extend(selected.iter().copied().filter(|id| *id != root));
            return Ok(vec![root]);
        }
        if let Some(&root) = direct_exact.first() {
            if direct_exact.len() != 1 {
                return Err(refuse(AMBIGUOUS_EXACT_ROOTS));
            }
            auxiliary.extend(selected.iter().copied().filter(|id| *id != root));
            return Ok(direct_exact);
        }
        self.check_admitted_products(&selected)?;
        Ok(selected)
    }

    fn container_root(&self, exact: &[usize], containers: &[usize], auxiliary: &mut BTreeSet<usize>) -> Result<usize> {
        if containers.len() > 1 || containers.is_empty() && exact.len() != 1 || containers.len() == 1 && exact.len() > 1 {
            return Err(refuse(AMBIGUOUS_EXACT_ROOTS));
        }
        let Some(mut root) = containers.first().or_else(|| exact.first()).copied() else {
            return Err(refuse(AMBIGUOUS_EXACT_ROOTS));
        };
        if containers.is_empty() {
            return Ok(root);
        }
        let unlinked = |candidate: usize| {
            !self
                .untransformed_edges()
                .any(|(from, to)| from == root && to == candidate || from == candidate && to == root)
        };
        if exact.iter().any(|candidate| unlinked(*candidate)) {
            return Err(refuse(AMBIGUOUS_EXACT_ROOTS));
        }
        let incoming = self
            .untransformed_edges()
            .filter_map(|(from, to)| (to == root && self.is_type(from, EXACT_BREP_REPRESENTATION)).then_some(from))
            .collect::<Vec<_>>();
        if incoming.len() > 1 {
            return Err(refuse(AMBIGUOUS_EXACT_ROOTS));
        }
        if let Some(exact) = incoming.first() {
            auxiliary.insert(root);
            root = *exact;
        }
        Ok(root)
    }

    fn all_bodies(&self) -> Vec<usize> {
        let allow_open_shells = self.allow_open_shells();
        self.entities
            .keys()
            .copied()
            .filter(|id| {
                call(self.entities, *id).is_ok_and(|(ty, _)| {
                    matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                        || allow_open_shells && ty == "SHELL_BASED_SURFACE_MODEL"
                })
            })
            .collect()
    }

    fn occurrences(&self, representations: &[usize]) -> Result<Vec<RepresentationOccurrence>> {
        if !self.ap242_composition {
            return Ok(Vec::new());
        }
        representation_occurrences(
            self.entities,
            representations,
            self.skips_non_uniform_operators(),
            self.is_v9_or_later(),
            self.allow_open_shells(),
        )
    }

    fn occurrence_identities(&self, occurrences: &[RepresentationOccurrence]) -> Result<(Vec<String>, Vec<String>)> {
        let mut definition_identities = Vec::new();
        let mut occurrence_identities = Vec::new();
        if !self.allow_degenerate {
            return Ok((definition_identities, occurrence_identities));
        }
        for occurrence in occurrences {
            let definition = format!("representation:{}", entity_digest(self.entities, occurrence.representation)?);
            if !definition_identities.contains(&definition) {
                definition_identities.push(definition)
            }
            let path = occurrence
                .path
                .iter()
                .map(|id| entity_digest(self.entities, *id))
                .collect::<Result<Vec<_>>>()?
                .join("/");
            let matrix = occurrence
                .transform
                .iter()
                .flatten()
                .map(|value| format!("{:016x}", value.to_bits()))
                .collect::<String>();
            occurrence_identities.push(format!("occurrence:{}", hash(&(path + &matrix))));
        }
        Ok((definition_identities, occurrence_identities))
    }

    fn product_hierarchy(&self) -> Vec<String> {
        if !self.allow_degenerate {
            return Vec::new();
        }
        self.entities
            .keys()
            .filter_map(|id| {
                let (ty, args) = call(self.entities, *id).ok()?;
                product_hierarchy_entry(*id, ty, args)
            })
            .collect()
    }

    fn external_references(&self) -> Vec<String> {
        if !self.allow_degenerate {
            return Vec::new();
        }
        self.entities
            .keys()
            .filter_map(|id| {
                let (ty, args) = call(self.entities, *id).ok()?;
                let uri = args.first().filter(|_| ty == "DOCUMENT_FILE")?;
                Some(format!("#{}:{}", id, string_value(uri, "document URI").ok()?))
            })
            .collect()
    }

    fn linked_graph(&self, graph_roots: &[usize], occurrences: &[RepresentationOccurrence]) -> Result<BTreeSet<usize>> {
        let mut linked = reachable(self.entities, graph_roots)?;
        if self.ap242_composition {
            if self.capability != STEP_INTERCHANGE_V9_CAPABILITY {
                linked.clear();
            }
            for occurrence in occurrences {
                linked.extend(occurrence.linked.iter().copied())
            }
        }
        Ok(linked)
    }

    fn extend_auxiliary(&self, representations: &[usize], auxiliary: &mut BTreeSet<usize>) -> Result<()> {
        loop {
            let previous = auxiliary.len();
            for id in self.entities.keys() {
                if let Some((from, to, 0)) = relationship_components(self.entities, *id)?
                    && (representations.contains(&from) || auxiliary.contains(&from))
                    && call(self.entities, to)?.0 != EXACT_BREP_REPRESENTATION
                {
                    auxiliary.insert(to);
                }
            }
            if auxiliary.len() == previous {
                break;
            }
        }
        for rep in representations {
            let (_, args) = call(self.entities, *rep)?;
            if let Some(items) = args.get(1) {
                for item in list(items, "selected representation items")? {
                    let item = one_ref(item, "selected representation item")?;
                    if matches!(call(self.entities, item)?.0, "MAPPED_ITEM" | "GEOMETRIC_CURVE_SET") {
                        auxiliary.insert(item);
                    }
                }
            }
        }
        Ok(())
    }

    fn is_skipped_non_uniform_operator(&self, id: usize) -> Result<bool> {
        Ok(self.skips_non_uniform_operators() && component(&components(self.entities, id)?, NON_UNIFORM_OPERATOR).is_some())
    }

    fn check_reachable_types(&self, linked: &BTreeSet<usize>) -> Result<()> {
        for id in linked {
            if self.is_skipped_non_uniform_operator(*id)? {
                continue;
            }
            let (ty, _) = call(self.entities, *id)?;
            if !REACHABLE_TYPES.contains(&ty) && !matches!(ty, "COMPLEX") {
                return Err(refuse(format!("Reachable entity {ty} is outside the direct finite graph subset")));
            }
        }
        Ok(())
    }

    fn ignored_entities(&self, linked: &BTreeSet<usize>, auxiliary_linked: &BTreeSet<usize>) -> Result<BTreeSet<String>> {
        let mut ignored = BTreeSet::new();
        for id in self.entities.keys() {
            if linked.contains(id) {
                continue;
            }
            if auxiliary_linked.contains(id) {
                ignored.insert(call(self.entities, *id)?.0.to_string());
                continue;
            }
            if self.ap242_composition && relationship_components(self.entities, *id)?.is_some() {
                continue;
            }
            if self.is_skipped_non_uniform_operator(*id)? {
                continue;
            }
            if self.entities.get(id).is_some_and(|entity| matches!(entity.value, Value::List(_))) {
                let parts = components(self.entities, *id)?;
                if parts.iter().all(|(ty, _)| UNREACHABLE_ALLOWLIST.contains(ty)) {
                    ignored.extend(parts.into_iter().map(|(ty, _)| ty.to_string()));
                    continue;
                }
            }
            let (ty, _) = call(self.entities, *id)?;
            if self.allow_degenerate
                && (matches!(ty, "NEXT_ASSEMBLY_USAGE_OCCURRENCE" | "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION")
                    || EXTERNAL_REFERENCE_TYPES.contains(&ty))
            {
                continue;
            }
            if UNREACHABLE_ALLOWLIST.contains(&ty) {
                ignored.insert(ty.to_string());
            } else {
                return Err(refuse(format!("Unreachable entity {ty} is not presentation metadata allowlisted by /3")));
            }
        }
        Ok(ignored)
    }

    fn build_occurrence_model(&self, occurrences: Vec<RepresentationOccurrence>) -> Result<(Model, TopologyMapping)> {
        if occurrences.is_empty() {
            return Err(refuse("Selected placement graph contains no manifold B-rep occurrence"));
        }
        let mut aggregate = Model::empty(DEFAULT_STEP_TOLERANCE_MM)?;
        let mut aggregate_tolerance = None::<f64>;
        let mut aggregate_map = Vec::new();
        for occurrence in occurrences {
            let scale = length_scale(self.entities, &occurrence.linked, false)?;
            let tolerance_mm = uncertainty_mm(self.entities, &occurrence.linked)?.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
            aggregate_tolerance = Some(aggregate_tolerance.map_or(tolerance_mm, |current| current.min(tolerance_mm)));
            let (mut component, map) = build_direct_roots(self.entities, &occurrence.bodies, scale, self.build_options(true))?;
            component.tolerance_mm = tolerance_mm;
            let component = brep_core::transform::affine(&component, occurrence.transform)?;
            append_direct_model(&mut aggregate, component, &map, &mut aggregate_map);
        }
        aggregate.tolerance_mm = aggregate_tolerance.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        aggregate.rebuild_topology_ids();
        Ok((aggregate, aggregate_map))
    }

    fn build_root_model(&self, linked: &BTreeSet<usize>, roots: &[usize]) -> Result<(Model, TopologyMapping)> {
        let scale = length_scale(self.entities, linked, true)?;
        let tolerance_mm = uncertainty_mm(self.entities, linked)?.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        let placement = reachable_transform(self.entities, linked, scale, false)?;
        let (mut model, map) = build_direct_roots(self.entities, roots, scale, self.build_options(false))?;
        model.tolerance_mm = tolerance_mm;
        if let Some(matrix) = placement {
            model = brep_core::transform::affine(&model, matrix)?
        }
        Ok((model, map))
    }

    fn identity_report(
        &self,
        occurrence_count: usize,
        identity_map: &TopologyMapping,
        model: &mut Model,
    ) -> Result<crate::step_interchange::StepIdentityReport> {
        if occurrence_count > 1 && (self.is_v9_or_later() || self.allow_degenerate) {
            return Ok(crate::step_interchange::StepIdentityReport {
                preserved: false,
                source: "assembly-definition-plus-occurrence",
                preserved_count: 0,
                created_count: identity_map.len(),
                lost_count: 0,
            });
        }
        restore_identity(self.entities, identity_map, model)
    }

    fn metadata_loss(&self, identity_preserved: bool, has_auxiliary: bool) -> Vec<String> {
        let mut metadata_loss: Vec<String> = if identity_preserved {
            vec![]
        } else if self.allow_degenerate {
            vec!["topology.occurrence_instance_ids_created".into(), "presentation_style".into()]
        } else {
            vec![
                "representation_item.name".into(),
                "product.id".into(),
                "product.name".into(),
                "product.description".into(),
                "presentation_style".into(),
            ]
        };
        if has_auxiliary {
            metadata_loss.push("projection.non_exact_representations_retained_as_metadata".into());
        }
        metadata_loss
    }

    fn certificate_notes(&self) -> Vec<&'static str> {
        let mut notes = BASE_CERTIFICATE_NOTES.to_vec();
        if self.whole_domain_proofs() {
            notes.push("whole_domain_rational_boundary_and_constant_pole_identity");
        }
        if self.is_v9_or_later() {
            notes.extend([
                "certified_trimmed_curve_point_inverse",
                "bounded_freeform_collapsed_boundary_ownership",
                "invertible_general_affine_occurrences",
                "open_shell_surface_products",
            ]);
        }
        notes
    }

    fn semantic_metadata(&self, identity_map: &TopologyMapping) -> Result<StepSemanticMetadata> {
        let xde = step_xde::parse(self.entities, identity_map)?;
        let presentation = step_presentation::parse(self.entities, identity_map, &xde.body_products, xde.product_parents.clone())?;
        Ok(StepSemanticMetadata { presentation, xde })
    }
}

fn product_hierarchy_entry(id: usize, ty: &str, args: &[Value]) -> Option<String> {
    if ty == "PRODUCT" && args.len() >= 3 {
        Some(format!(
            "product:#{}:{}:{}",
            id,
            string_value(&args[0], "product id").ok()?,
            string_value(&args[1], "product name").ok()?
        ))
    } else if matches!(ty, "PRODUCT_DEFINITION" | "PRODUCT_DEFINITION_FORMATION" | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE")
        && args.len() >= 2
    {
        Some(format!(
            "{}:#{}:{}:{}",
            ty.to_ascii_lowercase(),
            id,
            string_value(&args[0], "product metadata id").ok()?,
            string_value(&args[1], "product metadata description").ok()?
        ))
    } else if ty == "PRODUCT_RELATED_PRODUCT_CATEGORY" && args.len() >= 3 {
        Some(format!("product-category:#{}:{}", id, string_value(&args[0], "product category").ok()?))
    } else if ty == "PROPERTY_DEFINITION" && args.len() >= 2 {
        Some(format!("property-definition:#{}:{}", id, string_value(&args[0], "property name").ok()?))
    } else if ty == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" && args.len() >= 5 {
        Some(format!(
            "assembly-use:#{}:#{}:#{}",
            id,
            one_ref(&args[3], "relating product").ok()?,
            one_ref(&args[4], "related product").ok()?
        ))
    } else {
        None
    }
}

type InspectedDirectGraph = (
    Vec<RepresentationOccurrence>, BTreeSet<usize>, Vec<usize>, Vec<String>, bool,
);

fn inspect_direct_graph(route: &DirectImportRoute<'_>) -> Result<InspectedDirectGraph> {
    let mut auxiliary_selections = BTreeSet::new();
    let representations = route.select_representations(&mut auxiliary_selections)?;
    let all_bodies = route.all_bodies();
    let graph_roots = if representations.is_empty() { all_bodies.clone() } else { representations.clone() };
    if graph_roots.is_empty() {
        return Err(refuse("STEP /3 requires a manifold advanced B-rep root"));
    }
    let occurrences = route.occurrences(&representations)?;
    let mut linked = route.linked_graph(&graph_roots, &occurrences)?;
    let roots: Vec<_> = all_bodies.into_iter().filter(|id| linked.contains(id)).collect();
    if roots.is_empty() {
        return Err(refuse("Selected representation contains no manifold B-rep body"));
    }
    route.extend_auxiliary(&representations, &mut auxiliary_selections)?;
    let auxiliary_linked = if auxiliary_selections.is_empty() {
        BTreeSet::new()
    } else {
        reachable(route.entities, &auxiliary_selections.into_iter().collect::<Vec<_>>())?
    };
    linked.retain(|id| !auxiliary_linked.contains(id));
    route.check_reachable_types(&linked)?;
    let ignored = route.ignored_entities(&linked, &auxiliary_linked)?;
    Ok((occurrences, linked, roots, ignored.into_iter().collect(), !auxiliary_linked.is_empty()))
}

/// One placement of a shared STEP definition.
#[derive(Clone, Copy, Debug)]
pub struct StepAssemblyOccurrence {
    pub definition: usize,
    pub transform: [[f64; 4]; 4],
}

/// Validated STEP geometry kept by definition. Standalone bodies are
/// materialized only when requested by a consumer.
pub struct StepSharedAssembly {
    definitions: Vec<Model>,
    occurrences: Vec<StepAssemblyOccurrence>,
}

impl StepSharedAssembly {
    pub fn definitions(&self) -> &[Model] {
        &self.definitions
    }

    pub fn occurrences(&self) -> &[StepAssemblyOccurrence] {
        &self.occurrences
    }

    /// Replace geometry for every occurrence of this definition.
    pub fn replace_definition(&mut self, index: usize, model: Model) -> Result<()> {
        let definition = self.definitions.get_mut(index)
            .ok_or_else(|| refuse("STEP assembly definition index out of range"))?;
        model.validate()?;
        *definition = model;
        Ok(())
    }

    /// Replace one occurrence using geometry in its local coordinates.
    /// A shared definition is split only when other occurrences still use it.
    pub fn replace_occurrence_geometry(&mut self, index: usize, model: Model) -> Result<()> {
        let definition = self.occurrences.get(index)
            .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?
            .definition;
        let shared = self.occurrences.iter().enumerate().any(|(other, occurrence)| {
            other != index && occurrence.definition == definition
        });
        if !shared {
            return self.replace_definition(definition, model);
        }
        model.validate()?;
        let new_definition = self.definitions.len();
        self.definitions.push(model);
        self.occurrences[index].definition = new_definition;
        Ok(())
    }

    /// Replace an occurrence's local-to-assembly rigid placement in millimetres.
    pub fn set_occurrence_placement(&mut self, index: usize, transform: [[f64; 4]; 4]) -> Result<()> {
        let occurrence = self.occurrences.get_mut(index)
            .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
        if !super::is_proper_rigid_placement(transform) {
            return Err(refuse("STEP assembly placement must be a finite proper rigid transform"));
        }
        occurrence.transform = transform;
        Ok(())
    }

    /// Place an existing definition again. Returns the new occurrence index.
    pub fn add_occurrence(&mut self, definition: usize, transform: [[f64; 4]; 4]) -> Result<usize> {
        if definition >= self.definitions.len() {
            return Err(refuse("STEP assembly definition index out of range"));
        }
        if self.occurrences.len() >= super::MAX_OCCURRENCES {
            return Err(refuse("STEP assembly exceeds the limit of 256 occurrences"));
        }
        if !super::is_proper_rigid_placement(transform) {
            return Err(refuse("STEP assembly placement must be a finite proper rigid transform"));
        }
        let index = self.occurrences.len();
        self.occurrences.push(StepAssemblyOccurrence { definition, transform });
        Ok(index)
    }

    /// Remove one placement and discard its definition when it becomes unused.
    /// Indices after the removed occurrence or definition are compacted.
    pub fn remove_occurrence(&mut self, index: usize) -> Result<()> {
        let definition = self.occurrences.get(index)
            .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?
            .definition;
        if self.occurrences.len() == 1 {
            return Err(refuse("STEP assembly requires at least one occurrence"));
        }
        self.occurrences.remove(index);
        if !self.occurrences.iter().any(|occurrence| occurrence.definition == definition) {
            self.definitions.remove(definition);
            for occurrence in &mut self.occurrences {
                if occurrence.definition > definition {
                    occurrence.definition -= 1;
                }
            }
        }
        Ok(())
    }

    pub fn materialize(&self, index: usize) -> Result<Model> {
        let occurrence = self.occurrences.get(index)
            .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
        brep_core::transform::affine(&self.definitions[occurrence.definition], occurrence.transform)
    }
}

/// Import an AP242 assembly without expanding all of its placements into
/// a single B-rep. Each distinct definition is built and validated once.
pub fn import_step_shared_assembly(text: &str) -> Result<StepSharedAssembly> {
    validate_exchange_structure(text)?;
    let entities = parse(data_payload(text)?)?;
    let route = DirectImportRoute {
        entities: &entities, capability: STEP_INTERCHANGE_V9_CAPABILITY,
        analytic_surfaces: true, ap242_composition: true, allow_degenerate: true,
    };
    let (placements, _, _, _, _) = inspect_direct_graph(&route)?;
    if placements.is_empty() {
        return Err(refuse("Selected placement graph contains no manifold B-rep occurrence"));
    }
    let mut definition_indices = BTreeMap::new();
    let mut definitions = Vec::new();
    let mut occurrences = Vec::with_capacity(placements.len());
    for placement in placements {
        let scale = length_scale(&entities, &placement.linked, false)?;
        let tolerance = uncertainty_mm(&entities, &placement.linked)?.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        let key = (placement.representation, placement.bodies, scale.to_bits(), tolerance.to_bits());
        let definition = match definition_indices.get(&key) {
            Some(&index) => index,
            None => {
                let (mut model, identity) = build_direct_roots(
                    &entities, &key.1, scale, route.build_options(true),
                )?;
                model.tolerance_mm = tolerance;
                restore_identity(&entities, &identity, &mut model)?;
                model.validate()?;
                let index = definitions.len();
                definitions.push(model);
                definition_indices.insert(key, index);
                index
            }
        };
        occurrences.push(StepAssemblyOccurrence { definition, transform: placement.transform });
    }
    Ok(StepSharedAssembly { definitions, occurrences })
}

fn import_step_direct_impl(
    text: &str,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
    include_semantic: bool,
) -> Result<DirectImport> {
    if !text.to_ascii_uppercase().contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 exchange"));
    }
    if ap242_composition {
        validate_exchange_structure(text)?;
    }
    let entities = parse(if ap242_composition { data_payload(text)? } else { text })?;
    let route = DirectImportRoute { entities: &entities, capability, analytic_surfaces, ap242_composition, allow_degenerate };
    let (occurrences, linked, roots, ignored, has_auxiliary) = inspect_direct_graph(&route)?;
    let occurrence_count = occurrences.len();
    let (definition_identities, occurrence_identities) = route.occurrence_identities(&occurrences)?;
    let product_hierarchy = route.product_hierarchy();
    let external_references = route.external_references();
    let (mut model, identity_map) = if ap242_composition {
        route.build_occurrence_model(occurrences)?
    } else {
        route.build_root_model(&linked, &roots)?
    };
    let identity = route.identity_report(occurrence_count, &identity_map, &mut model)?;
    model.validate()?;
    let metadata_loss = route.metadata_loss(identity.preserved, has_auxiliary);
    let mut certificate_notes = route.certificate_notes();
    let semantic = include_semantic.then(|| route.semantic_metadata(&identity_map)).transpose()?;
    if semantic.is_some() {
        certificate_notes.extend(["ap214_face_body_product_presentation", "typed_xde_product_attributes"]);
    }
    Ok((
        model,
        FeatureCertificate { capability, complete: true, notes: certificate_notes },
        StepV3Report {
            identity,
            ignored_entities: ignored.into_iter().collect(),
            metadata_loss,
            instance_count: entities.len(),
            reachable_count: linked.len(),
            definition_identities,
            occurrence_identities,
            product_hierarchy,
            external_references,
        },
        semantic,
    ))
}

pub(super) fn import_step_direct(
    text: &str, analytic_surfaces: bool, ap242_composition: bool, allow_degenerate: bool, capability: &'static str,
) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    let (model, certificate, report, _) = import_step_direct_impl(text, analytic_surfaces, ap242_composition, allow_degenerate, capability, false)?;
    Ok((model, certificate, report))
}

pub fn import_step_v3(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}

pub fn import_step_v3_with_metadata(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report, StepSemanticMetadata)> {
    let (model, certificate, report, semantic) = import_step_direct_impl(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY, true)?;
    Ok((model, certificate, report, semantic.expect("metadata=true always returns semantic metadata")))
}

pub fn import_step_v4(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, false, false, STEP_INTERCHANGE_V4_CAPABILITY)
}

pub fn import_step_v5(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}

pub fn import_step_v6(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V6_CAPABILITY)
}

pub fn import_step_v7(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V7_CAPABILITY)
}

pub fn import_step_v8(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) = import_step_direct(text, true, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    let certificate = certify_step_v8_topology(&model)?;
    Ok((model, certificate, report))
}

pub fn import_step_v9(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) = import_step_direct(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors", "bounded_freeform_poles",
        "general_affine_occurrences", "retained_open_shell_products",
    ]);
    Ok((model, certificate, report))
}

pub fn import_step_v9_with_metadata(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report, StepSemanticMetadata)> {
    let (model, _, report, semantic) = import_step_direct_impl(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY, true)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors", "bounded_freeform_poles",
        "general_affine_occurrences", "retained_open_shell_products",
        "ap214_face_body_product_presentation", "typed_xde_product_attributes",
    ]);
    Ok((model, certificate, report, semantic.expect("metadata=true always returns semantic metadata")))
}

fn step_v10_graph_identity(entities: &BTreeMap<usize, Entity>) -> Result<String> {
    let mut nodes = entities.keys().map(|id| entity_digest(entities, *id)).collect::<Result<Vec<_>>>()?;
    nodes.sort();
    Ok(hash(&nodes.join("\n")))
}

pub fn import_step_v10(text: &str) -> Result<(Model, StepV8Certificate, StepV10Document)> {
    let (model, _, report) = import_step_direct(text, true, true, true, STEP_INTERCHANGE_V10_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.notes.extend([
        "certified_interior_point_selectors", "bounded_freeform_poles",
        "general_affine_occurrences", "retained_open_shell_products",
    ]);
    let entities = parse(data_payload(text)?)?;
    let mut operator_identities = Vec::new();
    for id in entities.keys() {
        if let Some((_, _, operator)) = relationship_components(&entities, *id)? && operator != 0 {
            operator_identities.push(format!("operator:{}", entity_digest(&entities, operator)?));
        }
    }
    operator_identities.sort();
    operator_identities.dedup();
    certificate.capability = STEP_INTERCHANGE_V10_CAPABILITY;
    certificate.notes.extend(["retained_affine_occurrence_graph", "exact_graph_isomorphism_identity"]);
    let document = StepV10Document {
        source: text.to_string(),
        graph_identity: step_v10_graph_identity(&entities)?,
        definition_identities: report.definition_identities,
        occurrence_identities: report.occurrence_identities,
        product_hierarchy: report.product_hierarchy,
        operator_identities,
        metadata_loss: report.metadata_loss,
    };
    Ok((model, certificate, document))
}

pub fn export_step_v10(document: &StepV10Document) -> Result<String> {
    if document.source.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP /10 output exceeds 16 MiB"));
    }
    let entities = parse(data_payload(&document.source)?)?;
    if step_v10_graph_identity(&entities)? != document.graph_identity {
        return Err(refuse("STEP /10 retained graph identity mismatch"));
    }
    Ok(document.source.clone())
}
