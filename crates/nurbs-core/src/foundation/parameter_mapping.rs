//! Native rational parameter maps, their monotonicity proofs and materialization.
use super::{
    Curve, Result, ToleranceContext, bernstein_product, binomial, budget_controls, check, context,
    distance, interval, numeric,
};
use math_core::next_up;
#[cfg(feature = "codec")]
pub mod serialization;

#[derive(Clone)]
pub struct MapPiece {
    pub domain: [f64; 2],
    pub range: [f64; 2],
    pub values: Vec<f64>,
    pub weights: Vec<f64>,
}
impl MapPiece {
    pub fn validate(&self) -> Result<()> {
        check(
            (2..=26).contains(&self.values.len()) && self.values.len() == self.weights.len(),
            "Map piece needs 2..26 matched controls and weights",
        )?;
        check(
            self.domain[0] < self.domain[1]
                && self.range[0] < self.range[1]
                && self
                    .domain
                    .iter()
                    .chain(self.range.iter())
                    .all(|x| x.is_finite()),
            "Map domains and ranges must increase finitely",
        )?;
        check(
            self.weights
                .iter()
                .all(|w| w.is_finite() && *w >= 1e-12 && *w <= 1e12),
            "Map weights must be positive and bounded",
        )?;
        check(
            self.values.iter().all(|value| value.is_finite()),
            "Map controls must be finite",
        )?;
        check(
            self.values.first() == Some(&self.range[0])
                && self.values.last() == Some(&self.range[1]),
            "Map endpoint controls must equal the declared range",
        )?;
        Ok(())
    }
}
#[derive(Clone)]
pub enum ParameterMapping {
    Pieces(Vec<MapPiece>),
    Composition {
        factors: Vec<ParameterMapping>,
        /// Optional piece support declared by legacy composite maps for join checks.
        piece_support: Option<Vec<MapPiece>>,
    },
}
impl ParameterMapping {
    fn pieces(&self) -> Result<&[MapPiece]> {
        let pieces = match self {
            Self::Pieces(pieces) => pieces,
            Self::Composition {
                piece_support: Some(pieces),
                ..
            } => pieces,
            Self::Composition {
                piece_support: None,
                ..
            } => return Err(crate::input("Reparameterization requires pieces")),
        };
        check(
            !pieces.is_empty() && pieces.len() <= 64,
            "Piecewise reparameterization needs 1..64 pieces",
        )?;
        for piece in pieces {
            piece.validate()?;
        }
        Ok(pieces)
    }
}
pub struct MapPieceCertificate {
    pub domain: [f64; 2],
    pub range: [f64; 2],
    pub derivative_numerator_bounds: [f64; 2],
    pub denominator_bounds: [f64; 2],
}
pub enum ParameterMapProof {
    Piecewise(Vec<MapPieceCertificate>),
    Composition(Vec<ParameterMapCertificate>),
}
pub struct ParameterMapCertificate {
    pub mapping: ParameterMapping,
    pub proof: ParameterMapProof,
    pub tolerance: ToleranceContext,
}
pub(super) fn certificate_support(certificate: &ParameterMapCertificate) -> Option<([f64; 2], [f64; 2])> {
    match &certificate.proof {
        ParameterMapProof::Piecewise(pieces) => {
            let first = pieces.first()?;
            let last = pieces.last()?;
            Some(([first.domain[0], last.domain[1]], [first.range[0], last.range[1]]))
        }
        ParameterMapProof::Composition(factors) => {
            let (domain, _) = certificate_support(factors.first()?)?;
            let (_, range) = certificate_support(factors.last()?)?;
            Some((domain, range))
        }
    }
}
pub struct ReparameterizedEvaluation {
    pub parameter: f64,
    pub source_parameter: f64,
    pub evaluation: crate::curve::Evaluation,
    pub certificate: ParameterMapCertificate,
}
pub enum MappingTree {
    Composition {
        depth: usize,
        children: Vec<MappingTree>,
    },
    Piecewise {
        piece_count: usize,
    },
}
pub struct MaterializedCurve {
    pub curve: Curve,
    pub degree_growth: usize,
    pub periodic_cover: bool,
    pub tree: MappingTree,
    pub map_certificate: ParameterMapCertificate,
    pub tolerance: ToleranceContext,
}

