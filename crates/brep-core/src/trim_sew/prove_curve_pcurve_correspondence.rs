use super::*;

fn exact_iso_pcurve(
    surface: &Surface,
    pcurve: &Curve,
) -> Result<Option<(Curve, CurvePcurveSupport)>> {
    let u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let degree = pcurve.degree;
    for fixed_u in [true, false] {
        let fixed_axis = usize::from(!fixed_u);
        let varying_axis = usize::from(fixed_u);
        let fixed = pcurve.control_points[0][fixed_axis];
        let varying_domain = if fixed_u { v } else { u };
        let fixed_domain = if fixed_u { u } else { v };
        if !(fixed >= fixed_domain[0] && fixed <= fixed_domain[1])
            || pcurve
                .control_points
                .iter()
                .any(|point| point[fixed_axis].to_bits() != fixed.to_bits())
            || pcurve.control_points.iter().enumerate().any(|(i, point)| {
                let expected = varying_domain[0]
                    + (varying_domain[1] - varying_domain[0]) * i as f64 / degree as f64;
                point[varying_axis].to_bits() != expected.to_bits()
            })
            || pcurve.domain().map(f64::to_bits) != varying_domain.map(f64::to_bits)
            || pcurve.weights.windows(2).any(|pair| pair[0] != pair[1])
        {
            continue;
        }
        let curve = surface.iso(if fixed_u { Axis::U } else { Axis::V }, fixed)?;
        return Ok(Some((
            curve,
            if fixed_u {
                CurvePcurveSupport::CurvedIsoU
            } else {
                CurvePcurveSupport::CurvedIsoV
            },
        )));
    }
    Ok(None)
}

fn affine_planar_lift(
    surface: &Surface,
    pcurve: &Curve,
    context: &ToleranceContext,
) -> Result<Option<Curve>> {
    // A geometric plane can still have a nonlinear rational chart. The
    // affine UV lift is valid only for a complete uniform-weight Bernstein
    // grid, not a prefix of a multispan or periodic surface.
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let weight = surface.weights[0][0];
    if surface.periodic_u || surface.periodic_v
        || nu != surface.degree_u + 1 || nv != surface.degree_v + 1
        || surface.weights.iter().flatten().any(|w| *w != weight)
        || !surface.knots_u[..=surface.degree_u].iter().all(|k| *k == surface.knots_u[surface.degree_u])
        || !surface.knots_u[nu..].iter().all(|k| *k == surface.knots_u[nu])
        || !surface.knots_v[..=surface.degree_v].iter().all(|k| *k == surface.knots_v[surface.degree_v])
        || !surface.knots_v[nv..].iter().all(|k| *k == surface.knots_v[nv])
    {
        return Ok(None);
    }
    let o = surface
        .evaluate(
            surface.knots_u[surface.degree_u],
            surface.knots_v[surface.degree_v],
        )?
        .point;
    let u_domain = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v_domain = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let pu = surface.evaluate(u_domain[1], v_domain[0])?.point;
    let pv = surface.evaluate(u_domain[0], v_domain[1])?.point;
    let eu: [f64; 3] =
        std::array::from_fn(|axis| (pu[axis] - o[axis]) / (u_domain[1] - u_domain[0]));
    let ev: [f64; 3] =
        std::array::from_fn(|axis| (pv[axis] - o[axis]) / (v_domain[1] - v_domain[0]));
    let tol = context.spatial_bounds().on_mm;
    for i in 0..=surface.degree_u {
        for j in 0..=surface.degree_v {
            let u = u_domain[0] + (u_domain[1] - u_domain[0]) * i as f64 / surface.degree_u as f64;
            let v = v_domain[0] + (v_domain[1] - v_domain[0]) * j as f64 / surface.degree_v as f64;
            let expected: [f64; 3] = std::array::from_fn(|axis| {
                o[axis] + (u - u_domain[0]) * eu[axis] + (v - v_domain[0]) * ev[axis]
            });
            let residual = surface.control_points[i][j]
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            if residual > tol {
                return Ok(None);
            }
        }
    }
    let lifted = Curve {
        degree: pcurve.degree,
        knots: pcurve.knots.clone(),
        control_points: pcurve
            .control_points
            .iter()
            .map(|point| {
                (0..3)
                    .map(|axis| {
                        o[axis]
                            + (point[0] - u_domain[0]) * eu[axis]
                            + (point[1] - v_domain[0]) * ev[axis]
                    })
                    .collect()
            })
            .collect(),
        weights: pcurve.weights.clone(),
        periodic: false,
    };
    lifted.validate()?;
    Ok(Some(lifted))
}

