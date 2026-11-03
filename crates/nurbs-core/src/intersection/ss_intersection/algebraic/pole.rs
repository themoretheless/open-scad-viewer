//! Exact collapsed source-boundary strata and denominator-safe saturation.
//! A source pole proof alone is not proof that the other support meets it.
use super::polynomial::SparseSsPolynomial;
use super::*;
#[derive(Clone, Debug)]
pub struct QualifiedSsPoleStratum {
    original: QualifiedOriginalSsIdeal,
    support: usize,
    axis: usize,
    shore: usize,
    point: [Q; 3],
    limits: ExactSsAlgebraLimits,
    work: usize,
}
impl QualifiedSsPoleStratum {
    pub fn original(&self) -> &QualifiedOriginalSsIdeal {
        &self.original
    }
    pub fn support(&self) -> usize {
        self.support
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn shore(&self) -> usize {
        self.shore
    }
    pub fn point(&self) -> &[Q; 3] {
        &self.point
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify(&self) -> Result<()> {
        self.original.reverify()?;
        let fresh = qualify_ss_pole_stratum(
            &self.original,
            self.support,
            self.axis,
            self.shore,
            self.limits,
        )?;
        check(
            self.point == fresh.point && self.work == fresh.work,
            "Original source pole boundary or its constant world image changed",
        )
    }
}
fn specialize(
    p: &ExactSsPolynomial,
    axis: usize,
    value: &Q,
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let mut out = ExactSsPolynomial::default();
    for (e, c) in &p.terms {
        let mut coefficient = c.clone();
        for _ in 0..e[axis] {
            w.charge()?;
            coefficient *= value;
            w.coefficient(&coefficient)?;
        }
        let mut exponent = *e;
        exponent[axis] = 0;
        out.insert(exponent, coefficient, w)?;
    }
    Ok(out)
}
/// Prove that the complete closed shore of this ORIGINAL knot patch is one
/// exact world point. Arbitrary rational bases and weights are permitted.
/// Neither source regularity at the point nor SS-contact presence is asserted.
pub fn qualify_ss_pole_stratum(
    original: &QualifiedOriginalSsIdeal,
    support: usize,
    axis: usize,
    shore: usize,
    limits: ExactSsAlgebraLimits,
) -> Result<QualifiedSsPoleStratum> {
    validate_limits(limits)?;
    check(
        support < 2 && axis < 2 && shore < 2,
        "Invalid original SS pole shore",
    )?;
    let mut w = Work { used: 0, limits };
    let coordinate = 2 * support + axis;
    let value = Q::from_integer(shore.into());
    let mut origin = std::array::from_fn(|_| Q::zero());
    origin[coordinate] = value.clone();
    let h = &original.homogeneous[support];
    let weight = h[3].evaluate(&origin);
    check(
        weight > Q::zero(),
        "Original pole reference denominator is not positive",
    )?;
    let point: [Q; 3] = std::array::from_fn(|k| h[k].evaluate(&origin) / &weight);
    for k in 0..3 {
        w.coefficient(&point[k])?;
        let expected = h[3].scale_monomial([0; 4], &point[k], &mut w)?;
        let difference = h[k].subtract(&expected, &mut w)?;
        check(
            specialize(&difference, coordinate, &value, &mut w)?.is_zero(),
            "Original source shore is not one world point",
        )?;
    }
    Ok(QualifiedSsPoleStratum {
        original: original.clone(),
        support,
        axis,
        shore,
        point,
        limits,
        work: w.used,
    })
}
#[derive(Clone, Debug)]
pub struct QualifiedSsPoleSaturation {
    original: QualifiedSsDeflatedSystem,
    pole: QualifiedSsPoleStratum,
    equations: Vec<ExactSsPolynomial>,
    limits: ExactSsGroebnerLimits,
    work: usize,
    pairs: usize,
}
impl QualifiedSsPoleSaturation {
    pub fn original(&self) -> &QualifiedSsDeflatedSystem {
        &self.original
    }
    pub fn pole(&self) -> &QualifiedSsPoleStratum {
        &self.pole
    }
    pub fn equations(&self) -> &[ExactSsPolynomial] {
        &self.equations
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn proves_no_off_pole_roots(&self) -> bool {
        self.equations
            .iter()
            .any(|p| p.terms.len() == 1 && p.terms.contains_key(&[0; 4]))
    }
    pub fn reverify(&self) -> Result<()> {
        self.original.reverify()?;
        self.pole.reverify()?;
        let fresh = qualify_ss_pole_saturation(&self.original, &self.pole, self.limits)?;
        check(
            self.equations == fresh.equations
                && self.work == fresh.work
                && self.pairs == fresh.pairs,
            "Original pole localization or limiting SS ideal changed",
        )
    }
}
/// Eliminate z from {source-equivalent SS equations, 1-z*r*W1*W2}.
/// Exact positive original denominators make localization root-equivalent off
/// the proved pole shore. The reduced system describes the algebraic closure
/// of off-pole branches; real-root/physical presence still requires native
/// root admission. The original pole stratum remains a separate obligation.
pub fn qualify_ss_pole_saturation(
    original: &QualifiedSsDeflatedSystem,
    pole: &QualifiedSsPoleStratum,
    limits: ExactSsGroebnerLimits,
) -> Result<QualifiedSsPoleSaturation> {
    validate_limits(limits.algebra)?;
    let mut order = limits.variable_order;
    order.sort();
    check(
        order == [0, 1, 2, 3]
            && (1..=65536).contains(&limits.max_pairs)
            && (1..=256).contains(&limits.max_basis),
        "Invalid source pole elimination work",
    )?;
    let ideal = original.original().original();
    check(
        ideal.spans == pole.original.spans
            && ideal.source.first() == pole.original.source.first()
            && ideal.source.second() == pole.original.source.second()
            && ideal.homogeneous == pole.original.homogeneous,
        "Pole stratum belongs to different original SS carriers or spans",
    )?;
    let mut w = Work {
        used: 0,
        limits: limits.algebra,
    };
    let mut coordinate = ExactSsPolynomial::default();
    let mut e = [0; 4];
    e[2 * pole.support + pole.axis] = 1;
    coordinate.insert(e, Q::one(), &mut w)?;
    coordinate.insert([0; 4], -Q::from_integer(pole.shore.into()), &mut w)?;
    let multiplier = coordinate
        .product(&ideal.homogeneous[0][3], &mut w)?
        .product(&ideal.homogeneous[1][3], &mut w)?;
    let mut input: Vec<SparseSsPolynomial<5>> = original
        .equations()
        .iter()
        .map(|p| SparseSsPolynomial {
            terms: p
                .terms
                .iter()
                .map(|(e, c)| ([e[0], e[1], e[2], e[3], 0], c.clone()))
                .collect(),
        })
        .collect();
    let mut inverse = SparseSsPolynomial::<5>::default();
    inverse.insert([0; 5], Q::one(), &mut w)?;
    for (e, c) in multiplier.terms {
        inverse.insert([e[0], e[1], e[2], e[3], 1], -c, &mut w)?;
    }
    input.push(inverse);
    let variable_order = [
        4,
        limits.variable_order[0],
        limits.variable_order[1],
        limits.variable_order[2],
        limits.variable_order[3],
    ];
    let (basis, pairs) = groebner::basis_for(
        &input,
        variable_order,
        limits.max_pairs,
        limits.max_basis,
        &mut w,
    )?;
    let equations = basis
        .into_iter()
        .filter(|p| p.terms.keys().all(|e| e[4] == 0))
        .map(|p| ExactSsPolynomial {
            terms: p
                .terms
                .into_iter()
                .map(|(e, c)| ([e[0], e[1], e[2], e[3]], c))
                .collect(),
        })
        .collect();
    Ok(QualifiedSsPoleSaturation {
        original: original.clone(),
        pole: pole.clone(),
        equations,
        limits,
        work: w.used,
        pairs,
    })
}