fn evaluate_map_piece(piece: &MapPiece, u: f64) -> f64 {
    let t = (u - piece.domain[0]) / (piece.domain[1] - piece.domain[0]);
    let degree = piece.values.len() - 1;
    let mut numerator = 0.;
    let mut denominator = 0.;
    for i in 0..=degree {
        let basis = binomial(degree, i) * t.powi(i as i32) * (1. - t).powi((degree - i) as i32);
        numerator += basis * piece.weights[i] * piece.values[i];
        denominator += basis * piece.weights[i];
    }
    numerator / denominator
}
fn map_derivative_coefficients(piece: &MapPiece) -> Vec<f64> {
    let homogeneous = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(x, w)| [x * w, *w])
        .collect::<Vec<_>>();
    let degree = piece.values.len() - 1;
    let derivative = homogeneous
        .array_windows()
        .map(|[a, b]| [degree as f64 * (b[0] - a[0]), degree as f64 * (b[1] - a[1])])
        .collect::<Vec<_>>();
    let first = bernstein_product(
        &derivative.iter().map(|x| x[0]).collect::<Vec<_>>(),
        &piece.weights,
    );
    let second = bernstein_product(
        &homogeneous.iter().map(|x| x[0]).collect::<Vec<_>>(),
        &derivative.iter().map(|x| x[1]).collect::<Vec<_>>(),
    );
    first
        .into_iter()
        .zip(second)
        .map(|(x, y)| (x - y) / (piece.domain[1] - piece.domain[0]))
        .collect()
}
fn evaluate_mapping(mapping: &ParameterMapping, u: f64) -> Result<f64> {
    if let ParameterMapping::Composition {
        factors: composition,
        ..
    } = mapping
    {
        check(
            !composition.is_empty() && composition.len() <= 16,
            "Mapping composition needs 1..16 factors",
        )?;
        return composition
            .iter()
            .try_fold(u, |parameter, factor| evaluate_mapping(factor, parameter));
    }
    let pieces = mapping.pieces()?;
    let piece = pieces
        .iter()
        .find(|piece| u >= piece.domain[0] && u <= piece.domain[1])
        .ok_or_else(|| crate::input("Reparameterized query lies outside the mapping domain"))?;
    Ok(evaluate_map_piece(piece, u))
}
pub fn certify_reparameterization_report(
    mapping: &ParameterMapping,
    tolerance: Option<ToleranceContext>,
) -> Result<ParameterMapCertificate> {
    if let ParameterMapping::Composition {
        factors: composition,
        ..
    } = mapping
    {
        check(
            !composition.is_empty() && composition.len() <= 16,
            "Mapping composition needs 1..16 factors",
        )?;
        let factors = composition
            .iter()
            .map(|factor| certify_reparameterization_report(factor, None))
            .collect::<Result<Vec<_>>>()?;
        for pair in factors.windows(2) {
            let (_, range) = certificate_support(&pair[0]).ok_or_else(|| crate::input("Missing composed map support"))?;
            let (domain, _) = certificate_support(&pair[1]).ok_or_else(|| crate::input("Missing composed map support"))?;
            check(range == domain, "Composed mapping ranges and domains must match exactly")?;
        }
        let tolerance = context(tolerance);
        return Ok(ParameterMapCertificate {
            mapping: mapping.clone(),
            proof: ParameterMapProof::Composition(factors),
            tolerance,
        });
    }
    let pieces = mapping.pieces()?;
    for pair in pieces.windows(2) {
        check(
            pair[0].domain[1] == pair[1].domain[0] && pair[0].range[1] == pair[1].range[0],
            "Piecewise mapping must be contiguous in domain and range",
        )?;
    }
    let certificates = pieces
        .iter()
        .map(|piece| {
            let derivative = map_derivative_coefficients(piece);
            let bounds = interval(derivative.iter().copied());
            numeric(
                bounds[0] > 0.,
                "Rational map derivative is not certified strictly positive",
            )?;
            Ok(MapPieceCertificate {
                domain: piece.domain,
                range: piece.range,
                derivative_numerator_bounds: bounds,
                denominator_bounds: interval(piece.weights.iter().copied()),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let tolerance = context(tolerance);
    Ok(ParameterMapCertificate {
        mapping: mapping.clone(),
        proof: ParameterMapProof::Piecewise(certificates),
        tolerance,
    })
}
pub fn evaluate_reparameterized_curve_report(
    curve: &Curve,
    mapping: &ParameterMapping,
    u: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<ReparameterizedEvaluation> {
    curve.validate()?;
    let certificate = certify_reparameterization_report(mapping, tolerance)?;
    let source_parameter = evaluate_mapping(mapping, u)?;
    let evaluation = curve.evaluate(source_parameter)?;
    Ok(ReparameterizedEvaluation {
        parameter: u,
        source_parameter,
        evaluation,
        certificate,
    })
}
fn compose_curve_with_piece(curve: &Curve, piece: &MapPiece) -> Result<Curve> {
    check(
        curve.degree <= 8 && piece.values.len() <= 9,
        "Admitted composition requires degree ≤8 map and curve",
    )?;
    let [a, b] = curve.domain();
    check(
        (piece.range[0] - a).abs() <= 64. * f64::EPSILON
            && (piece.range[1] - b).abs() <= 64. * f64::EPSILON
            || (piece.range[0] >= a && piece.range[1] <= b),
        "Map range must lie in the active curve domain",
    )?;
    let restricted = if (piece.range[0] - a).abs() <= 64. * f64::EPSILON
        && (piece.range[1] - b).abs() <= 64. * f64::EPSILON
    {
        curve.clone()
    } else {
        curve.trim(piece.range[0], piece.range[1])?
    };
    let segments = restricted.decompose()?;
    check(
        segments.len() == 1,
        "Composition requires a single Bézier span after restriction",
    )?;
    let span = segments[0].definition().clone();
    let dimension = span.control_points[0].len();
    let n = span.degree;
    // Map range normalized into [0,1] for the active Bézier parameter of `span`.
    let map_p: Vec<f64> = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(value, weight)| {
            ((value - piece.range[0]) / (piece.range[1] - piece.range[0])) * weight
        })
        .collect();
    let map_q = piece.weights.clone();
    // Cleared form uses P^i (Q-P)^{n-i}; for polynomial maps Q≡1 this is φ^i (1-φ)^{n-i}.
    let map_one_minus: Vec<f64> = map_q.iter().zip(&map_p).map(|(q, p)| q - p).collect();
    let mut p_powers = vec![vec![1.]];
    let mut one_powers = vec![vec![1.]];
    for _ in 1..=n {
        p_powers.push(bernstein_product(p_powers.last().unwrap(), &map_p));
        one_powers.push(bernstein_product(
            one_powers.last().unwrap(),
            &map_one_minus,
        ));
    }
    let mut homogeneous = vec![Vec::new(); dimension + 1];
    for axis in 0..=dimension {
        let coeffs: Vec<f64> = if axis == dimension {
            span.weights.clone()
        } else {
            span.control_points
                .iter()
                .zip(&span.weights)
                .map(|(p, w)| p[axis] * w)
                .collect()
        };
        let mut composed = vec![0.; n * (piece.values.len() - 1) + 1];
        for (i, coefficient) in coeffs.iter().enumerate() {
            let term = bernstein_product(&p_powers[i], &one_powers[n - i]);
            for (target, value) in composed.iter_mut().zip(term) {
                *target += value * binomial(n, i) * coefficient;
            }
        }
        homogeneous[axis] = composed;
    }
    let degree = homogeneous[0].len() - 1;
    check(degree <= 25, "Composed degree exceeds 25")?;
    budget_controls(degree + 1)?;
    let weights = homogeneous[dimension].clone();
    numeric(
        weights.iter().all(|w| *w >= 1e-12 && *w <= 1e12),
        "Composed weights left the positive window",
    )?;
    let control_points = (0..=degree)
        .map(|i| {
            (0..dimension)
                .map(|axis| homogeneous[axis][i] / weights[i])
                .collect()
        })
        .collect();
    let domain = piece.domain;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(domain[0], degree + 1)
            .chain(std::iter::repeat_n(domain[1], degree + 1))
            .collect(),
        control_points,
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}
fn materialize_pieces(curve: &Curve, pieces: &[MapPiece]) -> Result<(Curve, usize)> {
    let mut composed = Vec::new();
    let mut degree_growth = 0_usize;
    for piece in pieces {
        let part = compose_curve_with_piece(curve, piece)?;
        degree_growth = degree_growth.max(part.degree.saturating_sub(curve.degree));
        composed.push(part);
    }
    let mut result = composed[0].clone();
    for next in composed.into_iter().skip(1) {
        check(
            result.degree == next.degree,
            "Piecewise composition produced unequal degrees",
        )?;
        let left = result.evaluate(result.domain()[1])?.point;
        let right = next.evaluate(next.domain()[0])?.point;
        check(
            distance(&left, &right) <= 1e-9,
            "Composed pieces are not C0 joinable",
        )?;
        let mut knots = result.knots[..result.knots.len() - 1].to_vec();
        knots.extend(next.knots[next.degree + 1..].iter().copied());
        let mut controls = result.control_points.clone();
        controls.extend(next.control_points[1..].iter().cloned());
        let mut weights = result.weights.clone();
        weights.extend(next.weights[1..].iter().copied());
        budget_controls(controls.len())?;
        result = Curve {
            degree: result.degree,
            knots,
            control_points: controls,
            weights,
            periodic: false,
        };
        result.validate()?;
    }
    Ok((result, degree_growth))
}
fn materialize_mapping_tree(
    curve: &Curve,
    mapping: &ParameterMapping,
    depth: usize,
) -> Result<(Curve, usize, MappingTree)> {
    check(depth <= 8, "Nested composition depth exceeds 8")?;
    if let ParameterMapping::Composition { factors: parts, .. } = mapping {
        check(
            !parts.is_empty() && parts.len() <= 8,
            "Nested composition needs 1..8 maps",
        )?;
        let mut current = curve.clone();
        let mut growth = 0_usize;
        let mut children = Vec::new();
        for part in parts {
            let (next, part_growth, child) = materialize_mapping_tree(&current, part, depth + 1)?;
            growth = growth.saturating_add(part_growth);
            children.push(child);
            current = next;
        }
        return Ok((
            current,
            growth,
            MappingTree::Composition { depth, children },
        ));
    }
    let pieces = mapping.pieces()?;
    let (result, growth) = materialize_pieces(curve, &pieces)?;
    Ok((
        result,
        growth,
        MappingTree::Piecewise {
            piece_count: pieces.len(),
        },
    ))
}
pub fn materialize_reparameterized_curve_report(
    curve: &Curve,
    mapping: &ParameterMapping,
    tolerance: Option<ToleranceContext>,
) -> Result<MaterializedCurve> {
    curve.validate()?;
    let certificate = certify_reparameterization_report(mapping, None)?;
    let (source, periodic_cover) = if curve.periodic {
        let [a, b] = curve.domain();
        let mut open = curve.trim(a, b)?;
        open.periodic = false;
        (open, true)
    } else {
        (curve.clone(), false)
    };
    let (mut result, degree_growth, tree) = materialize_mapping_tree(&source, mapping, 0)?;
    if periodic_cover {
        // Close the fundamental cover when endpoints match to restore periodic storage.
        let [a, b] = result.domain();
        let left = result.evaluate(a)?.point;
        let right = result.evaluate(b)?.point;
        check(
            distance(&left, &right) <= 1e-9,
            "Periodic composition cover is not closed",
        )?;
        result.periodic = true;
        result.validate()?;
    }
    let tolerance = context(tolerance);
    Ok(MaterializedCurve {
        curve: result,
        degree_growth,
        periodic_cover,
        tree,
        map_certificate: certificate,
        tolerance,
    })
}

/// Certified Möbius (linear fractional) reparameterization of a rational
/// curve. The map t = (a·s + b)/(c·s + d) is applied in homogeneous space via
/// the existing piecewise rational-map materialization, so the degree is
/// preserved and the image of the curve is unchanged up to the certified
/// `deviation` bound (sampled with outward rounding). Multi-span curves are
/// materialized as joined Bézier pieces: interior knots are stored at full
/// (C⁰) multiplicity, so the control count may grow.
pub struct MobiusReparameterization {
    pub curve: Curve,
    /// Map coefficients [a, b, c, d] with t = (a·s + b)/(c·s + d).
    pub coefficients: [f64; 4],
    pub old_domain: [f64; 2],
    pub new_domain: [f64; 2],
    /// Certified upper bound on the Hausdorff deviation between the source
    /// curve and the reparameterized curve (max sample distance, rounded up).
    pub deviation: f64,
    pub map_certificate: ParameterMapCertificate,
    pub tolerance: ToleranceContext,
}

/// Sampled image-preservation bound for a Möbius reparameterization:
/// max over a uniform grid of ‖C(t(s)) − C̃(s)‖ with outward rounding.
fn mobius_deviation_bound(
    source: &Curve,
    coefficients: [f64; 4],
    image: &Curve,
) -> Result<f64> {
    let [a, b, c, d] = coefficients;
    let [s0, s1] = image.domain();
    let mut maximum = 0_f64;
    for i in 0..=256 {
        let s = s0 + (s1 - s0) * i as f64 / 256.;
        let t = (a * s + b) / (c * s + d);
        maximum = maximum.max(distance(
            &source.evaluate(t)?.point,
            &image.evaluate(s)?.point,
        ));
    }
    Ok(next_up(maximum))
}

/// Inverse of the Möbius map: s = (d·t − b)/(a − c·t).
fn mobius_inverse(coefficients: [f64; 4], t: f64) -> f64 {
    let [a, b, c, d] = coefficients;
    (d * t - b) / (a - c * t)
}

/// Refit the Möbius-reparameterized curve on the mapped knot vector by
/// Greville interpolation in homogeneous space. The reparameterized curve
/// lies in the target spline space (Möbius reparameterization preserves the
/// degree and the per-span rationality and continuity), so the interpolant is
/// exact up to solve conditioning; the caller certifies with a sampled
/// deviation bound.
fn refit_mobius(
    source: &Curve,
    coefficients: [f64; 4],
    degree: usize,
    knots: Vec<f64>,
) -> Result<Curve> {
    let count = knots.len() - degree - 1;
    budget_controls(count)?;
    let parameters = (0..count)
        .map(|i| knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64)
        .collect::<Vec<_>>();
    let matrix = parameters
        .iter()
        .map(|&u| crate::curve::basis(degree, &knots, count, u, false).map(|basis| basis.basis))
        .collect::<Result<Vec<_>>>()?;
    let dimension = source.control_points[0].len();
    // The polynomial-space homogeneous representative of the reparameterized
    // curve is H̃(s) = |c·s + d|^p · H(t(s)): clearing the Möbius denominator
    // makes it piecewise polynomial of degree p with the source continuity,
    // hence a member of the target spline space. The absolute value is one
    // global sign (c·s + d = det/(a − c·t) keeps one sign), so the projected
    // curve is unchanged and weights stay positive.
    let p = degree as i32;
    let mut values = Vec::new();
    for &u in &parameters {
        let t = (coefficients[0] * u + coefficients[1]) / (coefficients[2] * u + coefficients[3]);
        let scale = (coefficients[2] * u + coefficients[3]).abs().powi(p);
        let basis =
            crate::curve::basis(source.degree, &source.knots, source.control_points.len(), t, false)?
                .basis;
        let weight = basis
            .iter()
            .zip(&source.weights)
            .map(|(b, w)| b * w)
            .sum::<f64>()
            * scale;
        let point = source.evaluate(t)?.point;
        values.push(
            point
                .into_iter()
                .map(|x| x * weight)
                .chain(std::iter::once(weight))
                .collect(),
        );
    }
    let homogeneous = super::solve(matrix, values)?;
    let weights = homogeneous
        .iter()
        .map(|value| value[dimension])
        .collect::<Vec<_>>();
    numeric(
        weights
            .iter()
            .all(|weight| *weight >= 1e-12 && *weight <= 1e12),
        "Möbius reparameterization produced inadmissible weights",
    )?;
    let control_points = homogeneous
        .iter()
        .map(|value| {
            value[..dimension]
                .iter()
                .map(|x| x / value[dimension])
                .collect()
        })
        .collect();
    let output = Curve {
        degree,
        knots,
        control_points,
        weights,
        periodic: false,
    };
    output.validate()?;
    Ok(output)
}

/// Möbius map t = (a·s + b)/(c·s + d), ad − bc ≠ 0, monotone increasing on the
/// curve's active domain (the denominator c·t + d must keep one strict sign on
/// the domain and the induced new domain must increase). Knots are mapped
/// through the inverse map (multiplicities and continuity preserved), control
/// points and weights are recomputed in homogeneous space, and the rational
/// image of the curve is unchanged — certified by the returned `deviation`
/// bound and the `map_certificate` monotonicity proof. Periodic curves are
/// materialized as their clamped fundamental cover.
pub fn mobius_reparameterize_curve_report(
    curve: &Curve,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
) -> Result<MobiusReparameterization> {
    curve.validate()?;
    check(
        [a, b, c, d].iter().all(|value| value.is_finite()),
        "Möbius coefficients must be finite",
    )?;
    let determinant = a * d - b * c;
    numeric(
        determinant.is_finite() && determinant != 0.,
        "Möbius map requires ad − bc ≠ 0",
    )?;
    let old_domain = curve.domain();
    let [t0, t1] = old_domain;
    // Monotonicity: the denominator must not vanish anywhere on the closed
    // domain; linearity reduces this to strict same-sign endpoint values.
    let g0 = c * t0 + d;
    let g1 = c * t1 + d;
    numeric(
        g0 * g1 > 0.,
        "Möbius denominator has a root inside the curve domain",
    )?;
    numeric(
        (a - c * t0) * (a - c * t1) > 0.,
        "Möbius inverse map has a pole inside the curve domain",
    )?;
    let s0 = mobius_inverse([a, b, c, d], t0);
    let s1 = mobius_inverse([a, b, c, d], t1);
    check(
        s0.is_finite() && s1.is_finite() && s0 < s1,
        "Möbius map must be increasing with a finite new domain",
    )?;
    // Clamped non-periodic source: exterior knots do not participate in the
    // rational map and periodic storage cannot express the moved knots.
    let source = crate::curve::clamped(curve)?;
    // One rational degree-1 piece per Bézier span of the source curve. On the
    // span [u0, u1] the normalized map is τ = λσ/(1 − σ + λσ), which matches
    // the global Möbius map for λ = τ(½)/(1 − τ(½)). The pieces are the
    // monotonicity certificate; the curve itself is refit on the mapped knots.
    let mut breaks = vec![t0];
    for &knot in &source.knots {
        if knot > t0 && knot < t1 && *breaks.last().unwrap() != knot {
            breaks.push(knot);
        }
    }
    breaks.push(t1);
    let mut pieces = Vec::with_capacity(breaks.len() - 1);
    for span in breaks.array_windows() {
        let [u0, u1] = *span;
        let lo = mobius_inverse([a, b, c, d], u0);
        let hi = mobius_inverse([a, b, c, d], u1);
        check(
            lo.is_finite() && hi.is_finite() && lo < hi,
            "Möbius inverse map collapsed a knot span",
        )?;
        let middle = (lo + hi) * 0.5;
        let t_middle = (a * middle + b) / (c * middle + d);
        let tau = (t_middle - u0) / (u1 - u0);
        numeric(
            tau > 0. && tau < 1.,
            "Möbius map is not monotone on a knot span",
        )?;
        let lambda = tau / (1. - tau);
        numeric(
            lambda.is_finite() && (1e-12..=1e12).contains(&lambda),
            "Möbius piece weight left the positive window",
        )?;
        pieces.push(MapPiece {
            domain: [lo, hi],
            range: [u0, u1],
            values: vec![u0, u1],
            weights: vec![1., lambda],
        });
    }
    let map_certificate = certify_reparameterization_report(&ParameterMapping::Pieces(pieces), None)?;
    let new_knots = source
        .knots
        .iter()
        .map(|&knot| mobius_inverse([a, b, c, d], knot))
        .collect::<Vec<_>>();
    check(
        source
            .knots
            .windows(2)
            .zip(new_knots.windows(2))
            .all(|(old, new)| old[0] == old[1] || new[0] < new[1]),
        "Möbius map collapsed distinct knots at coordinate precision",
    )?;
    let result = refit_mobius(&source, [a, b, c, d], source.degree, new_knots)?;
    let deviation = mobius_deviation_bound(&source, [a, b, c, d], &result)?;
    Ok(MobiusReparameterization {
        new_domain: result.domain(),
        curve: result,
        coefficients: [a, b, c, d],
        old_domain,
        deviation,
        map_certificate,
        tolerance: context(None),
    })
}

/// Normalizes endpoint weights to w₀ = wₙ = 1 via the Möbius map that fixes
/// the domain endpoints (the standard rational-curve reparameterization
/// τ = λσ/(1 − σ + λσ) with λ = (wₙ/w₀)^(−1/p)). Control-point positions are
/// preserved in homogeneous space; interior weights and knots move under the
/// map. The image is unchanged, certified by the returned `deviation` bound.
/// Periodic curves are rejected: clamp them first.
pub fn normalize_endpoint_weights_report(curve: &Curve) -> Result<MobiusReparameterization> {
    curve.validate()?;
    check(
        !curve.periodic,
        "Endpoint weight normalization requires a non-periodic curve",
    )?;
    let p = curve.degree;
    let w0 = curve.weights[0];
    let wn = *curve.weights.last().unwrap();
    let [t0, t1] = curve.domain();
    // The reparameterized homogeneous representative is H̃(s) = |c·s + d|^p ·
    // H(t(s)). For the endpoint-fixing map with normalized form
    // τ = λσ/(1 − σ + λσ), the endpoint weights become (t₁−t₀)^p·w₀ and
    // λ^p·(t₁−t₀)^p·wₙ, so λ = (w₀/wₙ)^(1/p) equalizes them.
    let lambda = (w0 / wn).powf(1. / p as f64);
    numeric(
        lambda.is_finite() && (1e-12..=1e12).contains(&lambda),
        "Endpoint weight normalization left the admissible map window",
    )?;
    // Endpoint-fixing Möbius map with normalized form τ = λσ/(1 − σ + λσ).
    let a = lambda * t1 - t0;
    let b = t0 * t1 * (1. - lambda);
    let c = lambda - 1.;
    let d = t1 - lambda * t0;
    let mut report = mobius_reparameterize_curve_report(curve, a, b, c, d)?;
    // Scale weights so the first endpoint weight is exactly 1; the map above
    // already made the two endpoint weights equal, so the last one snaps.
    let scale = report.curve.weights[0];
    numeric(
        scale.is_finite() && scale > 0.,
        "Endpoint weight normalization produced a degenerate weight",
    )?;
    for weight in &mut report.curve.weights {
        *weight /= scale;
    }
    let last = report.curve.weights.len() - 1;
    numeric(
        (report.curve.weights[last] - 1.).abs() <= 1e-9,
        "Endpoint weights did not equalize under the Möbius map",
    )?;
    report.curve.weights[last] = 1.;
    report.curve.validate()?;
    report.deviation = mobius_deviation_bound(curve, [a, b, c, d], &report.curve)?;
    Ok(report)
}

#[cfg(test)]
#[path = "tests/parameter_mapping.rs"]
mod tests;


