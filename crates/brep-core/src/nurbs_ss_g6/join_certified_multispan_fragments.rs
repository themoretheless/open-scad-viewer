use super::*;

pub(super) fn curve_endpoint_bits(curve: &Curve, end: usize) -> Result<Vec<u64>> {
    let domain = curve.domain();
    Ok(curve
        .evaluate(domain[end])?
        .point
        .iter()
        .map(|value| {
            let value = if *value == 0. { 0. } else { *value };
            value.to_bits()
        })
        .collect())
}

pub(super) fn joined_endpoint_key(
    fragment: &CertifiedBranchFragment,
    end: usize,
) -> Result<(Vec<u64>, Vec<u64>, Vec<u64>)> {
    Ok((
        curve_endpoint_bits(&fragment.curve, end)?,
        curve_endpoint_bits(&fragment.pcurves[0], end)?,
        curve_endpoint_bits(&fragment.pcurves[1], end)?,
    ))
}

/// Deterministically join exact per-cell fragments. A join exists only when
/// the 3D endpoint and both oriented UV endpoints are bit-identical.
pub fn join_certified_multispan_fragments(
    mut fragments: Vec<CertifiedBranchFragment>,
    context: &ToleranceContext,
    source_span_count: [usize; 2],
    candidate_span_pairs: usize,
    denominator_lower_bound: f64,
    resource_limit: usize,
) -> Result<BranchGraph> {
    if fragments.is_empty() || fragments.len() > resource_limit || denominator_lower_bound <= 0. {
        return Err(Error::new(
            "BREP_SS_RESOURCE_LIMIT",
            "Branch fragment budget is empty or exceeded",
        ));
    }
    fragments.sort_by(|a, b| {
        joined_endpoint_key(a, 0)
            .unwrap()
            .cmp(&joined_endpoint_key(b, 0).unwrap())
            .then_with(|| a.source_spans.cmp(&b.source_spans))
            .then_with(|| a.parameter_interval[0].total_cmp(&b.parameter_interval[0]))
    });
    let mut endpoints = BTreeMap::<(Vec<u64>, Vec<u64>, Vec<u64>), Vec<(usize, usize)>>::new();
    for (id, fragment) in fragments.iter().enumerate() {
        for end in 0..2 {
            endpoints
                .entry(joined_endpoint_key(fragment, end)?)
                .or_default()
                .push((id, end));
        }
    }
    if endpoints.values().any(|uses| uses.len() > 2) {
        return Err(refuse(
            "Ambiguous branch fork or merge at an exact endpoint",
        ));
    }
    let mut adjacency = vec![[None; 2]; fragments.len()];
    for uses in endpoints.values().filter(|uses| uses.len() == 2) {
        let [(a, ae), (b, be)] = uses.as_slice() else {
            unreachable!()
        };
        if a == b || ae == be {
            return Err(refuse("Ambiguous branch orientation at an exact join"));
        }
        adjacency[*a][*ae] = Some(*b);
        adjacency[*b][*be] = Some(*a);
    }
    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    let starts = (0..fragments.len())
        .filter(|id| adjacency[*id].iter().filter(|next| next.is_some()).count() < 2)
        .chain(0..fragments.len())
        .collect::<Vec<_>>();
    for start in starts {
        if visited.contains(&start) {
            continue;
        }
        let mut ordered = Vec::new();
        let mut current = start;
        let mut entered_end = adjacency[current][0].is_some() && adjacency[current][1].is_none();
        let closed;
        loop {
            if !visited.insert(current) {
                closed = current == start;
                break;
            }
            let mut fragment = fragments[current].clone();
            if entered_end {
                fragment.curve = fragment.curve.reverse()?;
                fragment.pcurves = [
                    fragment.pcurves[0].reverse()?,
                    fragment.pcurves[1].reverse()?,
                ];
                fragment.parameter_interval.reverse();
                fragment.orientations =
                    fragment.orientations.map(|orientation| match orientation {
                        BranchOrientation::Increasing => BranchOrientation::Decreasing,
                        BranchOrientation::Decreasing => BranchOrientation::Increasing,
                    });
            }
            ordered.push(fragment);
            let exit = usize::from(!entered_end);
            let Some(next) = adjacency[current][exit] else {
                closed = false;
                break;
            };
            let next_enter = adjacency[next][1] == Some(current);
            current = next;
            entered_end = next_enter;
        }
        components.push(BranchComponent {
            component_id: 0,
            fragments: ordered,
            closed,
        });
    }
    components.sort_by(|a, b| {
        joined_endpoint_key(&a.fragments[0], 0)
            .unwrap()
            .cmp(&joined_endpoint_key(&b.fragments[0], 0).unwrap())
    });
    for (id, component) in components.iter_mut().enumerate() {
        component.component_id = id;
    }
    let definitions = components
        .iter()
        .flat_map(|component| &component.fragments)
        .map(fragment_authority)
        .collect::<Result<Vec<_>>>()?;
    let mut offset = 0;
    let component_ranges = components
        .iter()
        .map(|component| {
            let result = [offset, offset + component.fragments.len()];
            offset = result[1];
            result
        })
        .collect::<Vec<_>>();
    let authority = BranchGraphAuthority {
        context: context.spec_identity(),
        fragment_definitions: definitions,
        component_ranges,
        component_closed: components
            .iter()
            .map(|component| component.closed)
            .collect(),
        source_span_count,
        candidate_span_pairs,
        denominator_lower_bound_bits: denominator_lower_bound.to_bits(),
        resource_limit,
    };
    let graph = BranchGraph {
        certificate: BranchCompletenessCertificate {
            context: context.spec_identity(),
            source_span_count,
            candidate_span_pairs,
            fragment_count: offset,
            component_count: components.len(),
            denominator_lower_bound,
            all_cells_classified: true,
            one_to_one_joins: true,
            no_tangency_coincidence_or_multi_root: true,
            resource_limit,
        },
        components,
        authority,
    };
    if !graph.permits_topology_authorship() {
        return Err(refuse(
            "Joined branch graph failed its native authority check",
        ));
    }
    Ok(graph)
}