/// Prove exact 3D rational curve correspondence to either an exact curved
/// constant-U/V pcurve or an affine-planar pcurve. Generic freeform lifts,
/// periodic and multispan inputs refuse.
pub fn prove_curve_pcurve_correspondence(
    context: &ToleranceContext,
    curve: &Curve,
    surface: &Surface,
    pcurve: &Curve,
    orientation: ParameterOrientation,
    owner: BoundaryUse,
) -> Result<CurvePcurveCorrespondence> {
    curve.validate()?;
    surface.validate()?;
    pcurve.validate()?;
    if curve.control_points[0].len() != 3
        || pcurve.control_points[0].len() != 2
        || curve.periodic
        || pcurve.periodic
        || curve.decompose()?.len() != 1
        || pcurve.decompose()?.len() != 1
        || curve.domain().map(f64::to_bits) != pcurve.domain().map(f64::to_bits)
    {
        return Err(refuse(
            "BREP_SEW_PARAMETERIZATION",
            "Curve/pcurve proof requires matching non-periodic single-span domains",
        ));
    }
    let (lifted, support) = if let Some((iso, support)) = exact_iso_pcurve(surface, pcurve)? {
        (iso, support)
    } else if let Some(lifted) = affine_planar_lift(surface, pcurve, context)? {
        (lifted, CurvePcurveSupport::AffinePlanar)
    } else {
        return Err(refuse(
            "BREP_SEW_CURVE_MISMATCH",
            "Pcurve is neither an exact support iso nor an affine-planar lift",
        ));
    };
    let expected = match orientation {
        ParameterOrientation::Same => lifted,
        ParameterOrientation::Reversed => lifted.reverse()?,
    };
    let curve_definition = RationalCurveDefinition::from_curve(curve)?;
    let expected_definition = RationalCurveDefinition::from_curve(&expected)?;
    let coefficient_residual = curve
        .control_points
        .iter()
        .zip(&expected.control_points)
        .map(|(left, right)| {
            left.iter()
                .zip(right)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let same_rational_basis = curve.degree == expected.degree
        && curve.knots == expected.knots
        && curve.weights == expected.weights
        && curve.periodic == expected.periodic
        && curve.control_points.len() == expected.control_points.len();
    if curve_definition != expected_definition
        && !(support == CurvePcurveSupport::AffinePlanar && same_rational_basis)
    {
        return Err(refuse(
            "BREP_SEW_CURVE_MISMATCH",
            "3D rational definition does not equal the oriented pcurve lift",
        ));
    }
    if support == CurvePcurveSupport::AffinePlanar {
        // Source near-planarity and curve coefficient residuals can add up.
        // Qualification must bound the actual original surface composition
        // over the entire parameter interval within the model tolerance.
        let agreement = nurbs_core::curve_surface_agreement::verify(
            curve, pcurve, surface, orientation == ParameterOrientation::Reversed,
            context.spatial_bounds().on_mm, 10000,
        )?;
        if agreement.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
            return Err(refuse("BREP_SEW_CURVE_MISMATCH",
                "Affine support correspondence is not qualified over the full source interval"));
        }
    }
    let pcurve_definition = RationalCurveDefinition::from_curve(pcurve)?;
    let parameter_domain_bits = curve.domain().map(f64::to_bits);
    let evidence = compose_predicate_evidence(
        context,
        [
            PredicateEvidence::correspondence(context, coefficient_residual, 1.)?,
            PredicateEvidence::topology_preservation(
                context,
                "curve_pcurve_parameter_orientation_context_owner",
                true,
            )?,
        ],
    )?;
    let authority = CurvePcurveAuthority {
        curve: curve_definition,
        pcurve: pcurve_definition,
        support,
        parameter_domain_bits,
        orientation,
        owner: owner.clone(),
        context: context.spec_identity(),
    };
    let certificate = CurvePcurveCorrespondence {
        curve: curve.clone(),
        pcurve: pcurve.clone(),
        support,
        parameter_domain_bits,
        orientation,
        owner,
        context: context.spec_identity(),
        evidence,
        no_snapping: true,
        authority,
    };
    if !certificate.permits_exact_correspondence() {
        return Err(refuse(
            "BREP_SEW_CORRESPONDENCE_REQUIRED",
            "Curve/pcurve certificate failed its retained authority check",
        ));
    }
    Ok(certificate)
}

/// Construct a boundary proof from two authored uses of one rational curve.
/// The caller supplies explicit cyclic locations and shell ownership.
pub fn prove_boundary_correspondence(
    context: &ToleranceContext,
    input: BoundaryCorrespondenceInput<'_>,
) -> Result<BoundaryCorrespondence> {
    let BoundaryCorrespondenceInput {
        curve_a,
        curve_b,
        endpoints_a,
        endpoints_b,
        orientation,
        seam_shift,
        shell,
        uses,
    } = input;
    let definition_a = RationalCurveDefinition::from_curve(curve_a)?;
    let definition_b = RationalCurveDefinition::from_curve(curve_b)?;
    if definition_a != definition_b {
        return Err(refuse(
            "BREP_SEW_CURVE_MISMATCH",
            "Boundary curves differ in their complete rational definition",
        ));
    }
    let a = [point_bits(endpoints_a[0])?, point_bits(endpoints_a[1])?];
    let b = [point_bits(endpoints_b[0])?, point_bits(endpoints_b[1])?];
    let endpoint_ok = match orientation {
        ParameterOrientation::Same => a == b,
        ParameterOrientation::Reversed => a[0] == b[1] && a[1] == b[0],
    };
    if !endpoint_ok {
        return Err(refuse(
            "BREP_SEW_ENDPOINT_MISMATCH",
            "Endpoint correspondence disagrees with parameter orientation",
        ));
    }
    if curve_a.periodic {
        if seam_shift.abs() > 1 {
            return Err(refuse(
                "BREP_SEW_SEAM",
                "Periodic seam shift exceeds one canonical period",
            ));
        }
    } else if seam_shift != 0 {
        return Err(refuse(
            "BREP_SEW_SEAM",
            "Non-periodic boundary cannot carry a seam shift",
        ));
    }
    if uses[0].wire == uses[1].wire && uses[0].cyclic_index == uses[1].cyclic_index {
        return Err(refuse(
            "BREP_SEW_DUPLICATE",
            "Boundary proof repeats the same cyclic use",
        ));
    }
    if uses[0].reversed == uses[1].reversed {
        return Err(refuse(
            "BREP_SEW_ORIENTATION",
            "Boundary uses must traverse the curve oppositely",
        ));
    }
    let [lo, hi] = curve_a.domain();
    let evaluated_lo = curve_a.evaluate(lo)?.point;
    let evaluated_hi = curve_a.evaluate(hi)?.point;
    if evaluated_lo.len() != 3 || evaluated_hi.len() != 3 {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Boundary correspondence requires a 3D curve",
        ));
    }
    let evaluated = [
        [evaluated_lo[0], evaluated_lo[1], evaluated_lo[2]],
        [evaluated_hi[0], evaluated_hi[1], evaluated_hi[2]],
    ];
    let distance = |left: [f64; 3], right: [f64; 3]| {
        left.into_iter()
            .zip(right)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
    };
    let expected_a = if uses[0].reversed {
        [evaluated[1], evaluated[0]]
    } else {
        evaluated
    };
    let expected_b = if uses[1].reversed {
        [evaluated[1], evaluated[0]]
    } else {
        evaluated
    };
    let residual = distance(endpoints_a[0], expected_a[0])
        .max(distance(endpoints_a[1], expected_a[1]))
        .max(distance(endpoints_b[0], expected_b[0]))
        .max(distance(endpoints_b[1], expected_b[1]));
    let local_scale = distance(evaluated[0], evaluated[1]).max(1.);
    let evidence = compose_predicate_evidence(
        context,
        [
            PredicateEvidence::correspondence(context, residual, local_scale)?,
            PredicateEvidence::topology_preservation(
                context,
                "shell_owner_and_cyclic_boundary_uses",
                true,
            )?,
        ],
    )?;
    Ok(BoundaryCorrespondence {
        context: context.spec_identity(),
        authority: CurveAuthority::Rational(definition_a),
        endpoints_bits: [a, b],
        parameter_domain_bits: [lo.to_bits(), hi.to_bits()],
        orientation,
        seam_shift,
        shell,
        uses,
        evidence,
        heal_recipe: None,
    })
}

