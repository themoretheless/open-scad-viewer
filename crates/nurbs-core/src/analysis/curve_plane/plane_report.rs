//! Plane report: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn make(
    c: &Curve,
    span: usize,
    d: [f64; 2],
    n: [f64; 3],
    o: f64,
    exact: &mut ExactEndpoints<'_>,
) -> Result<Pending> {
    if c.control_points[span - c.degree..=span]
        .iter()
        .all(|p| exact_plane_zero(p, n, o))
    {
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [0., 0.],
                classification: Classification::Coincident,
                stop_reason: StopReason::ExactCoincidence,
                root_multiplicity: None,
                isolated_root_bounds: None,
                proof: Some(Proof::CoplanarControls),
            },
            unique: false,
            root_proof: None,
        });
    }
    let j = curve_jets::enclose(c, span, d)?;
    let mut f = project(&j[0], n, o)?;
    let endpoint_residuals = if c.degree == 1 {
        let a = project(&curve_jets::endpoint(c, span, d, false)?[0], n, o)?;
        let b = project(&curve_jets::endpoint(c, span, d, true)?[0], n, o)?;
        // Every scalar projection of a positive-weight rational line is
        // monotone (possibly constant). Endpoint bounds cover the whole cell,
        // independently of whether its derivative sign can be certified.
        f = f.intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
        Some((a, b))
    } else {
        None
    };
    let linear_root_bounds = if let Some((a, b)) = endpoint_residuals {
        if (a.hi < 0. && b.lo > 0.) || (b.hi < 0. && a.lo > 0.) {
            linear_parameter_bounds(c, span, d, n, o)?
        } else {
            None
        }
    } else {
        None
    };
    let mut excluded = f.lo > 0. || f.hi < 0.;
    let mut exclusion_proof = Proof::ResidualBounds;
    if let Some((at_start, order)) = bernstein_endpoint_order(c, span, d, n, o)? {
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [f.lo, f.hi],
                classification: match (at_start, order) {
                    (true, 1) => Classification::RootAtStart,
                    (false, 1) => Classification::RootAtEnd,
                    (true, 2) => Classification::TangencyAtStart,
                    (false, 2) => Classification::TangencyAtEnd,
                    (true, _) => Classification::HigherOrderContactAtStart,
                    (false, _) => Classification::HigherOrderContactAtEnd,
                },
                stop_reason: if order == 1 {
                    StopReason::ExactEndpoint
                } else if order == 2 {
                    StopReason::ExactTangency
                } else {
                    StopReason::ExactHigherOrderContact
                },
                root_multiplicity: Some(order),
                isolated_root_bounds: None,
                proof: Some(Proof::EndpointCoefficients),
            },
            unique: false,
            root_proof: None,
        });
    }
    if let Some(order) = exact_midpoint_power(c, span, d, n, o) {
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [f.lo, f.hi],
                classification: if order == 2 {
                    Classification::TangencyAtMidpoint
                } else {
                    Classification::HigherOrderContactAtMidpoint
                },
                stop_reason: if order == 2 {
                    StopReason::ExactTangency
                } else {
                    StopReason::ExactHigherOrderContact
                },
                root_multiplicity: Some(order),
                isolated_root_bounds: None,
                proof: Some(if c.degree == 2 {
                    Proof::QuadraticSquare
                } else {
                    Proof::BernsteinMidpointPower
                }),
            },
            unique: false,
            root_proof: None,
        });
    }
    if let Some(proof) = quadratic::inspect(c, span, d, n, o, exact) {
        let excluded = matches!(proof, quadratic::QuadraticProof::Excluded);
        let bounds = match proof {
            quadratic::QuadraticProof::DoubleRoot(bounds) => Some(bounds),
            quadratic::QuadraticProof::Excluded => None,
        };
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [f.lo, f.hi],
                classification: if excluded {
                    Classification::Excluded
                } else {
                    Classification::Unresolved
                },
                stop_reason: if excluded {
                    StopReason::ExactQuadraticDiscriminant
                } else {
                    StopReason::ParameterTolerance
                },
                root_multiplicity: None,
                isolated_root_bounds: bounds,
                proof: if excluded {
                    Some(Proof::QuadraticDiscriminant)
                } else {
                    None
                },
            },
            unique: false,
            root_proof: if excluded {
                None
            } else {
                Some(Proof::QuadraticDiscriminant)
            },
        });
    }
    let mut unique = false;
    let mut root_proof = None;
    let mut endpoint = None;
    let mut rational_endpoint = false;
    if !excluded {
        let derivative = if c.degree == 1 {
            // For positive weights, the projected rational line derivative
            // has the sign of n·(b-a) everywhere. Its scalar factor is
            // wa*wb*(hi-lo)/[wa(hi-t)+wb(t-lo)]² > 0. No quotient dependency.
            let a = &c.control_points[span - 1];
            let b = &c.control_points[span];
            let direction = (0..3)
                .map(|axis| Interval::point(b[axis]).sub(Interval::point(a[axis])))
                .collect::<Result<Vec<_>>>()?;
            project(&direction, n, 0.)?
        } else {
            project(&j[1], n, 0.)?
        };
        if derivative.lo > 0. || derivative.hi < 0. {
            let lo = c.knots[span];
            let hi = c.knots[span + 1];
            // Multiplicity >= degree makes the corresponding one-sided
            // endpoint equal to an original control, without knot insertion.
            if d[0] == lo
                && c.knots[span - c.degree + 1] == lo
                && exact_plane_zero(&c.control_points[span - c.degree], n, o)
            {
                endpoint = Some(Classification::RootAtStart);
            } else if d[1] == hi
                && c.knots[span + c.degree] == hi
                && (hi == c.domain()[1] || c.knots.iter().filter(|&&k| k == hi).count() <= c.degree)
                && exact_plane_zero(&c.control_points[span], n, o)
            {
                endpoint = Some(Classification::RootAtEnd);
            }
            let (a, b) = if let Some(pair) = endpoint_residuals {
                pair
            } else {
                (
                    project(&curve_jets::endpoint(c, span, d, false)?[0], n, o)?,
                    project(&curve_jets::endpoint(c, span, d, true)?[0], n, o)?,
                )
            };
            if endpoint.is_none() {
                if exact_linear_parameter(c, span, d[0], n, o)
                    || exact_quadratic_knot(c, d[0], n, o)
                    || crate::exact_curve_segments::plane_incidence(c, span, d[0], n, o)
                        == Some(true)
                {
                    endpoint = Some(Classification::RootAtStart);
                } else if exact_linear_parameter(c, span, d[1], n, o)
                    || exact_quadratic_knot(c, d[1], n, o)
                    || crate::exact_curve_segments::plane_incidence(c, span, d[1], n, o)
                        == Some(true)
                {
                    endpoint = Some(Classification::RootAtEnd);
                }
            }
            if endpoint.is_none() {
                if a.lo <= 0. && a.hi >= 0. && exact.incidence(span, d[0]) == Some(true) {
                    endpoint = Some(Classification::RootAtStart);
                    rational_endpoint = true;
                } else if b.lo <= 0. && b.hi >= 0. && exact.incidence(span, d[1]) == Some(true) {
                    endpoint = Some(Classification::RootAtEnd);
                    rational_endpoint = true;
                }
            }
            unique = (a.hi < 0. && b.lo > 0.) || (b.hi < 0. && a.lo > 0.);
            if unique {
                root_proof = Some(Proof::MonotonicityWithSignChange);
            }
        }
    }
    if !excluded && !unique && endpoint.is_none() {
        let a = project(&curve_jets::endpoint(c, span, d, false)?[0], n, o)?;
        let b = project(&curve_jets::endpoint(c, span, d, true)?[0], n, o)?;
        let opposite = (a.hi < 0. && b.lo > 0.) || (b.hi < 0. && a.lo > 0.);
        let same = (a.lo > 0. && b.lo > 0.) || (a.hi < 0. && b.hi < 0.);
        if opposite || same {
            let variations = bernstein_variation_upper(c, span, d, n, o)?;
            if same && variations == 0 {
                excluded = true;
                exclusion_proof = Proof::BernsteinZeroVariation;
            }
            if opposite && variations <= 1 {
                // Bernstein Descartes bounds positive interior roots (counting
                // multiplicity). Strict endpoint signs prove existence; <=1
                // therefore proves exactly one simple root without monotonicity.
                unique = true;
                root_proof = Some(Proof::BernsteinOneVariation);
            }
        }
    }
    Ok(Pending {
        cell: Cell {
            domain: d,
            span,
            residual_bounds: [f.lo, f.hi],
            classification: if let Some(endpoint) = endpoint {
                endpoint
            } else if excluded {
                Classification::Excluded
            } else {
                Classification::Unresolved
            },
            stop_reason: if endpoint.is_some() {
                StopReason::ExactEndpoint
            } else {
                StopReason::ParameterTolerance
            },
            root_multiplicity: endpoint.map(|_| 1),
            isolated_root_bounds: linear_root_bounds,
            proof: if excluded {
                Some(exclusion_proof)
            } else {
                endpoint.map(|_| {
                    if rational_endpoint {
                        Proof::RationalDeBoorEndpointWithMonotonicity
                    } else {
                        Proof::EndpointWithMonotonicity
                    }
                })
            },
        },
        unique,
        root_proof,
    })
}