pub(super) fn affine_plane_uv_domain(
    surface: &Surface,
    point: [f64; 3],
    tolerance: f64,
) -> Result<Option<[f64; 2]>> {
    let u = active_breaks(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    )?;
    let v = active_breaks(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
    )?;
    let [umin, umax] = [u[0], *u.last().unwrap()];
    let [vmin, vmax] = [v[0], *v.last().unwrap()];
    let origin = surface.evaluate(umin, vmin)?.point;
    let pu = surface.evaluate(umax, vmin)?.point;
    let pv = surface.evaluate(umin, vmax)?.point;
    let eu = sub(pu, origin);
    let ev = sub(pv, origin);
    let rhs = sub(point, origin);
    let [uu, uv, vv] = [dot(eu, eu), dot(eu, ev), dot(ev, ev)];
    let determinant = uu * vv - uv * uv;
    if determinant.abs() <= tolerance * tolerance {
        return Ok(None);
    }
    let a = (dot(rhs, eu) * vv - dot(rhs, ev) * uv) / determinant;
    let b = (dot(rhs, ev) * uu - dot(rhs, eu) * uv) / determinant;
    let result = [umin + a * (umax - umin), vmin + b * (vmax - vmin)];
    Ok(((result[0] >= umin - tolerance)
        && (result[0] <= umax + tolerance)
        && (result[1] >= vmin - tolerance)
        && (result[1] <= vmax + tolerance))
        .then_some(result))
}

pub(super) fn affine_plane_trace(plane: &Surface, curve: &Curve, tolerance: f64) -> Result<Option<Curve>> {
    let mut controls = Vec::with_capacity(curve.control_points.len());
    for point in &curve.control_points {
        let Some(uv) = affine_plane_uv_domain(plane, [point[0], point[1], point[2]], tolerance)?
        else {
            return Ok(None);
        };
        controls.push(uv.to_vec());
    }
    let trace = Curve {
        degree: curve.degree,
        knots: curve.knots.clone(),
        control_points: controls,
        weights: curve.weights.clone(),
        periodic: false,
    };
    trace.validate()?;
    Ok(Some(trace))
}