/// Derive correspondence authority from the model's own two manifold edge
/// uses. Bridge callers cannot provide or override any evidence fields.
pub fn prove_model_edge_correspondence(
    model: &Model,
    context: &ToleranceContext,
    edge_index: usize,
) -> Result<BoundaryCorrespondence> {
    let edge = model.edges.get(edge_index).ok_or_else(|| {
        refuse(
            "BREP_HEAL_RECIPE_INVALID",
            "Heal edge index is out of range",
        )
    })?;
    let mut uses = Vec::new();
    for (face_index, face) in model.faces.iter().enumerate() {
        for &wire_index in std::iter::once(&face.outer).chain(&face.holes) {
            for (cyclic_index, coedge) in model.loops[wire_index].coedges.iter().enumerate() {
                if coedge.edge == edge_index {
                    uses.push(BoundaryUse {
                        face: face_index,
                        wire: wire_index,
                        cyclic_index,
                        reversed: coedge.reversed,
                    });
                }
            }
        }
    }
    if uses.len() != 2 || uses[0].reversed == uses[1].reversed {
        return Err(refuse(
            "BREP_HEAL_PROOF_REQUIRED",
            "Heal requires exactly two native opposite manifold boundary uses",
        ));
    }
    let authored = [
        model.vertices[edge.vertices[0]].point,
        model.vertices[edge.vertices[1]].point,
    ];
    let endpoints = |use_: &BoundaryUse| {
        if use_.reversed {
            [authored[1], authored[0]]
        } else {
            authored
        }
    };
    let shell = model
        .shells
        .iter()
        .position(|shell| shell.faces.iter().any(|use_| use_.face == uses[0].face))
        .ok_or_else(|| {
            refuse(
                "BREP_HEAL_PROOF_REQUIRED",
                "Boundary face has no shell owner",
            )
        })?;
    prove_boundary_correspondence(
        context,
        BoundaryCorrespondenceInput {
            curve_a: &edge.curve,
            curve_b: &edge.curve,
            endpoints_a: endpoints(&uses[0]),
            endpoints_b: endpoints(&uses[1]),
            orientation: ParameterOrientation::Reversed,
            seam_shift: 0,
            shell,
            uses: [uses[0].clone(), uses[1].clone()],
        },
    )
}

