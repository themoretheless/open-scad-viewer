use super::*;

/// Certify the intersection of a bicubic with a planar patch when the signed
/// distance control coefficients are constant in V and affine in U. In that
/// finite cell the plane preimage is exactly U=constant; endpoint and sampled
/// affine-plane inversion then prove the full iso-curve lies in both domains.
fn certify_exact_planar_iso_intersection_cell(
    surface: &Surface,
    plane: &Surface,
    context: &ToleranceContext,
    rational: bool,
) -> Result<ExactIsoIntersectionCertificate> {
    surface.validate()?;
    plane.validate()?;
    let graph_admitted = if rational {
        rational_weight_bounds(surface).is_some()
    } else {
        is_bezier_le3_positive(surface)
    };
    if !graph_admitted
        || !is_bezier_le3_positive(plane)
        || !affine_graph_projection(surface, context.spatial_bounds().clear_mm)
    {
        return Err(refuse(
            "Exact iso certification requires an admitted single-span degree<=3 graph and an affine unit-weight patch",
        ));
    }
    let tol = context.spatial_bounds().on_mm;
    let (plane_origin, plane_normal) =
        planar_support(plane, tol).ok_or_else(|| refuse("Second support is not planar"))?;
    // Try both exact preimages: U=constant and V=constant.
    for fixed_u in [true, false] {
        if rational && !rational_weights_constant_on_fixed_axis(surface, fixed_u) {
            continue;
        }
        let degree = if fixed_u {
            surface.degree_u
        } else {
            surface.degree_v
        };
        let varying_degree = if fixed_u {
            surface.degree_v
        } else {
            surface.degree_u
        };
        let mut d = vec![0.; degree + 1];
        let mut valid = true;
        for i in 0..=degree {
            let q0 = if fixed_u {
                &surface.control_points[i][0]
            } else {
                &surface.control_points[0][i]
            };
            d[i] = dot(sub([q0[0], q0[1], q0[2]], plane_origin), plane_normal);
            for j in 1..=varying_degree {
                let q = if fixed_u {
                    &surface.control_points[i][j]
                } else {
                    &surface.control_points[j][i]
                };
                let dj = dot(sub([q[0], q[1], q[2]], plane_origin), plane_normal);
                if (dj - d[i]).abs() > tol * 0.1 {
                    valid = false;
                    break;
                }
            }
            if !valid {
                break;
            }
        }
        let span = d[degree] - d[0];
        if !valid || span.abs() <= context.spatial_bounds().clear_mm || d[0] * d[degree] >= 0. {
            continue;
        }
        if d.iter().enumerate().any(|(i, actual)| {
            let expected = d[0] + span * (i as f64 / degree as f64);
            (*actual - expected).abs() > tol * 0.1
        }) {
            continue;
        }
        let fixed = -d[0] / span;
        let param_margin = context.parametric_bounds().floor.max(64. * f64::EPSILON);
        if !(fixed > param_margin && fixed < 1. - param_margin) {
            continue;
        }
        let axis = if fixed_u { Axis::U } else { Axis::V };
        let exact_curve = surface.iso(axis, fixed)?;
        if exact_curve.periodic || exact_curve.decompose()?.len() != 1 {
            continue;
        }
        let domain = exact_curve.domain();
        let first_trace = if rational {
            Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: if fixed_u {
                    vec![vec![fixed, 0.], vec![fixed, 1.]]
                } else {
                    vec![vec![0., fixed], vec![1., fixed]]
                },
                weights: vec![1., 1.],
                periodic: false,
            }
        } else {
            Curve {
                degree: exact_curve.degree,
                knots: exact_curve.knots.clone(),
                control_points: exact_curve
                    .control_points
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let t = i as f64 / exact_curve.degree as f64;
                        if fixed_u {
                            vec![fixed, t]
                        } else {
                            vec![t, fixed]
                        }
                    })
                    .collect(),
                weights: exact_curve.weights.clone(),
                periodic: false,
            }
        };
        first_trace.validate()?;
        let Some(second_trace) = affine_uv_trace(plane, &exact_curve, tol)? else {
            continue;
        };
        if second_trace
            .control_points
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || *value < 0. || *value > 1.)
        {
            continue;
        }
        let first_endpoints = if fixed_u {
            [IsoEndpointLocation::VMin, IsoEndpointLocation::VMax]
        } else {
            [IsoEndpointLocation::UMin, IsoEndpointLocation::UMax]
        };
        let second_endpoints = [
            uv_location(
                [
                    second_trace.evaluate(domain[0])?.point[0],
                    second_trace.evaluate(domain[0])?.point[1],
                ],
                param_margin,
            )?,
            uv_location(
                [
                    second_trace.evaluate(domain[1])?.point[0],
                    second_trace.evaluate(domain[1])?.point[1],
                ],
                param_margin,
            )?,
        ];
        let endpoints = IsoEndpointOwnership {
            first_support: first_endpoints,
            second_support: second_endpoints,
        };
        let evidence = compose_predicate_evidence(
            context,
            [
                PredicateEvidence::root_parameter(context, [fixed, fixed])?,
                PredicateEvidence::tangent_normal(context, 0.)?,
                PredicateEvidence::correspondence(context, 0., 1.)?,
                PredicateEvidence::topology_preservation(
                    context,
                    "exact_iso_whole_domain_and_boundary_ownership",
                    true,
                )?,
            ],
        )?;
        let authority = ExactIsoAuthority {
            curve: RationalCurveDefinition::from_curve(&exact_curve)?,
            traces: [
                RationalCurveDefinition::from_curve(&first_trace)?,
                RationalCurveDefinition::from_curve(&second_trace)?,
            ],
            axis: if fixed_u {
                ExactIsoAxis::U
            } else {
                ExactIsoAxis::V
            },
            fixed_parameter_bits: fixed.to_bits(),
            transverse_slope_bits: span.to_bits(),
            endpoints: endpoints.clone(),
            context: context.spec_identity(),
        };
        let mut diagnostic_samples = Vec::with_capacity(9);
        for i in 0..=8 {
            let varying = i as f64 / 8.;
            let surface_uv = if fixed_u {
                [fixed, varying]
            } else {
                [varying, fixed]
            };
            let p = surface.evaluate(surface_uv[0], surface_uv[1])?.point;
            let p3 = [p[0], p[1], p[2]];
            let Some(plane_uv) = affine_plane_uv(plane, p3, tol)? else {
                valid = false;
                break;
            };
            diagnostic_samples.push((p3, surface_uv, plane_uv));
        }
        if !valid {
            continue;
        }
        let certificate = ExactIsoIntersectionCertificate {
            curve: exact_curve,
            uv_traces: [first_trace, second_trace],
            first_axis: authority.axis,
            fixed_parameter: fixed,
            transverse_signed_distance_slope: span,
            endpoints,
            whole_domain_contained: true,
            strict_transverse: true,
            context: context.spec_identity(),
            evidence,
            diagnostic_samples,
            authority,
        };
        if certificate.permits_topology_authorship() {
            return Ok(certificate);
        }
    }
    Err(refuse(
        "No unique strict-interior transverse constant-U/V iso branch was certified",
    ))
}

