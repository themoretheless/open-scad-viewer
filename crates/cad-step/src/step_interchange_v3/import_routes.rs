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
        let mut repeats = BTreeMap::new();
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
            let key = path + &matrix;
            let ordinal = repeats.entry(hash(&key)).or_insert(0usize);
            let identity = if *ordinal == 0 {
                hash(&key)
            } else {
                hash(&format!("{key}/instance-{ordinal}"))
            };
            *ordinal += 1;
            occurrence_identities.push(format!("occurrence:{identity}"));
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
        let has_coincident_occurrences = occurrences.iter().any(|occurrence| occurrence.coincident);
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
        if has_coincident_occurrences {
            assign_occurrence_topology_ids(&mut aggregate, self.capability);
        }
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

/// Directional changes of material in assembly coordinates.
pub struct StepMaterialDelta {
    pub added: Model,
    pub removed: Model,
}

impl StepMaterialDelta {
    fn between(before: &Model, after: &Model) -> Result<Self> {
        Ok(Self {
            added: brep_core::boolean(after, before, "difference")?,
            removed: brep_core::boolean(before, after, "difference")?,
        })
    }
}

/// Inner and outer unions of positive-length material spans in ray parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct StepRayMaterialCoverage {
    pub guaranteed: Vec<[f64; 2]>,
    pub possible: Vec<[f64; 2]>,
}

/// Classification against a caller-supplied minimum material clearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepClearanceStatus {
    BelowMinimum,
    MeetsMinimum,
    Unresolved,
}

pub struct StepClearanceCheck {
    pub status: StepClearanceStatus,
    pub measurement: brep_core::solid_distance::Report,
}

/// Conservative bounds on the smallest distance between distinct occurrences.
#[derive(Debug)]
pub struct StepMinimumClearance {
    pub lower_bound_mm: f64,
    pub upper_bound_mm: Option<f64>,
    /// Pair furnishing the upper bound, which may not be the unique closest pair.
    pub upper_bound_pair: Option<[usize; 2]>,
    pub checked_pairs: usize,
    pub converged: bool,
    pub pair_errors: Vec<([usize; 2], Error)>,
}