pub(super) fn owning_span(breaks: &[f64], parameter: f64) -> Option<usize> {
    breaks.windows(2).enumerate().find_map(|(index, pair)| {
        (parameter >= pair[0]
            && (parameter < pair[1] || (index + 2 == breaks.len() && parameter == pair[1])))
            .then_some(index)
    })
}

/// Candidate `/6` infrastructure only. It certifies all affine homogeneous
/// roots that are unique and transverse in each positive rational Bezier span,
/// then joins their exact iso fragments. It does not assemble Boolean topology.
pub fn certify_multispan_ss(
    source: &Surface,
    plane: &Surface,
    source_faces: [usize; 2],
    context: &ToleranceContext,
    resource_limit: usize,
) -> Result<BranchGraph> {
    let source_decomposition =
        decompose_rational_bezier_spans(source, source_faces[0], resource_limit)?;
    let plane_decomposition =
        decompose_rational_bezier_spans(plane, source_faces[1], resource_limit)?;
    let tolerance = context.spatial_bounds().on_mm;
    let (plane_origin, plane_normal) =
        planar_support(plane, tolerance).ok_or_else(|| refuse("Second support is not planar"))?;
    if !affine_graph_projection(plane, tolerance) {
        return Err(refuse(
            "Second support is not an affine planar parameterization",
        ));
    }
    let candidate_span_pairs = source_decomposition
        .spans
        .len()
        .checked_mul(plane_decomposition.spans.len())
        .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "Candidate span-pair overflow"))?;
    if candidate_span_pairs > resource_limit.saturating_mul(resource_limit) {
        return Err(Error::new(
            "BREP_SS_RESOURCE_LIMIT",
            "Candidate span-pair budget exceeded",
        ));
    }
    let mut fragments = Vec::new();
    for span in &source_decomposition.spans {
        let mut cell_candidates = Vec::new();
        for fixed_u in [true, false] {
            let fixed_degree = if fixed_u {
                span.patch.degree_u
            } else {
                span.patch.degree_v
            };
            let varying_degree = if fixed_u {
                span.patch.degree_v
            } else {
                span.patch.degree_u
            };
            let mut coefficients = vec![0.; fixed_degree + 1];
            let mut invariant = true;
            for (i, coefficient_slot) in coefficients.iter_mut().enumerate() {
                for j in 0..=varying_degree {
                    let (u, v) = if fixed_u { (i, j) } else { (j, i) };
                    let point = &span.patch.control_points[u][v];
                    let coefficient = span.patch.weights[u][v]
                        * dot(
                            sub([point[0], point[1], point[2]], plane_origin),
                            plane_normal,
                        );
                    if j == 0 {
                        *coefficient_slot = coefficient;
                    } else if (coefficient - *coefficient_slot).abs()
                        > 32.
                            * f64::EPSILON
                            * coefficient.abs().max((*coefficient_slot).abs()).max(1.)
                    {
                        invariant = false;
                    }
                }
            }
            let slope = coefficients[fixed_degree] - coefficients[0];
            let affine = invariant
                && slope.abs() > context.spatial_bounds().clear_mm
                && coefficients.iter().enumerate().all(|(i, value)| {
                    let expected = coefficients[0] + slope * i as f64 / fixed_degree as f64;
                    value.to_bits() == expected.to_bits()
                        || (value - expected).abs() <= 32. * f64::EPSILON * slope.abs().max(1.)
                });
            if !affine {
                let coefficient_min = coefficients.iter().copied().fold(f64::INFINITY, f64::min);
                let coefficient_max = coefficients
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                if invariant
                    && (coefficients.iter().all(|value| value.abs() <= tolerance)
                        || (coefficient_min <= 0. && coefficient_max >= 0.))
                {
                    return Err(refuse(
                        "Coincident, tangent, or unresolved multi-root span is outside /6",
                    ));
                }
                continue;
            }
            let local_root = -coefficients[0] / slope;
            let fixed_span = if fixed_u { span.span.u } else { span.span.v };
            let fixed_count = if fixed_u {
                source_decomposition.u_breaks.len() - 1
            } else {
                source_decomposition.v_breaks.len() - 1
            };
            let owns = local_root >= 0.
                && (local_root < 1. || (fixed_span + 1 == fixed_count && local_root == 1.));
            if !owns {
                continue;
            }
            let fixed_domain = span.domain[usize::from(!fixed_u)];
            let fixed_parameter =
                fixed_domain[0] + local_root * (fixed_domain[1] - fixed_domain[0]);
            cell_candidates.push((fixed_u, fixed_parameter, coefficients, local_root, slope));
        }
        if cell_candidates.len() > 1 {
            return Err(refuse(
                "One tensor cell has ambiguous multiple branch families",
            ));
        }
        let Some((fixed_u, fixed, coefficients, local_root, slope)) = cell_candidates.pop() else {
            continue;
        };
        let varying_domain = span.domain[usize::from(fixed_u)];
        let curve = source
            .iso(if fixed_u { Axis::U } else { Axis::V }, fixed)?
            .trim(varying_domain[0], varying_domain[1])?;
        let first_trace = Curve {
            degree: 1,
            knots: vec![
                varying_domain[0],
                varying_domain[0],
                varying_domain[1],
                varying_domain[1],
            ],
            control_points: if fixed_u {
                vec![
                    vec![fixed, varying_domain[0]],
                    vec![fixed, varying_domain[1]],
                ]
            } else {
                vec![
                    vec![varying_domain[0], fixed],
                    vec![varying_domain[1], fixed],
                ]
            },
            weights: vec![1., 1.],
            periodic: false,
        };
        first_trace.validate()?;
        let second_trace = affine_plane_trace(plane, &curve, tolerance)?
            .ok_or_else(|| refuse("Intersection branch leaves the affine plane domain"))?;
        let midpoint =
            second_trace.evaluate(0.5 * (second_trace.domain()[0] + second_trace.domain()[1]))?;
        let plane_u = owning_span(&plane_decomposition.u_breaks, midpoint.point[0])
            .ok_or_else(|| refuse("Plane pcurve has no half-open U-span owner"))?;
        let plane_v = owning_span(&plane_decomposition.v_breaks, midpoint.point[1])
            .ok_or_else(|| refuse("Plane pcurve has no half-open V-span owner"))?;
        fragments.push(CertifiedBranchFragment {
            source_faces,
            source_spans: [
                span.span,
                TensorSpanId {
                    u: plane_u,
                    v: plane_v,
                },
            ],
            parameter_interval: varying_domain,
            curve,
            pcurves: [first_trace, second_trace],
            orientations: [BranchOrientation::Increasing, BranchOrientation::Increasing],
            evidence: TransverseSpanEvidence {
                homogeneous_distance_coefficients_bits: coefficients
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
                local_root,
                signed_slope: slope,
                denominator_lower_bound: span.denominator_lower_bound,
                exact_affine_factor: true,
            },
        });
    }
    join_certified_multispan_fragments(
        fragments,
        context,
        [
            source_decomposition.spans.len(),
            plane_decomposition.spans.len(),
        ],
        candidate_span_pairs,
        source_decomposition
            .denominator_lower_bound
            .min(plane_decomposition.denominator_lower_bound),
        resource_limit,
    )
}