/// Compatibility wrapper. Endpoint keys do not contain enough information to
/// prove a full boundary correspondence and therefore fail closed.
pub fn exact_sew(
    _base: &SewSnapshot,
    _pending: &[SewLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    Err(refuse(
        "BREP_SEW_CORRESPONDENCE_REQUIRED",
        "Exact sew requires context-bound BoundaryCorrespondence proofs",
    ))
}

/// Exact-match sew using one complete correspondence proof per boundary pair.
pub fn exact_sew_correspondences(
    base: &SewSnapshot,
    context: &ToleranceContext,
    pending: &[CorrespondenceLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    if pending.len() > 4096 {
        return Err(refuse(
            "BREP_SEW_RESOURCE_LIMIT",
            "Boundary correspondence budget exceeded",
        ));
    }
    let mut next = base.clone();
    let mut uses: BTreeMap<SewEdgeKey, Vec<&CorrespondenceLedgerEntry>> = BTreeMap::new();
    for entry in pending {
        uses.entry(entry.key.clone()).or_default().push(entry);
    }
    for (key, entries) in &uses {
        if entries.len() != 1 {
            return Err(refuse(
                "BREP_SEW_DUPLICATE",
                "Every sew key must have exactly one boundary correspondence",
            ));
        }
        let proof = &entries[0].correspondence;
        if proof.context != context.spec_identity() || proof.evidence.context != proof.context {
            return Err(refuse(
                "BREP_SEW_CONTEXT_MISMATCH",
                "Boundary correspondence belongs to a different tolerance context",
            ));
        }
        if proof.uses[0].reversed == proof.uses[1].reversed {
            return Err(refuse(
                "BREP_SEW_ORIENTATION",
                "Boundary proof does not establish opposite traversal",
            ));
        }
        let endpoints_match = match proof.orientation {
            ParameterOrientation::Same => proof.endpoints_bits[0] == proof.endpoints_bits[1],
            ParameterOrientation::Reversed => {
                proof.endpoints_bits[0][0] == proof.endpoints_bits[1][1]
                    && proof.endpoints_bits[0][1] == proof.endpoints_bits[1][0]
            }
        };
        if !endpoints_match {
            return Err(refuse(
                "BREP_SEW_ENDPOINT_MISMATCH",
                "Boundary proof endpoint mutation invalidated orientation",
            ));
        }
        let domain = proof.parameter_domain_bits.map(f64::from_bits);
        if !domain.iter().all(|value| value.is_finite()) || domain[0] >= domain[1] {
            return Err(refuse(
                "BREP_SEW_PARAMETERIZATION",
                "Boundary proof parameter domain is invalid",
            ));
        }
        let periodic = match &proof.authority {
            CurveAuthority::Rational(definition) => definition.periodic,
            CurveAuthority::CanonicalConstruction(identity) => {
                if identity.is_empty() || identity.len() > 256 {
                    return Err(refuse(
                        "BREP_SEW_CURVE_MISMATCH",
                        "Canonical construction identity is invalid",
                    ));
                }
                false
            }
        };
        let authority_key = authority_key(&proof.authority);
        if key.definition.as_deref() != Some(authority_key.as_str()) {
            return Err(refuse(
                "BREP_SEW_CURVE_MISMATCH",
                "Sew key does not carry the proven complete curve authority",
            ));
        }
        if (!periodic && proof.seam_shift != 0) || proof.seam_shift.abs() > 1 {
            return Err(refuse(
                "BREP_SEW_SEAM",
                "Boundary proof carries an incompatible periodic seam shift",
            ));
        }
        if key.shell != Some(proof.shell)
            || (proof.uses[0].wire == proof.uses[1].wire
                && proof.uses[0].cyclic_index == proof.uses[1].cyclic_index)
        {
            return Err(refuse(
                "BREP_SEW_SHELL_OWNERSHIP",
                "Boundary proof shell or cyclic ownership is inconsistent",
            ));
        }
        let has_correspondence = proof
            .evidence
            .claims
            .iter()
            .any(|claim| matches!(claim, EvidenceClaim::Correspondence { .. }));
        let has_topology = proof
            .evidence
            .claims
            .iter()
            .any(|claim| matches!(claim, EvidenceClaim::TopologyPreservation { .. }));
        if !has_correspondence || !has_topology {
            return Err(refuse(
                "BREP_SEW_CORRESPONDENCE_REQUIRED",
                "Boundary proof lacks correspondence or topology evidence",
            ));
        }
        next.edges.insert(key.clone());
        next.oriented.insert(key.clone(), true);
    }
    let budget_ok = next.displacements.iter().all(|d| d.bound <= 1_000_000);
    Ok((
        next,
        SewCertificate {
            matched: uses.len(),
            complete: true,
            displacement_budget_ok: budget_ok,
        },
    ))
}

/// Rollback drill helper: apply sew or restore `base` on any refusal.
pub fn sew_atomic(
    base: SewSnapshot,
    pending: &[SewLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    exact_sew(&base, pending)
}