impl StepSharedAssembly {
    /// Build a shared assembly directly from authored local solids and placements.
    pub fn from_placed_solids(definitions: Vec<Model>, occurrences: Vec<StepAssemblyOccurrence>) -> Result<Self> {
        if occurrences.is_empty() || occurrences.len() > super::MAX_OCCURRENCES
            || definitions.is_empty() || definitions.len() > occurrences.len()
        {
            return Err(refuse("STEP assembly requires 1 to 256 placed solids"));
        }
        let mut used = vec![false; definitions.len()];
        for occurrence in &occurrences {
            let slot = used.get_mut(occurrence.definition)
                .ok_or_else(|| refuse("STEP assembly definition index out of range"))?;
            if super::orthogonal_placement_scales(occurrence.transform).is_none() {
                return Err(refuse("STEP assembly requires finite nonsingular orthogonal placements"));
            }
            *slot = true;
        }
        if used.contains(&false) {
            return Err(refuse("STEP assembly contains an unplaced definition"));
        }
        for model in &definitions {
            if model.bodies.len() != 1 || model.shells.iter().any(|shell| !shell.closed) {
                return Err(refuse("STEP assembly requires one closed solid per definition"));
            }
            model.validate()?;
        }
        Ok(Self { definitions, occurrences })
    }

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
        self.replace_variant(&[index], model).map(|_| ())
    }

    /// Give selected occurrences of one prototype a common local-space revision.
    /// Reuse the definition when all its occurrences are selected.
    pub fn replace_variant(&mut self, selection: &[usize], model: Model) -> Result<usize> {
        let first = selection.first().and_then(|&i| self.occurrences.get(i))
            .ok_or_else(|| refuse("STEP variant requires a valid occurrence selection"))?;
        let definition = first.definition;
        let mut seen = vec![false; self.occurrences.len()];
        for &index in selection {
            let occurrence = self.occurrences.get(index)
                .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
            if seen[index] || occurrence.definition != definition {
                return Err(refuse("STEP variant requires distinct occurrences of one definition"));
            }
            seen[index] = true;
        }
        if selection.len() == self.occurrences.iter().filter(|o| o.definition == definition).count() {
            self.replace_definition(definition, model)?;
            return Ok(definition);
        }
        model.validate()?;
        let variant = self.definitions.len();
        self.definitions.push(model);
        for &index in selection {
            self.occurrences[index].definition = variant;
        }
        Ok(variant)
    }

    /// Apply one assembly-space rigid motion to selected occurrences, atomically.
    /// Preserve existing scales and reflections and keep local definitions shared.
    pub fn move_occurrences(&mut self, selection: &[usize], motion: [[f64; 4]; 4]) -> Result<()> {
        let moved = self.transformed_selection(selection, motion)?;
        for (&index, occurrence) in selection.iter().zip(moved) {
            self.occurrences[index] = occurrence;
        }
        Ok(())
    }

    /// Place an additional linked copy of each selected occurrence in selection order.
    /// Return the contiguous indices of the new occurrences.
    pub fn copy_occurrences(&mut self, selection: &[usize], motion: [[f64; 4]; 4])
        -> Result<std::ops::Range<usize>> {
        let start = self.occurrences.len();
        if selection.len() > super::MAX_OCCURRENCES - start {
            return Err(refuse("STEP group copy exceeds the occurrence limit"));
        }
        let copies = self.transformed_selection(selection, motion)?;
        self.occurrences.extend(copies);
        Ok(start..self.occurrences.len())
    }

    fn transformed_selection(&self, selection: &[usize], motion: [[f64; 4]; 4])
        -> Result<Vec<StepAssemblyOccurrence>> {
        if selection.is_empty() || selection.len() > self.occurrences.len()
            || !super::is_proper_rigid_placement(motion) {
            return Err(refuse("STEP group move requires a nonempty selection and a finite rigid motion"));
        }
        let mut seen = vec![false; self.occurrences.len()];
        let mut updates = Vec::with_capacity(selection.len());
        for &index in selection {
            let occurrence = self.occurrences.get(index)
                .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
            if seen[index] { return Err(refuse("STEP group move contains a duplicate occurrence")); }
            seen[index] = true;
            let transform = mul(motion, occurrence.transform);
            if super::orthogonal_placement_scales(transform).is_none() {
                return Err(refuse("STEP group move produced an invalid placement"));
            }
            updates.push(StepAssemblyOccurrence { definition: occurrence.definition, transform });
        }
        Ok(updates)
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

    /// Set a local-to-assembly placement with perpendicular axes, including
    /// independent axis scales and reflection.
    pub fn set_occurrence_orthogonal_placement(
        &mut self,
        index: usize,
        transform: [[f64; 4]; 4],
    ) -> Result<()> {
        let occurrence = self.occurrences.get_mut(index)
            .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
        if super::orthogonal_placement_scales(transform).is_none() {
            return Err(refuse("STEP assembly placement must be finite, orthogonal and nonsingular"));
        }
        occurrence.transform = transform;
        Ok(())
    }

    /// Place an existing definition again. Returns the new occurrence index.
    pub fn add_occurrence(&mut self, definition: usize, transform: [[f64; 4]; 4]) -> Result<usize> {
        if definition >= self.definitions.len() {
            return Err(refuse("STEP assembly definition index out of range"));
        }
        self.check_new_placement(transform)?;
        Ok(self.push_occurrence(definition, transform))
    }

    /// Insert a closed solid in local coordinates and place its first occurrence.
    pub fn insert_solid(&mut self, model: Model, transform: [[f64; 4]; 4]) -> Result<usize> {
        self.check_new_placement(transform)?;
        if model.bodies.len() != 1 || model.shells.iter().any(|shell| !shell.closed) {
            return Err(refuse("STEP assembly insertion requires one closed solid"));
        }
        model.validate()?;
        let definition = self.definitions.len();
        self.definitions.push(model);
        Ok(self.push_occurrence(definition, transform))
    }

    /// Append a shared assembly under a rigid placement, flattening its hierarchy
    /// while retaining the definition sharing within the appended assembly.
    pub fn append_assembly(&mut self, source: Self, placement: [[f64; 4]; 4]) -> Result<()> {
        if self.occurrences.len() + source.occurrences.len() > super::MAX_OCCURRENCES {
            return Err(refuse("STEP assembly exceeds the limit of 256 occurrences"));
        }
        if !super::is_proper_rigid_placement(placement) {
            return Err(refuse("STEP assembly placement must be a finite proper rigid transform"));
        }
        let offset = self.definitions.len();
        let occurrences = source.occurrences.into_iter().map(|occurrence| {
            let transform = mul(placement, occurrence.transform);
            if !super::is_proper_rigid_placement(transform) {
                return Err(refuse("STEP appended occurrences must have finite proper rigid placements"));
            }
            Ok(StepAssemblyOccurrence { definition: offset + occurrence.definition, transform })
        }).collect::<Result<Vec<_>>>()?;
        self.definitions.extend(source.definitions);
        self.occurrences.extend(occurrences);
        Ok(())
    }

    /// Coalesce identical indexed B-rep payloads, including the model tolerance.
    /// The first definition supplies the retained topology identities.
    pub fn deduplicate_definitions(&mut self) -> usize {
        let original_count = self.definitions.len();
        let mut unique: Vec<Model> = Vec::with_capacity(original_count);
        let mut remap = Vec::with_capacity(original_count);
        for model in std::mem::take(&mut self.definitions) {
            let index = unique.iter().position(|candidate| candidate.0 == model.0)
                .unwrap_or_else(|| {
                    let index = unique.len();
                    unique.push(model);
                    index
                });
            remap.push(index);
        }
        for occurrence in &mut self.occurrences {
            occurrence.definition = remap[occurrence.definition];
        }
        self.definitions = unique;
        original_count - self.definitions.len()
    }

    /// Copy selected occurrences, in the requested order, with only the
    /// definitions they reference and with their assembly-space placements.
    pub fn extract_occurrences(&self, indices: &[usize]) -> Result<Self> {
        if indices.is_empty() || indices.len() > self.occurrences.len() {
            return Err(refuse("STEP assembly extraction requires a nonempty distinct selection"));
        }
        let mut seen = BTreeSet::new();
        let mut remap = BTreeMap::new();
        let mut definitions = Vec::new();
        let mut occurrences = Vec::with_capacity(indices.len());
        for &index in indices {
            let occurrence = self.occurrences.get(index)
                .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
            if !seen.insert(index) {
                return Err(refuse("STEP assembly extraction contains a duplicate occurrence"));
            }
            let definition = *remap.entry(occurrence.definition).or_insert_with(|| {
                let index = definitions.len();
                definitions.push(self.definitions[occurrence.definition].clone());
                index
            });
            occurrences.push(StepAssemblyOccurrence { definition, transform: occurrence.transform });
        }
        Ok(Self { definitions, occurrences })
    }

    fn check_new_placement(&self, transform: [[f64; 4]; 4]) -> Result<()> {
        if self.occurrences.len() >= super::MAX_OCCURRENCES {
            return Err(refuse("STEP assembly exceeds the limit of 256 occurrences"));
        }
        if !super::is_proper_rigid_placement(transform) {
            return Err(refuse("STEP assembly placement must be a finite proper rigid transform"));
        }
        Ok(())
    }

    fn push_occurrence(&mut self, definition: usize, transform: [[f64; 4]; 4]) -> usize {
        let index = self.occurrences.len();
        self.occurrences.push(StepAssemblyOccurrence { definition, transform });
        index
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

    /// Conservative assembly-space boxes, in occurrence order, in millimetres.
    /// A definition's control hull is scanned once even when repeatedly placed.
    /// Bounds may include portions of a surface outside its trimmed region.
    pub fn occurrence_bounds(&self) -> Result<Vec<[[f64; 3]; 2]>> {
        let local: Vec<_> = self.definitions.iter().map(definition_control_bounds).collect();
        self.occurrences.iter().map(|occurrence| {
            let [min, max] = local[occurrence.definition];
            let mut placed = [[0.; 3]; 2];
            for (axis, row) in occurrence.transform[..3].iter().enumerate() {
                let (mut lower, mut upper) = (row[3], row[3]);
                for (column, &coefficient) in row[..3].iter().enumerate() {
                    if coefficient == 0. {
                        continue;
                    }
                    let (lo, hi) = if coefficient > 0. {
                        (min[column], max[column])
                    } else {
                        (max[column], min[column])
                    };
                    lower = (lower + (coefficient * lo).next_down()).next_down();
                    upper = (upper + (coefficient * hi).next_up()).next_up();
                }
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(refuse("STEP occurrence bounds are empty or exceed finite coordinates"));
                }
                placed[0][axis] = lower;
                placed[1][axis] = upper;
            }
            Ok(placed)
        }).collect()
    }

    /// Return unordered occurrence pairs whose boxes may be within the given
    /// Euclidean distance in millimetres. Touching boxes are included at zero.
    pub fn proximity_candidates(&self, distance_mm: f64) -> Result<Vec<[usize; 2]>> {
        if !distance_mm.is_finite() || distance_mm < 0. {
            return Err(refuse("STEP proximity distance must be finite and nonnegative"));
        }
        let bounds = self.occurrence_bounds()?;
        let limit = (distance_mm * distance_mm).next_up();
        let mut pairs = Vec::new();
        for (index, a) in bounds.iter().enumerate() {
            for (other, b) in bounds.iter().enumerate().skip(index + 1) {
                if box_distance_squared_lower(a, b) <= limit {
                    pairs.push([index, other]);
                }
            }
        }
        Ok(pairs)
    }

    /// Build the regularized common material of two placed occurrences.
    /// Unsupported cases propagate the B-rep kernel's diagnostic.
    pub fn intersect_occurrences(&self, first: usize, second: usize) -> Result<Model> {
        let first = self.materialize(first)?;
        let second = self.materialize(second)?;
        brep_core::boolean(&first, &second, "intersection")
    }

    /// Clip placed material to a finite inspection box in assembly coordinates.
    /// Preserve occurrence indices and report unresolved candidates individually.
    pub fn material_in_box(&self, min: [f64; 3], max: [f64; 3], max_candidates: usize)
        -> Result<Vec<(usize, Result<Model>)>> {
        let candidates = self.box_candidates(min, max, max_candidates)?;
        let clip = brep_core::cuboid(min, max)?;
        let mut hits = Vec::new();
        for (index, [lo, hi]) in candidates {
            let inside = (0..3).all(|i| min[i] <= lo[i] && hi[i] <= max[i]);
            let result = self.materialize(index).and_then(|model| {
                if inside { Ok(model) } else { brep_core::boolean(&model, &clip, "intersection") }
            });
            if !matches!(&result, Ok(model) if model.bodies.is_empty()) {
                hits.push((index, result));
            }
        }
        Ok(hits)
    }

    /// Return the regularized free volume inside a finite assembly-space box.
    pub fn free_space_in_box(&self, min: [f64; 3], max: [f64; 3], max_candidates: usize) -> Result<Model> {
        let candidates = self.box_candidates(min, max, max_candidates)?;
        let mut free = brep_core::cuboid(min, max)?;
        for (index, _) in candidates {
            if free.bodies.is_empty() { break; }
            free = brep_core::boolean(&free, &self.materialize(index)?, "difference")?;
        }
        Ok(free)
    }

    fn box_candidates(&self, min: [f64; 3], max: [f64; 3], max_candidates: usize)
        -> Result<Vec<(usize, [[f64; 3]; 2])>> {
        if (0..3).any(|i| !min[i].is_finite() || !max[i].is_finite() || min[i] >= max[i]) {
            return Err(refuse("STEP inspection box requires finite ordered limits"));
        }
        let bounds = self.occurrence_bounds()?;
        let candidates: Vec<_> = bounds.into_iter().enumerate().filter(|(_, [lo, hi])| {
            (0..3).all(|i| lo[i] < max[i] && min[i] < hi[i])
        }).collect();
        if candidates.len() > max_candidates {
            return Err(refuse("STEP inspection box candidate budget exceeded"));
        }
        Ok(candidates)
    }

    /// Split at a world X, Y or Z coordinate. Return the low and high material respectively.
    pub fn split_at_coordinate(&self, index: usize, axis: usize, coordinate: f64) -> Result<[Model; 2]> {
        if axis >= 3 || !coordinate.is_finite() {
            return Err(refuse("STEP coordinate split requires axis 0..2 and a finite coordinate"));
        }
        split_placed_model(self.materialize(index)?, axis, coordinate)
    }

    /// Return a solid for each interval between ordered parallel cuts, including both end intervals.
    pub fn coordinate_layers(&self, index: usize, axis: usize, cuts: &[f64]) -> Result<Vec<Model>> {
        if axis >= 3 || cuts.len() > 256 || cuts.iter().any(|x| !x.is_finite())
            || cuts.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(refuse("STEP layers require an axis and up to 256 strictly increasing finite coordinates"));
        }
        let mut remaining = self.materialize(index)?;
        let mut layers = Vec::with_capacity(cuts.len() + 1);
        for &cut in cuts {
            if remaining.bodies.is_empty() {
                layers.push(Model::empty(remaining.tolerance_mm)?);
            } else {
                let [low, high] = split_placed_model(remaining, axis, cut)?;
                layers.push(low);
                remaining = high;
            }
        }
        layers.push(remaining);
        Ok(layers)
    }

    /// Compare two placed versions of a solid, preserving both directions of change.
    pub fn material_delta(&self, before: usize, after: usize) -> Result<StepMaterialDelta> {
        let before = self.materialize(before)?;
        let after = self.materialize(after)?;
        StepMaterialDelta::between(&before, &after)
    }

    /// Fuse the selected occurrences in assembly coordinates, retaining disjoint material.
    /// Repeated indices select the same material once.
    pub fn fuse_occurrences(&self, selection: &[usize]) -> Result<Model> {
        if selection.is_empty() || selection.len() > super::MAX_OCCURRENCES
            || selection.iter().any(|&index| index >= self.occurrences.len()) {
            return Err(refuse("STEP fusion requires 1 to 256 valid occurrence indices"));
        }
        let mut fused = self.materialize(selection[0])?;
        let mut used = vec![false; self.occurrences.len()];
        used[selection[0]] = true;
        for &index in &selection[1..] {
            if !used[index] {
                fused = brep_core::boolean(&fused, &self.materialize(index)?, "union")?;
                used[index] = true;
            }
        }
        Ok(fused)
    }

    /// Compare a stock blank with the combined material of a layout of finished parts.
    /// Added material is missing from the blank; removed material is machining allowance.
    pub fn stock_layout_delta(&self, stock: usize, parts: &[usize]) -> Result<StepMaterialDelta> {
        let blank = self.materialize(stock)?;
        let required = self.fuse_occurrences(parts)?;
        StepMaterialDelta::between(&blank, &required)
    }

    /// Cut a placed stock solid with the selected occurrences, in assembly coordinates.
    /// Duplicate cutters are applied once. The shared source assembly stays editable.
    pub fn cut_stock(&self, stock: usize, cutters: &[usize]) -> Result<Model> {
        if cutters.len() > super::MAX_OCCURRENCES || cutters.iter().any(|&i| i >= self.occurrences.len()) {
            return Err(refuse("STEP stock cutting requires at most 256 valid cutter occurrences"));
        }
        let mut remaining = self.materialize(stock)?;
        let mut applied = vec![false; self.occurrences.len()];
        for &index in cutters {
            if remaining.bodies.is_empty() { break; }
            if !applied[index] {
                remaining = brep_core::boolean(&remaining, &self.materialize(index)?, "difference")?;
                applied[index] = true;
            }
        }
        Ok(remaining)
    }

    /// Bound the distance between the filled materials of two placed
    /// occurrences, retaining unresolved solid and containment diagnostics.
    pub fn occurrence_clearance(
        &self,
        pair: [usize; 2],
        tolerance_mm: f64,
        tolerance_uv: f64,
        limits: brep_core::solid_distance::Limits,
    ) -> Result<brep_core::solid_distance::Report> {
        let first = self.materialize(pair[0])?;
        let second = self.materialize(pair[1])?;
        brep_core::solid_distance::distance(&first, &second, tolerance_mm, tolerance_uv, limits)
    }

    /// Bound the smallest material clearance without evaluating pairs whose
    /// box separation is already larger than the best measured upper bound.
    /// None indicates an assembly with fewer than two occurrences.
    pub fn minimum_clearance(
        &self,
        tolerance_mm: f64,
        tolerance_uv: f64,
        limits: brep_core::solid_distance::Limits,
        max_pairs: usize,
    ) -> Result<Option<StepMinimumClearance>> {
        self.minimum_clearance_filtered(tolerance_mm, tolerance_uv, limits, max_pairs, |a, b| Some([a, b]))
    }

    /// Bound the smallest clearance across two disjoint groups, retaining the left/right pair identity.
    pub fn minimum_clearance_between(&self, left: &[usize], right: &[usize],
        tolerance_mm: f64, tolerance_uv: f64, limits: brep_core::solid_distance::Limits,
        max_pairs: usize) -> Result<StepMinimumClearance> {
        let sides = self.selection_sides(left, right)?;
        self.minimum_clearance_filtered(tolerance_mm, tolerance_uv, limits, max_pairs, |a, b| {
            match (sides[a], sides[b]) {
                (1, 2) => Some([a, b]),
                (2, 1) => Some([b, a]),
                _ => None,
            }
        })?.ok_or_else(|| refuse("STEP interface requires a cross-group pair"))
    }

    fn minimum_clearance_filtered(&self, tolerance_mm: f64, tolerance_uv: f64,
        limits: brep_core::solid_distance::Limits, max_pairs: usize,
        select: impl Fn(usize, usize) -> Option<[usize; 2]>) -> Result<Option<StepMinimumClearance>> {
        if !tolerance_mm.is_finite() || tolerance_mm <= 0.
            || !tolerance_uv.is_finite() || tolerance_uv <= 0.
        {
            return Err(refuse("STEP clearance requires positive finite tolerances"));
        }
        let bounds = self.occurrence_bounds()?;
        let mut pairs = Vec::new();
        for (first, a) in bounds.iter().enumerate() {
            for (second, b) in bounds.iter().enumerate().skip(first + 1) {
                let Some(pair) = select(first, second) else { continue; };
                let lower = box_distance_squared_lower(a, b).sqrt().next_down().max(0.);
                pairs.push((pair, lower));
            }
        }
        if pairs.is_empty() {
            return Ok(None);
        }
        pairs.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut result = StepMinimumClearance {
            lower_bound_mm: f64::INFINITY,
            upper_bound_mm: None,
            upper_bound_pair: None,
            checked_pairs: 0,
            converged: false,
            pair_errors: Vec::new(),
        };
        for (pair, box_lower) in pairs {
            if result.upper_bound_mm.is_some_and(|upper| box_lower > upper) {
                break;
            }
            if result.checked_pairs == max_pairs {
                result.lower_bound_mm = result.lower_bound_mm.min(box_lower);
                break;
            }
            result.checked_pairs += 1;
            let mut pair_lower = box_lower;
            match self.occurrence_clearance(pair, tolerance_mm, tolerance_uv, limits) {
                Ok(report) => {
                    if let Some([lower, upper]) = report.distance_interval_mm {
                        pair_lower = pair_lower.max(lower);
                        if upper.is_finite() && result.upper_bound_mm.is_none_or(|best| upper < best) {
                            result.upper_bound_mm = Some(upper);
                            result.upper_bound_pair = Some(pair);
                        }
                    }
                }
                Err(error) => result.pair_errors.push((pair, error)),
            }
            result.lower_bound_mm = result.lower_bound_mm.min(pair_lower);
        }
        result.converged = result.upper_bound_mm.is_some_and(|upper| {
            upper == result.lower_bound_mm
                || (upper - result.lower_bound_mm).next_up() <= tolerance_mm
        });
        Ok(Some(result))
    }

    /// Check neighboring pairs against a minimum clearance in millimetres.
    /// Pairs absent from the report are excluded conservatively by their boxes.
    /// An unresolved measurement or pair error is retained, not treated as a pass.
    pub fn check_clearances(
        &self,
        minimum_mm: f64,
        tolerance_mm: f64,
        tolerance_uv: f64,
        limits: brep_core::solid_distance::Limits,
        max_pairs: usize,
    ) -> Result<Vec<([usize; 2], Result<StepClearanceCheck>)>> {
        let pairs = self.proximity_candidates(minimum_mm)?;
        if pairs.len() > max_pairs {
            return Err(refuse("STEP assembly clearance candidate budget exceeded"));
        }
        self.measure_clearance_pairs(pairs, minimum_mm, tolerance_mm, tolerance_uv, limits)
    }

    /// Measure candidate clearances across a selected assembly interface.
    pub fn clearances_between(&self, left: &[usize], right: &[usize], minimum_mm: f64,
        tolerance_mm: f64, tolerance_uv: f64, limits: brep_core::solid_distance::Limits, max_pairs: usize)
        -> Result<Vec<([usize; 2], Result<StepClearanceCheck>)>> {
        let pairs = self.cross_group_candidates(left, right, minimum_mm, max_pairs)?;
        self.measure_clearance_pairs(pairs, minimum_mm, tolerance_mm, tolerance_uv, limits)
    }

    fn measure_clearance_pairs(&self, pairs: Vec<[usize; 2]>, minimum_mm: f64,
        tolerance_mm: f64, tolerance_uv: f64, limits: brep_core::solid_distance::Limits)
        -> Result<Vec<([usize; 2], Result<StepClearanceCheck>)>> {
        if !tolerance_mm.is_finite() || tolerance_mm <= 0.
            || !tolerance_uv.is_finite() || tolerance_uv <= 0.
        {
            return Err(refuse("STEP clearance requires positive finite tolerances"));
        }
        Ok(pairs.into_iter().map(|pair| {
            let checked = self.occurrence_clearance(pair, tolerance_mm, tolerance_uv, limits)
                .map(|measurement| {
                    let status = match measurement.distance_interval_mm {
                        Some([lower, _]) if lower >= minimum_mm => StepClearanceStatus::MeetsMinimum,
                        Some([_, upper]) if upper < minimum_mm => StepClearanceStatus::BelowMinimum,
                        _ => StepClearanceStatus::Unresolved,
                    };
                    StepClearanceCheck { status, measurement }
                });
            (pair, checked)
        }).collect())
    }

    /// Check overlapping occurrence boxes for common material. Each result
    /// retains the pair indices and any diagnostic from materialization or CSG.
    /// If there are more candidates than allowed, no CSG work is performed.
    pub fn check_interferences(&self, max_pairs: usize) -> Result<Vec<([usize; 2], Result<bool>)>> {
        let pairs = self.interference_candidates(max_pairs)?;
        Ok(pairs.into_iter().map(|[first, second]| {
            let result = self.intersect_occurrences(first, second)
                .map(|common| !common.bodies.is_empty());
            ([first, second], result)
        }).collect())
    }

    /// Recover pairwise common solids and their integrated mass properties in assembly coordinates.
    /// Touch-only pairs are omitted. Each unresolved candidate retains its diagnostic.
    pub fn interference_regions(&self, max_pairs: usize, relative_tolerance: f64, max_evaluations: usize)
        -> Result<Vec<([usize; 2], Result<(Model, brep_core::analysis::MassProperties)>)>> {
        if !(1e-10..=1e-3).contains(&relative_tolerance) || !(100..=2_000_000).contains(&max_evaluations) {
            return Err(refuse("STEP interference measurement options outside mass integration limits"));
        }
        let pairs = self.interference_candidates(max_pairs)?;
        let mut regions = Vec::new();
        for [first, second] in pairs {
            let region = match self.intersect_occurrences(first, second) {
                Ok(common) if common.bodies.is_empty() => continue,
                Ok(common) => brep_core::analysis::mass_properties(&common, relative_tolerance, max_evaluations)
                    .map(|properties| (common, properties)),
                Err(error) => Err(error),
            };
            regions.push(([first, second], region));
        }
        Ok(regions)
    }

    /// Connected groups of occurrences with positive-volume interference.
    /// Return deterministic groups ordered by first index, omitting isolated occurrences.
    /// An unresolved pair prevents claiming a complete partition.
    pub fn interference_groups(&self, max_pairs: usize) -> Result<Vec<Vec<usize>>> {
        let mut neighbors = vec![Vec::new(); self.occurrences.len()];
        for ([first, second], intersects) in self.check_interferences(max_pairs)? {
            if intersects? {
                neighbors[first].push(second);
                neighbors[second].push(first);
            }
        }
        let mut seen = vec![false; neighbors.len()];
        let mut groups = Vec::new();
        for start in 0..neighbors.len() {
            if seen[start] || neighbors[start].is_empty() { continue; }
            seen[start] = true;
            let mut pending = vec![start];
            let mut group = Vec::new();
            while let Some(current) = pending.pop() {
                group.push(current);
                for &next in &neighbors[current] {
                    if !seen[next] {
                        seen[next] = true;
                        pending.push(next);
                    }
                }
            }
            group.sort_unstable();
            groups.push(group);
        }
        Ok(groups)
    }

    /// Material occupied by at least two occurrences, with multiple overlaps counted once.
    /// Propagate an unresolved pair or union instead of returning a partial solid.
    pub fn interference_material(&self, max_pairs: usize) -> Result<Model> {
        let pairs = self.interference_candidates(max_pairs)?;
        let mut combined: Option<Model> = None;
        for [first, second] in pairs {
            let common = self.intersect_occurrences(first, second)?;
            if !common.bodies.is_empty() {
                combined = Some(match combined {
                    None => common,
                    Some(previous) => brep_core::boolean(&previous, &common, "union")?,
                });
            }
        }
        match combined {
            Some(model) => Ok(model),
            None => Model::empty(self.definitions.iter().map(|model| model.tolerance_mm)
                .fold(DEFAULT_STEP_TOLERANCE_MM, f64::max)),
        }
    }

    /// Check only cross-group contacts. Pair indices follow the left/right selection order.
    /// Return common material for positive overlaps and diagnostics for unresolved pairs.
    pub fn interferences_between(&self, left: &[usize], right: &[usize], max_pairs: usize)
        -> Result<Vec<([usize; 2], Result<Model>)>> {
        let pairs = self.cross_group_candidates(left, right, 0., max_pairs)?;
        let mut regions = Vec::new();
        for [a, b] in pairs {
            let result = self.intersect_occurrences(a, b);
            if !matches!(&result, Ok(model) if model.bodies.is_empty()) {
                regions.push(([a, b], result));
            }
        }
        Ok(regions)
    }

    fn selection_sides(&self, left: &[usize], right: &[usize]) -> Result<Vec<u8>> {
        let mut side = vec![0; self.occurrences.len()];
        for (members, label) in [(left, 1), (right, 2)] {
            if members.is_empty() || members.len() > side.len() {
                return Err(refuse("STEP groups require nonempty occurrence selections"));
            }
            for &index in members {
                let slot = side.get_mut(index).ok_or_else(|| refuse("STEP group index out of range"))?;
                if *slot != 0 { return Err(refuse("STEP selections must be distinct and disjoint")); }
                *slot = label;
            }
        }
        Ok(side)
    }

    fn cross_group_candidates(&self, left: &[usize], right: &[usize], distance: f64, max_pairs: usize)
        -> Result<Vec<[usize; 2]>> {
        let side = self.selection_sides(left, right)?;
        let pairs: Vec<_> = self.proximity_candidates(distance)?.into_iter().filter_map(|[a, b]| {
            match (side[a], side[b]) {
                (1, 2) => Some([a, b]),
                (2, 1) => Some([b, a]),
                _ => None,
            }
        }).collect();
        if pairs.len() > max_pairs { return Err(refuse("STEP cross-group candidate budget exceeded")); }
        Ok(pairs)
    }

    fn interference_candidates(&self, max_pairs: usize) -> Result<Vec<[usize; 2]>> {
        let pairs = self.proximity_candidates(0.)?;
        if pairs.len() > max_pairs {
            return Err(refuse("STEP assembly interference candidate budget exceeded"));
        }
        Ok(pairs)
    }

    /// Estimate total material volume in mm³, counting each occurrence.
    /// Intersecting parts contribute their individual volumes in full.
    /// The integration budget and convergence target apply per definition.
    pub fn material_volume_mm3(
        &self,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<f64> {
        let properties = self.definition_mass_properties(relative_tolerance, max_evaluations_per_definition)?;
        self.occurrences.iter().try_fold(0., |total, occurrence| {
            let volume = placed_material_volume(occurrence, properties[occurrence.definition].signed_volume_mm3)?;
            let total: f64 = total + volume;
            if !total.is_finite() {
                return Err(refuse("STEP assembly volume exceeds the finite range"));
            }
            Ok(total)
        })
    }

    /// Estimate the centroid in assembly coordinates, weighted by each
    /// occurrence's material volume. Overlapping volumes count separately.
    pub fn material_centroid_mm(
        &self,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<[f64; 3]> {
        let properties = self.definition_mass_properties(relative_tolerance, max_evaluations_per_definition)?;
        weighted_centroid(&self.placed_centroids(&properties)?)
    }

    /// Estimate central volume inertia in mm⁵ for uniform unit density,
    /// counting each occurrence independently.
    pub fn material_inertia_mm5(
        &self,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<[[f64; 3]; 3]> {
        self.material_mass_properties(None, relative_tolerance, max_evaluations_per_definition)
            .map(|properties| properties.inertia)
    }

    /// Estimate mass, center of mass and central inertia with one density
    /// per occurrence, in kg/mm³. Outputs are kg, mm and kg·mm².
    pub fn mass_properties_with_densities(
        &self,
        densities_kg_per_mm3: &[f64],
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<brep_core::analysis::AssemblyMass> {
        if densities_kg_per_mm3.len() != self.occurrences.len()
            || densities_kg_per_mm3.iter().any(|value| !value.is_finite() || *value <= 0.)
        {
            return Err(refuse("STEP assembly requires a finite positive density for each occurrence"));
        }
        self.material_mass_properties(Some(densities_kg_per_mm3), relative_tolerance, max_evaluations_per_definition)
    }

    fn material_mass_properties(
        &self,
        densities: Option<&[f64]>,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<brep_core::analysis::AssemblyMass> {
        let properties = self.definition_mass_properties(relative_tolerance, max_evaluations_per_definition)?;
        let mut placed = self.placed_centroids(&properties)?;
        if let Some(densities) = densities {
            for ((weight, _), density) in placed.iter_mut().zip(densities) {
                *weight *= density;
                if !weight.is_finite() || *weight <= 0. {
                    return Err(refuse("STEP occurrence mass is outside the finite positive range"));
                }
            }
        }
        let center = weighted_centroid(&placed)?;
        let mut second = [[0.; 3]; 3];
        let total_mass = placed.iter().map(|(weight, _)| weight).sum();
        for (index, (occurrence, (weight, point))) in self.occurrences.iter().zip(placed).enumerate() {
            let mass = &properties[occurrence.definition];
            let half_trace = (0..3).map(|i| mass.inertia_mm5[i][i] / 2.).sum::<f64>();
            let density = densities.map_or(1., |values| values[index]);
            let scale = placement_determinant(occurrence.transform).abs() * density;
            let offset: [f64; 3] = std::array::from_fn(|i| point[i] - center[i]);
            for i in 0..3 {
                for j in i..3 {
                    // I = trace(C) Id - C, where C is the central second moment.
                    let mut transformed = 0.;
                    for k in 0..3 {
                        for l in 0..3 {
                            let moment = if k == l { half_trace } else { 0. } - mass.inertia_mm5[k][l];
                            transformed += occurrence.transform[i][k] * moment * occurrence.transform[j][l];
                        }
                    }
                    second[i][j] += scale * transformed + weight * offset[i] * offset[j];
                    second[j][i] = second[i][j];
                }
            }
        }
        let inertia = std::array::from_fn(|i| std::array::from_fn(|j| {
            if i == j {
                second[(i + 1) % 3][(i + 1) % 3] + second[(i + 2) % 3][(i + 2) % 3]
            } else {
                -second[i][j]
            }
        }));
        if !inertia.iter().flatten().all(|entry| entry.is_finite()) {
            return Err(refuse("STEP assembly inertia exceeds the finite range"));
        }
        Ok(brep_core::analysis::AssemblyMass { mass: total_mass, centroid: center, inertia })
    }

    /// Estimate principal moments in mm⁵ and their right-handed unit axes
    /// in assembly coordinates. Repeated moments have non-unique axes.
    pub fn material_principal_inertia(
        &self,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<brep_core::analysis::PrincipalInertia> {
        let tensor = self.material_inertia_mm5(relative_tolerance, max_evaluations_per_definition)?;
        let scale = tensor.iter().flatten().map(|entry| entry.abs()).fold(0., f64::max);
        if scale == 0. {
            return brep_core::analysis::principal_inertia(tensor);
        }
        let mut principal = brep_core::analysis::principal_inertia(tensor.map(|row| row.map(|entry| entry / scale)))?;
        principal.moments_mm5 = principal.moments_mm5.map(|moment| moment * scale);
        if !principal.moments_mm5.iter().all(|moment| moment.is_finite()) {
            return Err(refuse("STEP assembly principal inertia exceeds the finite range"));
        }
        Ok(principal)
    }

    /// Estimate the sum of occurrence boundary areas in mm². Area is reused
    /// per definition for similarities, and per linear map otherwise.
    pub fn material_surface_area_mm2(
        &self,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<f64> {
        let mut areas = BTreeMap::new();
        let mut total: f64 = 0.;
        for occurrence in &self.occurrences {
            let mut transform = occurrence.transform;
            for row in &mut transform[..3] {
                row[3] = 0.;
            }
            let uniform_scale = super::orthogonal_placement_scales(transform)
                .filter(|[x, y, z]| x == y && y == z).map(|scales| scales[0]);
            let linear = uniform_scale.is_none().then(|| {
                [0, 1, 2].map(|row| [0, 1, 2].map(|col| transform[row][col].to_bits()))
            });
            let model = &self.definitions[occurrence.definition];
            let area = match areas.entry((occurrence.definition, linear)) {
                std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let transformed;
                    let source = if uniform_scale.is_some() {
                        model
                    } else {
                        transformed = brep_core::transform::affine(model, transform)?;
                        &transformed
                    };
                    *entry.insert(brep_core::analysis::mass_properties(
                        source, relative_tolerance, max_evaluations_per_definition,
                    )?.surface_area_mm2)
                }
            };
            let scale = uniform_scale.unwrap_or(1.);
            total += area * scale * scale;
        }
        if !total.is_finite() || total <= 0. {
            return Err(refuse("STEP assembly surface area is outside the finite positive range"));
        }
        Ok(total)
    }

    fn placed_centroids(
        &self,
        properties: &[brep_core::analysis::MassProperties],
    ) -> Result<Vec<(f64, [f64; 3])>> {
        let mut placed = Vec::with_capacity(self.occurrences.len());
        for occurrence in &self.occurrences {
            let mass = &properties[occurrence.definition];
            let volume = placed_material_volume(occurrence, mass.signed_volume_mm3)?;
            let center: [f64; 3] = std::array::from_fn(|axis| {
                let row = occurrence.transform[axis];
                row[..3].iter().zip(mass.centroid)
                    .fold(row[3], |offset, (coefficient, coordinate)| coefficient.mul_add(coordinate, offset))
            });
            placed.push((volume, center));
        }
        Ok(placed)
    }

    fn definition_mass_properties(
        &self,
        relative_tolerance: f64,
        max_evaluations_per_definition: usize,
    ) -> Result<Vec<brep_core::analysis::MassProperties>> {
        self.definitions.iter().map(|model| {
            brep_core::analysis::mass_properties(model, relative_tolerance, max_evaluations_per_definition)
        }).collect()
    }

    /// Bound the forward ray parameter interval for each intersectable
    /// occurrence box. These are broad-phase candidates in assembly order.
    pub fn ray_candidates(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Vec<(usize, [f64; 2])>> {
        if !origin.iter().chain(&direction).all(|coordinate| coordinate.is_finite())
            || direction == [0.; 3]
        {
            return Err(refuse("STEP assembly ray requires a finite origin and nonzero finite direction"));
        }
        Ok(self.occurrence_bounds()?.into_iter().enumerate().filter_map(|(index, bounds)| {
            ray_box_interval(origin, direction, bounds).map(|interval| (index, interval))
        }).collect())
    }

    /// Trace a forward ray against placed trimmed faces. Returns occurrence
    /// indices in assembly order after conservative ray-box culling.
    /// Unresolved crossings and exhausted work remain explicit per occurrence.
    pub fn ray_crossings(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        max_cells: usize,
        max_domain_cells: usize,
    ) -> Result<Vec<(usize, Result<brep_core::ray_parity::Report>)>> {
        validate_ray_work(tolerance_uv, max_cells, max_domain_cells)?;
        let mut remaining = [max_cells, max_domain_cells];
        let mut results = Vec::new();
        for (index, _) in self.ray_candidates(origin, direction)? {
            let report = if remaining.contains(&0) {
                Err(refuse("STEP assembly ray work budget exhausted"))
            } else {
                Ok(self.trace_occurrence_ray(index, origin, direction, tolerance_uv, &mut remaining)?)
            };
            results.push((index, report));
        }
        Ok(results)
    }

    fn trace_occurrence_ray(
        &self,
        index: usize,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        remaining: &mut [usize; 2],
    ) -> Result<brep_core::ray_parity::Report> {
        if remaining.contains(&0) {
            return Err(refuse("STEP assembly ray work budget exhausted"));
        }
        let model = self.materialize(index)?;
        let report = brep_core::ray_parity::classify_ray(
            &model, origin, direction, tolerance_uv, remaining[0], remaining[1],
        )?;
        remaining[0] -= report.cells;
        remaining[1] -= report.domain_cells;
        Ok(report)
    }

    /// Return the first trimmed-face crossing only when its ray interval
    /// strictly precedes every other candidate. None means a resolved miss.
    pub fn first_ray_crossing(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        max_cells: usize,
        max_domain_cells: usize,
    ) -> Result<Option<(usize, brep_core::ray_parity::Crossing)>> {
        validate_ray_work(tolerance_uv, max_cells, max_domain_cells)?;
        let mut remaining = [max_cells, max_domain_cells];
        let mut boxes = self.ray_candidates(origin, direction)?;
        boxes.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]));
        let mut candidates = Vec::new();
        let mut earliest_upper = f64::INFINITY;
        for (index, [entry, _]) in boxes {
            if entry > earliest_upper {
                break;
            }
            let report = self.trace_occurrence_ray(index, origin, direction, tolerance_uv, &mut remaining)?;
            if !report.unresolved.is_empty() {
                return Err(refuse("STEP assembly ray has unresolved trimmed-face crossings"));
            }
            if let Some(crossing) = report.crossings.into_iter().next() {
                earliest_upper = earliest_upper.min(crossing.parameter[1]);
                candidates.push((index, crossing));
            }
        }
        candidates.sort_by(|a, b| a.1.parameter[0].total_cmp(&b.1.parameter[0]));
        let mut ordered = candidates.into_iter();
        let first = ordered.next();
        if let (Some((_, first)), Some((_, next))) = (&first, ordered.next())
            && first.parameter[1] >= next.parameter[0]
        {
            return Err(refuse("STEP assembly nearest ray crossing is ambiguous"));
        }
        Ok(first)
    }

    /// Prove material ownership for one placed occurrence in assembly coordinates.
    /// The returned solid owns the proved geometry and can answer repeated queries.
    pub fn qualify_occurrence(
        &self,
        index: usize,
        policy: brep_core::closed_solid::AdmissionPolicy,
    ) -> Result<brep_core::closed_solid::QualifiedClosedSolid> {
        let model = self.materialize(index)?;
        brep_core::closed_solid::qualify(model, policy)?.into_solid().ok_or_else(|| {
            Error::new("BREP_CLOSED_SOLID_UNPROVED", "STEP occurrence material ownership is unproved")
        })
    }

    /// Bound the union of material along a finite forward ray segment.
    /// Admission and ray budgets apply to each occurrence whose box overlaps the segment.
    /// If a relevant occurrence cannot be checked, the entire query returns an error.
    pub fn ray_material_coverage(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_parameter: f64,
        policy: brep_core::closed_solid::AdmissionPolicy,
        max_cells_per_occurrence: usize,
        max_domain_cells_per_occurrence: usize,
    ) -> Result<StepRayMaterialCoverage> {
        self.ray_material_coverage_at_least(
            1, origin, direction, max_parameter, policy,
            max_cells_per_occurrence, max_domain_cells_per_occurrence,
        )
    }

    /// Bound the spans crossing material of at least the requested number of
    /// distinct placed occurrences. Isolated contacts contribute no span.
    pub fn ray_material_coverage_at_least(
        &self,
        count: usize,
        origin: [f64; 3],
        direction: [f64; 3],
        max_parameter: f64,
        policy: brep_core::closed_solid::AdmissionPolicy,
        max_cells_per_occurrence: usize,
        max_domain_cells_per_occurrence: usize,
    ) -> Result<StepRayMaterialCoverage> {
        if count == 0 {
            return Err(refuse("STEP material ray coverage count must be positive"));
        }
        if !max_parameter.is_finite() || max_parameter < 0. {
            return Err(refuse("STEP material ray needs a finite nonnegative end parameter"));
        }
        validate_ray_work(policy.tolerance_uv, max_cells_per_occurrence, max_domain_cells_per_occurrence)?;
        let mut guaranteed = Vec::new();
        let mut possible = Vec::new();
        for (index, [lower, _]) in self.ray_candidates(origin, direction)? {
            if lower >= max_parameter {
                continue;
            }
            let solid = self.qualify_occurrence(index, policy)?;
            for [entry, exit] in solid.ray_material_intervals(
                origin, direction, max_cells_per_occurrence, max_domain_cells_per_occurrence,
            )? {
                let inner_exit = exit[0].min(max_parameter);
                if entry[1] < inner_exit {
                    guaranteed.push([entry[1], inner_exit]);
                }
                let outer_exit = exit[1].min(max_parameter);
                if entry[0] < outer_exit {
                    possible.push([entry[0], outer_exit]);
                }
            }
        }
        Ok(StepRayMaterialCoverage {
            guaranteed: ray_spans_at_least(guaranteed, count),
            possible: ray_spans_at_least(possible, count),
        })
    }

    /// Return exact transverse section loops and material area for each
    /// candidate polyhedral occurrence, in assembly coordinates.
    pub fn exact_polyhedral_sections(
        &self,
        normal: [f64; 3],
        offset: f64,
        policy: brep_core::closed_solid::AdmissionPolicy,
        max_exact_work_per_occurrence: u64,
    ) -> Result<Vec<(usize, Result<brep_core::analysis::ExactPolyhedralSectionArea>)>> {
        if normal == [0.; 3] || !normal.iter().all(|value| value.is_finite()) || !offset.is_finite() {
            return Err(refuse("STEP section requires a finite plane with a nonzero normal"));
        }
        let mut sections = Vec::new();
        for (index, bounds) in self.occurrence_bounds()?.into_iter().enumerate() {
            if !plane_may_intersect_box(normal, offset, bounds) {
                continue;
            }
            let section = self.qualify_occurrence(index, policy).and_then(|solid| {
                brep_core::analysis::exact_polyhedral_section_area(
                    &solid, normal, offset, max_exact_work_per_occurrence,
                )
            });
            sections.push((index, section));
        }
        Ok(sections)
    }

    pub fn materialize(&self, index: usize) -> Result<Model> {
        let occurrence = self.occurrences.get(index)
            .ok_or_else(|| refuse("STEP assembly occurrence index out of range"))?;
        brep_core::transform::affine(&self.definitions[occurrence.definition], occurrence.transform)
    }
}

fn split_placed_model(model: Model, axis: usize, coordinate: f64) -> Result<[Model; 2]> {
    let [min, max] = definition_control_bounds(&model);
    let tolerance = model.tolerance_mm;
    let empty = || Model::empty(tolerance);
    if coordinate <= min[axis] { return Ok([empty()?, model]); }
    if coordinate >= max[axis] { return Ok([model, empty()?]); }
    // Pad every artificial face outside the complete positive-weight control hull.
    let margin = (0..3).map(|i| max[i] - min[i]).fold(model.tolerance_mm, f64::max);
    let lo = min.map(|v| (v - margin).next_down());
    let mut hi = max.map(|v| (v + margin).next_up());
    hi[axis] = coordinate;
    if lo.iter().chain(&hi).any(|v| !v.is_finite()) {
        return Err(refuse("STEP coordinate split exceeds finite bounds"));
    }
    let clip = brep_core::cuboid(lo, hi)?;
    Ok([brep_core::boolean(&model, &clip, "intersection")?,
        brep_core::boolean(&model, &clip, "difference")?])
}

fn plane_may_intersect_box(normal: [f64; 3], offset: f64, [min, max]: [[f64; 3]; 2]) -> bool {
    let (mut lower, mut upper) = (0_f64, 0_f64);
    for (axis, coefficient) in normal.into_iter().enumerate() {
        if coefficient == 0. {
            continue;
        }
        let (lo, hi) = if coefficient > 0. {
            (min[axis], max[axis])
        } else {
            (max[axis], min[axis])
        };
        lower = (lower + (coefficient * lo).next_down()).next_down();
        upper = (upper + (coefficient * hi).next_up()).next_up();
    }
    // Overflow can lose a useful bound, but never excludes an occurrence.
    !lower.is_finite() || !upper.is_finite() || (lower <= offset && offset <= upper)
}

fn box_distance_squared_lower(a: &[[f64; 3]; 2], b: &[[f64; 3]; 2]) -> f64 {
    let mut squared = 0.;
    for axis in 0..3 {
        let gap = (b[0][axis] - a[1][axis]).max(a[0][axis] - b[1][axis])
            .next_down().max(0.);
        squared = (squared + (gap * gap).next_down().max(0.)).next_down().max(0.);
    }
    squared
}

fn ray_spans_at_least(spans: Vec<[f64; 2]>, count: usize) -> Vec<[f64; 2]> {
    let mut events = Vec::with_capacity(2 * spans.len());
    for [start, end] in spans {
        events.extend([(start, 1_isize), (end, -1)]);
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut spans = Vec::new();
    let mut active = 0_isize;
    let mut start = 0.;
    for group in events.chunk_by(|a, b| a.0 == b.0) {
        let was_covered = active as usize >= count;
        active += group.iter().map(|event| event.1).sum::<isize>();
        let covered = active as usize >= count;
        let parameter = group[0].0;
        if covered && !was_covered {
            start = parameter;
        } else if was_covered && !covered && start < parameter {
            spans.push([start, parameter]);
        }
    }
    spans
}

fn validate_ray_work(tolerance_uv: f64, max_cells: usize, max_domain_cells: usize) -> Result<()> {
    if !tolerance_uv.is_finite() || tolerance_uv <= 0.
        || !(1..=1_000_000).contains(&max_cells)
        || !(1..=8_000_000).contains(&max_domain_cells)
    {
        return Err(refuse("STEP assembly ray requires positive tolerance and bounded work"));
    }
    Ok(())
}

fn ray_box_interval(
    origin: [f64; 3],
    direction: [f64; 3],
    [min, max]: [[f64; 3]; 2],
) -> Option<[f64; 2]> {
    let (mut entry, mut exit) = (0_f64, f64::INFINITY);
    for axis in 0..3 {
        let start = origin[axis];
        let velocity = direction[axis];
        if velocity == 0. {
            if start < min[axis] || start > max[axis] {
                return None;
            }
            continue;
        }
        // Form both numerators for division by a positive speed, then
        // round each arithmetic operation outward.
        let (lo, hi) = if velocity > 0. {
            (min[axis] - start, max[axis] - start)
        } else {
            (start - max[axis], start - min[axis])
        };
        let speed = velocity.abs();
        entry = entry.max((lo.next_down() / speed).next_down());
        exit = exit.min((hi.next_up() / speed).next_up());
        if entry > exit {
            return None;
        }
    }
    Some([entry, exit])
}

fn weighted_centroid(placed: &[(f64, [f64; 3])]) -> Result<[f64; 3]> {
    let total: f64 = placed.iter().map(|entry| entry.0).sum();
    if !total.is_finite() || total <= 0. {
        return Err(refuse("STEP assembly volume is outside the finite positive range"));
    }
    let center = std::array::from_fn(|axis| {
        placed.iter().map(|(volume, point)| (volume / total) * point[axis]).sum::<f64>()
    });
    if !center.iter().all(|coordinate| coordinate.is_finite()) {
        return Err(refuse("STEP assembly centroid exceeds finite coordinates"));
    }
    Ok(center)
}

fn placed_material_volume(occurrence: &StepAssemblyOccurrence, local_volume: f64) -> Result<f64> {
    let volume = placement_determinant(occurrence.transform).abs() * local_volume;
    if !volume.is_finite() || volume <= 0. {
        return Err(refuse("STEP occurrence volume is outside the finite positive range"));
    }
    Ok(volume)
}

fn definition_control_bounds(model: &Model) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let points = model.vertices.iter().map(|vertex| vertex.point.as_slice())
        .chain(model.edges.iter().flat_map(|edge| edge.curve.control_points.iter().map(Vec::as_slice)))
        .chain(model.faces.iter().flat_map(|face| face.surface.control_points.iter().flatten().map(Vec::as_slice)));
    for point in points {
        for (axis, &coordinate) in point.iter().enumerate() {
            bounds[0][axis] = bounds[0][axis].min(coordinate);
            bounds[1][axis] = bounds[1][axis].max(coordinate);
        }
    }
    bounds
}

/// Import an AP242 assembly without expanding all of its placements into
/// a single B-rep. Each distinct definition is built and validated once.
pub fn import_step_shared_assembly(text: &str) -> Result<StepSharedAssembly> {
    Ok(read_shared_products(text, false)?.0)
}

/// Shared geometry and product labels resolved through the selected STEP graph.
pub struct NamedStepAssembly {
    pub assembly: StepSharedAssembly,
    pub name: String,
    /// Labels indexed by the definition referenced by each occurrence.
    pub definition_names: Vec<String>,
}

/// A part definition and its placed quantity in the flattened assembly.
#[derive(Debug, PartialEq, Eq)]
pub struct StepPartsListRow<'a> {
    pub definition: usize,
    pub name: &'a str,
    pub quantity: usize,
}

impl NamedStepAssembly {
    /// Export all occurrences of the selected parts-list definitions in their
    /// original assembly coordinates. Definition indices, not labels, select parts.
    pub fn export_selected_parts(&self, definitions: &[usize], assembly_name: &str) -> Result<String> {
        let count = self.assembly.definitions().len();
        if self.definition_names.len() != count || definitions.is_empty() || definitions.len() > count {
            return Err(refuse("Expected a nonempty selection of named assembly definitions"));
        }
        let selected: BTreeSet<_> = definitions.iter().copied().collect();
        if selected.len() != definitions.len() || selected.iter().any(|&index| index >= count) {
            return Err(refuse("Invalid or duplicate part definition index"));
        }
        let indices: Vec<_> = self.assembly.occurrences().iter().enumerate()
            .filter(|(_, part)| selected.contains(&part.definition)).map(|(index, _)| index).collect();
        let subset = self.assembly.extract_occurrences(&indices)?;
        let mut names = vec![""; subset.definitions().len()];
        for (&source, part) in indices.iter().zip(subset.occurrences()) {
            let original = self.assembly.occurrences()[source].definition;
            names[part.definition] = &self.definition_names[original];
        }
        crate::export_named_step_assembly(&subset, assembly_name, &names)
    }

    /// Count placements by definition identity, including coincident placements.
    /// A scale or orientation on a placement does not create a new definition.
    pub fn parts_list(&self) -> Result<Vec<StepPartsListRow<'_>>> {
        if self.definition_names.len() != self.assembly.definitions().len() {
            return Err(refuse("Expected one parts-list label per definition"));
        }
        let mut quantities = vec![0; self.definition_names.len()];
        for occurrence in self.assembly.occurrences() { quantities[occurrence.definition] += 1; }
        Ok(quantities.into_iter().enumerate().filter(|(_, quantity)| *quantity > 0)
            .map(|(definition, quantity)| StepPartsListRow {
                definition, name: &self.definition_names[definition], quantity,
            }).collect())
    }
}

pub fn import_named_step_assembly(text: &str) -> Result<NamedStepAssembly> {
    let (assembly, labels) = read_shared_products(text, true)?;
    let (name, definition_names) = labels.ok_or_else(|| refuse("Missing assembly product names"))?;
    Ok(NamedStepAssembly { assembly, name, definition_names })
}

fn read_shared_products(text: &str, named: bool) -> Result<(StepSharedAssembly, Option<(String, Vec<String>)>)> {
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
    let mut labels = if named {
        let (_, shapes, _) = step_xde::product_graph(&entities)?;
        let mut products = BTreeMap::new();
        for entity in entities.values() {
            if let Value::Call(kind,args) = &entity.value
                && kind == "SHAPE_DEFINITION_REPRESENTATION" && args.len() == 2
                && let Some(&product) = shapes.get(&one_ref(&args[0], "product shape")?) {
                let rep = one_ref(&args[1], "product representation")?;
                if products.insert(rep, product).is_some_and(|prior| prior != product) {
                    return Err(refuse("Ambiguous representation product"));
                }
            }
        }
        let root = placements[0].path.first().copied().ok_or_else(|| refuse("Missing assembly root"))?;
        if placements.iter().any(|p| p.path.first() != Some(&root)) {
            return Err(refuse("Named assembly requires one selected root"));
        }
        Some((products, root, Vec::new()))
    } else { None };
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
                if let Some((_, _, representations)) = &mut labels {
                    representations.push(placement.representation);
                }
                definitions.push(model);
                definition_indices.insert(key, index);
                index
            }
        };
        occurrences.push(StepAssemblyOccurrence { definition, transform: placement.transform });
    }
    let names = if let Some((products, root, representations)) = labels {
        let name = |rep| -> Result<String> {
            let id = products.get(&rep).ok_or_else(|| refuse("Selected representation has no product"))?;
            let (kind, args) = call(&entities, *id)?;
            if kind != "PRODUCT" || args.len() < 2 { return Err(refuse("Malformed named product")); }
            step_xde::decode_step_string(&string_value(&args[1], "product name")?)
        };
        Some((name(root)?, representations.into_iter().map(name).collect::<Result<_>>()?))
    } else { None };
    Ok((StepSharedAssembly { definitions, occurrences }, names))
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