/// Fail-closed G6 slice: separated Complete-empty, exact planar intersections,
/// and exact plane preimages on admitted Bezier patches. Generic sampled
/// surface/surface proximity never publishes Complete.
pub fn narrow_transverse_bicubic(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<Report<G6Component>> {
    let options = options.validate()?;
    a.validate()?;
    b.validate()?;
    let source_a = a;
    let source_b = b;
    if G6_MATURITY == G6Maturity::Unavailable {
        return Err(refuse("G6 capability is Unavailable per R1 stop"));
    }
    let a_elev = as_uniform_bicubic(a);
    let b_elev = as_uniform_bicubic(b);
    let (Some(a), Some(b)) = (a_elev.as_ref(), b_elev.as_ref()) else {
        let mut report = Report::default();
        report.unresolved(
            [
                a.knots_u.first().copied().unwrap_or(0.),
                a.knots_u.last().copied().unwrap_or(1.),
                b.knots_u.first().copied().unwrap_or(0.),
                b.knots_u.last().copied().unwrap_or(1.),
            ],
            UnresolvedReason::UnsupportedSurface,
        );
        return Ok(report);
    };
    let (amin, amax) = bbox(a);
    let (bmin, bmax) = bbox(b);
    let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
    if separated {
        return Ok(complete_empty());
    }

    let tol = options.distance_tolerance.max(1e-9);
    let same_cage =
        (0..3).all(|i| (amin[i] - bmin[i]).abs() <= tol && (amax[i] - bmax[i]).abs() <= tol);
    if same_cage {
        let mut report = Report::default();
        report.unresolved(
            vec![0., 1., 0., 1., 0., 1., 0., 1.],
            UnresolvedReason::NearCoincidence,
        );
        return Ok(report);
    }

    if let (Some((oa, na)), Some((ob, nb))) = (planar_support(a, tol), planar_support(b, tol)) {
        let parallel = norm(cross(na, nb)) <= 1e-9;
        if parallel {
            let d = dot(sub(ob, oa), na).abs();
            if d <= tol {
                let mut report = Report::default();
                report.unresolved(
                    vec![0., 1., 0., 1., 0., 1., 0., 1.],
                    UnresolvedReason::NearCoincidence,
                );
                return Ok(report);
            }
            let mut report = Report::default();
            report.unresolved(
                vec![0., 1., 0., 1., 0., 1., 0., 1.],
                UnresolvedReason::TangencyOrMultipleRoot,
            );
            return Ok(report);
        }
        let dir = normalize(cross(na, nb)).ok_or_else(|| refuse("Degenerate plane normals"))?;
        let center = [
            (amin[0].max(bmin[0]) + amax[0].min(bmax[0])) * 0.5,
            (amin[1].max(bmin[1]) + amax[1].min(bmax[1])) * 0.5,
            (amin[2].max(bmin[2]) + amax[2].min(bmax[2])) * 0.5,
        ];
        let da = -dot(sub(center, oa), na);
        let db = -dot(sub(center, ob), nb);
        let nn = dot(na, nb);
        let denom = 1. - nn * nn;
        if denom.abs() <= 1e-15 {
            let mut report = Report::default();
            report.unresolved(
                vec![0., 1., 0., 1., 0., 1., 0., 1.],
                UnresolvedReason::NearCoincidence,
            );
            return Ok(report);
        }
        let alpha = (da - db * nn) / denom;
        let beta = (db - da * nn) / denom;
        let point = [
            center[0] + alpha * na[0] + beta * nb[0],
            center[1] + alpha * na[1] + beta * nb[1],
            center[2] + alpha * na[2] + beta * nb[2],
        ];
        let extent = 0.5
            * ((amax[0] - amin[0])
                .hypot(amax[1] - amin[1])
                .hypot(amax[2] - amin[2])
                .max(1.));
        let start = [
            point[0] - dir[0] * extent,
            point[1] - dir[1] * extent,
            point[2] - dir[2] * extent,
        ];
        let end = [
            point[0] + dir[0] * extent,
            point[1] + dir[1] * extent,
            point[2] + dir[2] * extent,
        ];
        let report = complete_line(start, end);
        verify_complete_report(&report, false)?;
        return Ok(report);
    }

    let pa = planar_support(a, tol);
    let pb = planar_support(b, tol);
    if pa.is_none() {
        if pb.is_some() {
            let context = ToleranceContext::default_valid();
            if let Ok(certificate) =
                certify_exact_planar_iso_intersection(source_a, source_b, &context)
            {
                let report = complete_curve(certificate);
                verify_complete_report(&report, false)?;
                return Ok(report);
            }
        }
    } else if pb.is_none() && pa.is_some() {
        let context = ToleranceContext::default_valid();
        if let Ok(certificate) = certify_exact_planar_iso_intersection(source_b, source_a, &context)
        {
            let report = complete_curve(certificate);
            verify_complete_report(&report, false)?;
            return Ok(report);
        }
    }
    let mut report = Report::default();
    report.unresolved(
        vec![0., 1., 0., 1., 0., 1., 0., 1.],
        UnresolvedReason::NearCoincidence,
    );
    Ok(report)
}

/// Alias for [`narrow_transverse_bicubic`] under the Phase B capability id.
pub fn narrow_transverse_bezier_le3(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<Report<G6Component>> {
    narrow_transverse_bicubic(a, b, options)
}