pub fn certify_exact_planar_iso_intersection(
    surface: &Surface,
    plane: &Surface,
    context: &ToleranceContext,
) -> Result<ExactIsoIntersectionCertificate> {
    certify_exact_planar_iso_intersection_cell(surface, plane, context, false)
}

pub fn certify_exact_rational_planar_iso_intersection(
    surface: &Surface,
    plane: &Surface,
    context: &ToleranceContext,
) -> Result<ExactIsoIntersectionCertificate> {
    certify_exact_planar_iso_intersection_cell(surface, plane, context, true)
}

/// Identity of one positive tensor knot cell. Internal upper boundaries are
/// excluded; only the final non-periodic span owns its terminal knot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TensorSpanId {
    pub u: usize,
    pub v: usize,
}

#[derive(Clone, Debug)]
pub struct RationalBezierPatchSpan {
    pub source_face: usize,
    pub span: TensorSpanId,
    pub domain: [[f64; 2]; 2],
    pub owns_upper: [bool; 2],
    pub patch: Surface,
    pub denominator_lower_bound: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RationalSurfaceBits {
    degree: [usize; 2],
    knots: [Vec<u64>; 2],
    controls: Vec<Vec<[u64; 3]>>,
    weights: Vec<Vec<u64>>,
}

impl RationalSurfaceBits {
    fn from_surface(surface: &Surface) -> Result<Self> {
        surface.validate()?;
        Ok(Self {
            degree: [surface.degree_u, surface.degree_v],
            knots: [
                surface
                    .knots_u
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
                surface
                    .knots_v
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
            ],
            controls: surface
                .control_points
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|point| [point[0].to_bits(), point[1].to_bits(), point[2].to_bits()])
                        .collect()
                })
                .collect(),
            weights: surface
                .weights
                .iter()
                .map(|row| row.iter().map(|value| value.to_bits()).collect())
                .collect(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct RationalBezierDecomposition {
    pub source_face: usize,
    pub u_breaks: Vec<f64>,
    pub v_breaks: Vec<f64>,
    pub spans: Vec<RationalBezierPatchSpan>,
    pub denominator_lower_bound: f64,
    authority: Vec<RationalSurfaceBits>,
}

impl RationalBezierDecomposition {
    pub fn permits_exact_decomposition(&self) -> bool {
        self.spans.len() == self.authority.len()
            && !self.spans.is_empty()
            && self.denominator_lower_bound > 0.
            && self
                .spans
                .iter()
                .zip(&self.authority)
                .all(|(span, authority)| {
                    span.source_face == self.source_face
                        && span.denominator_lower_bound > 0.
                        && RationalSurfaceBits::from_surface(&span.patch).as_ref() == Ok(authority)
                        && span.domain[0]
                            == [self.u_breaks[span.span.u], self.u_breaks[span.span.u + 1]]
                        && span.domain[1]
                            == [self.v_breaks[span.span.v], self.v_breaks[span.span.v + 1]]
                        && span.owns_upper
                            == [
                                span.span.u + 2 == self.u_breaks.len(),
                                span.span.v + 2 == self.v_breaks.len(),
                            ]
                })
    }
}

pub(super) fn active_breaks(knots: &[f64], degree: usize, controls: usize) -> Result<Vec<f64>> {
    let domain = [knots[degree], knots[controls]];
    let mut breaks = knots
        .iter()
        .copied()
        .filter(|value| *value >= domain[0] && *value <= domain[1])
        .collect::<Vec<_>>();
    breaks.dedup();
    if breaks.len() < 2
        || breaks.windows(2).any(|pair| pair[0] >= pair[1])
        || breaks.iter().any(|value| !value.is_finite())
    {
        return Err(refuse("Surface has a degenerate active knot span"));
    }
    Ok(breaks)
}

/// Perform exact homogeneous knot insertion through `Surface::trim`, retaining
/// every resulting rational Bezier tensor cell and its positive denominator
/// convex-hull bound.
pub fn decompose_rational_bezier_spans(
    surface: &Surface,
    source_face: usize,
    resource_limit: usize,
) -> Result<RationalBezierDecomposition> {
    surface.validate()?;
    if surface.periodic_u
        || surface.periodic_v
        || !(1..=3).contains(&surface.degree_u)
        || !(1..=3).contains(&surface.degree_v)
    {
        return Err(refuse(
            "Multi-span certification requires non-periodic degree 1 through 3 surfaces",
        ));
    }
    let u_breaks = active_breaks(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    )?;
    let v_breaks = active_breaks(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
    )?;
    let count = (u_breaks.len() - 1)
        .checked_mul(v_breaks.len() - 1)
        .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "Tensor span count overflow"))?;
    if resource_limit == 0 || count > resource_limit {
        return Err(Error::new(
            "BREP_SS_RESOURCE_LIMIT",
            "Tensor span budget exceeded",
        ));
    }
    let mut spans = Vec::with_capacity(count);
    let mut authority = Vec::with_capacity(count);
    let mut global_lower = f64::INFINITY;
    for u in 0..u_breaks.len() - 1 {
        for v in 0..v_breaks.len() - 1 {
            let patch =
                surface.trim([u_breaks[u], u_breaks[u + 1], v_breaks[v], v_breaks[v + 1]])?;
            let lower = patch
                .weights
                .iter()
                .flatten()
                .copied()
                .fold(f64::INFINITY, f64::min);
            if !lower.is_finite() || lower <= 0. {
                return Err(refuse(
                    "Rational Bezier span lacks a positive denominator lower bound",
                ));
            }
            if patch.control_points.len() != patch.degree_u + 1
                || patch.control_points[0].len() != patch.degree_v + 1
            {
                return Err(refuse(
                    "Knot insertion did not isolate one Bezier tensor span",
                ));
            }
            global_lower = global_lower.min(lower);
            authority.push(RationalSurfaceBits::from_surface(&patch)?);
            spans.push(RationalBezierPatchSpan {
                source_face,
                span: TensorSpanId { u, v },
                domain: [
                    [u_breaks[u], u_breaks[u + 1]],
                    [v_breaks[v], v_breaks[v + 1]],
                ],
                owns_upper: [u + 2 == u_breaks.len(), v + 2 == v_breaks.len()],
                patch,
                denominator_lower_bound: lower,
            });
        }
    }
    let result = RationalBezierDecomposition {
        source_face,
        u_breaks,
        v_breaks,
        spans,
        denominator_lower_bound: global_lower,
        authority,
    };
    if !result.permits_exact_decomposition() {
        return Err(refuse("Rational Bezier decomposition failed closed"));
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BranchOrientation {
    Increasing,
    Decreasing,
}

#[derive(Clone, Debug)]
pub struct TransverseSpanEvidence {
    pub homogeneous_distance_coefficients_bits: Vec<u64>,
    pub local_root: f64,
    pub signed_slope: f64,
    pub denominator_lower_bound: f64,
    pub exact_affine_factor: bool,
}

#[derive(Clone, Debug)]
pub struct CertifiedBranchFragment {
    pub source_faces: [usize; 2],
    pub source_spans: [TensorSpanId; 2],
    pub parameter_interval: [f64; 2],
    pub curve: Curve,
    pub pcurves: [Curve; 2],
    pub orientations: [BranchOrientation; 2],
    pub evidence: TransverseSpanEvidence,
}

#[derive(Clone, Debug)]
pub struct BranchComponent {
    pub component_id: usize,
    pub fragments: Vec<CertifiedBranchFragment>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FragmentAuthority {
    source_faces: [usize; 2],
    source_spans: [TensorSpanId; 2],
    parameter_interval_bits: [u64; 2],
    curve: RationalCurveDefinition,
    pcurves: [RationalCurveDefinition; 2],
    orientations: [BranchOrientation; 2],
    homogeneous_distance_coefficients_bits: Vec<u64>,
    local_root_bits: u64,
    signed_slope_bits: u64,
    denominator_lower_bound_bits: u64,
    exact_affine_factor: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BranchGraphAuthority {
    pub(super) context: ToleranceSpecIdentity,
    pub(super) fragment_definitions: Vec<FragmentAuthority>,
    pub(super) component_ranges: Vec<[usize; 2]>,
    pub(super) component_closed: Vec<bool>,
    pub(super) source_span_count: [usize; 2],
    pub(super) candidate_span_pairs: usize,
    pub(super) denominator_lower_bound_bits: u64,
    pub(super) resource_limit: usize,
}

#[derive(Clone, Debug)]
pub struct BranchCompletenessCertificate {
    pub context: ToleranceSpecIdentity,
    pub source_span_count: [usize; 2],
    pub candidate_span_pairs: usize,
    pub fragment_count: usize,
    pub component_count: usize,
    pub denominator_lower_bound: f64,
    pub all_cells_classified: bool,
    pub one_to_one_joins: bool,
    pub no_tangency_coincidence_or_multi_root: bool,
    pub resource_limit: usize,
}

#[derive(Clone, Debug)]
pub struct BranchGraph {
    pub components: Vec<BranchComponent>,
    pub certificate: BranchCompletenessCertificate,
    pub(super) authority: BranchGraphAuthority,
}

pub(super) fn fragment_authority(fragment: &CertifiedBranchFragment) -> Result<FragmentAuthority> {
    Ok(FragmentAuthority {
        source_faces: fragment.source_faces,
        source_spans: fragment.source_spans,
        parameter_interval_bits: fragment.parameter_interval.map(f64::to_bits),
        curve: RationalCurveDefinition::from_curve(&fragment.curve)?,
        pcurves: [
            RationalCurveDefinition::from_curve(&fragment.pcurves[0])?,
            RationalCurveDefinition::from_curve(&fragment.pcurves[1])?,
        ],
        orientations: fragment.orientations,
        homogeneous_distance_coefficients_bits: fragment
            .evidence
            .homogeneous_distance_coefficients_bits
            .clone(),
        local_root_bits: fragment.evidence.local_root.to_bits(),
        signed_slope_bits: fragment.evidence.signed_slope.to_bits(),
        denominator_lower_bound_bits: fragment.evidence.denominator_lower_bound.to_bits(),
        exact_affine_factor: fragment.evidence.exact_affine_factor,
    })
}

impl BranchGraph {
    pub fn permits_topology_authorship(&self) -> bool {
        let definitions = self
            .components
            .iter()
            .flat_map(|component| &component.fragments)
            .map(fragment_authority)
            .collect::<Result<Vec<_>>>();
        let mut offset = 0;
        let ranges = self
            .components
            .iter()
            .map(|component| {
                let range = [offset, offset + component.fragments.len()];
                offset = range[1];
                range
            })
            .collect::<Vec<_>>();
        self.certificate.context == self.authority.context
            && self.certificate.fragment_count == offset
            && self.certificate.component_count == self.components.len()
            && self.certificate.denominator_lower_bound > 0.
            && self.certificate.all_cells_classified
            && self.certificate.one_to_one_joins
            && self.certificate.no_tangency_coincidence_or_multi_root
            && self.certificate.fragment_count <= self.certificate.resource_limit
            && definitions.as_ref() == Ok(&self.authority.fragment_definitions)
            && ranges == self.authority.component_ranges
            && self.certificate.source_span_count == self.authority.source_span_count
            && self.certificate.candidate_span_pairs == self.authority.candidate_span_pairs
            && self.certificate.denominator_lower_bound.to_bits()
                == self.authority.denominator_lower_bound_bits
            && self.certificate.resource_limit == self.authority.resource_limit
            && self
                .components
                .iter()
                .map(|component| component.closed)
                .eq(self.authority.component_closed.iter().copied())
    }
}
