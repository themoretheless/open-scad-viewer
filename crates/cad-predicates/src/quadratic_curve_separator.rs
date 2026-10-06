//! Exact quadratic separator signs on original rational Bezier UV curves.
//! The proposed functional is cross(T-J,P-J)-beta*dot(T-J,P-J)^2.
use crate::{
    Algebra, AuthoredScalar, Expansion, InputError, LeafRef, PredicateContext, Reason, Sign,
    exact_inputs,
};
pub struct Decision {
    pub coefficients: Option<[Vec<Sign>; 2]>,
    pub reason: Option<Reason>,
    pub work_used: u64,
}
pub fn quadratic_curve_separator(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 3]]; 2],
    frame: [[LeafRef; 2]; 2],
    beta: LeafRef,
) -> Result<Decision, InputError> {
    if curves.iter().any(|c| !(2..=17).contains(&c.len())) {
        return Err(InputError::InvalidInput(
            "Separator requires Bezier degrees 1..16",
        ));
    }
    let mut refs = frame.iter().flatten().copied().collect::<Vec<_>>();
    refs.push(beta);
    for c in curves {
        refs.extend(c.iter().flatten().copied());
    }
    let mut values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<[Vec<Sign>; 2], Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let dx = x[2].sub(&x[0], ctx)?;
        let dy = x[3].sub(&x[1], ctx)?;
        if dx.sign() == Sign::Zero && dy.sign() == Sign::Zero {
            return Err(Reason::MissingProof);
        }
        let unit2 = unit.mul(unit, ctx)?;
        let mut at = 5;
        let mut out = Vec::new();
        for c in curves {
            let mut linear = Vec::new();
            let mut tangent = Vec::new();
            let mut weights = Vec::new();
            for _ in c {
                let w = &x[at + 2];
                if w.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                let px = x[at].sub(&x[0], ctx)?;
                let py = x[at + 1].sub(&x[1], ctx)?;
                linear.push(vec![
                    dx.mul(&py, ctx)?
                        .sub(&dy.mul(&px, ctx)?, ctx)?
                        .mul(w, ctx)?,
                ]);
                tangent.push(vec![
                    dx.mul(&px, ctx)?
                        .add(&dy.mul(&py, ctx)?, ctx)?
                        .mul(w, ctx)?,
                ]);
                weights.push(vec![unit.mul(w, ctx)?]);
                at += 3;
            }
            let l = crate::projected_surface_jacobian::mul(&linear, &weights, [None, None], ctx)?;
            let t = crate::projected_surface_jacobian::mul(&tangent, &tangent, [None, None], ctx)?;
            let mut signs = Vec::new();
            for (l, t) in l.iter().zip(t) {
                signs.push(
                    l[0].mul(&unit2, ctx)?
                        .sub(&t[0].mul(&x[4], ctx)?, ctx)?
                        .sign(),
                );
            }
            out.push(signs);
        }
        ctx.charge(0)?;
        Ok(out.try_into().unwrap())
    })();
    let (coefficients, reason) = match result {
        Ok(c) => (Some(c), None),
        Err(r) => (None, Some(r)),
    };
    Ok(Decision {
        coefficients,
        reason,
        work_used: ctx.work_used(),
    })
}
