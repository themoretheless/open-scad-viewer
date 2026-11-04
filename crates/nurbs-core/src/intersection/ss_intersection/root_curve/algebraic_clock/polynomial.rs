use super::*;
pub(super) struct Work {
    pub used: usize,
    limits: SsAlgebraicClockLimits,
}
impl Work {
    pub fn new(limits: SsAlgebraicClockLimits) -> Result<Self> {
        check(
            (1..=64).contains(&limits.max_degree)
                && (1..=1000000).contains(&limits.max_operations)
                && (64..=131072).contains(&limits.max_coefficient_bits),
            "Invalid exact clock work policy",
        )?;
        Ok(Self { used: 0, limits })
    }
    pub fn input(&mut self, value: Q) -> Result<Q> {
        let value = self.keep(value)?;
        let canonical = Q::new(value.numer().clone(), value.denom().clone());
        check(
            value.numer() == canonical.numer() && value.denom() == canonical.denom(),
            "Exact clock inputs require canonical rational coefficients and bounds",
        )?;
        Ok(value)
    }
    pub fn keep(&mut self, value: Q) -> Result<Q> {
        if self.used == self.limits.max_operations {
            return Err(crate::resource("Exact clock operation work exhausted"));
        }
        self.used += 1;
        check(
            value.denom().is_positive(),
            "Invalid exact clock rational denominator",
        )?;
        if value.numer().bits() > self.limits.max_coefficient_bits
            || value.denom().bits() > self.limits.max_coefficient_bits
        {
            return Err(crate::resource("Exact clock coefficient size exhausted"));
        }
        Ok(value)
    }
}
pub(super) fn trim(p: &mut Vec<Q>) {
    while p.last().is_some_and(|x| x.is_zero()) {
        p.pop();
    }
}
pub(super) fn evaluate(p: &[Q], x: &Q, w: &mut Work) -> Result<Q> {
    let mut value = Q::zero();
    for c in p.iter().rev() {
        let a = w.keep(value * x)?;
        value = w.keep(a + c)?;
    }
    Ok(value)
}
fn derivative(p: &[Q], w: &mut Work) -> Result<Vec<Q>> {
    p.iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| w.keep(c * Q::from_integer(i.into())))
        .collect()
}
fn remainder(a: &[Q], b: &[Q], w: &mut Work) -> Result<Vec<Q>> {
    check(!b.is_empty(), "Exact clock division by zero")?;
    let mut r = a.to_vec();
    while !r.is_empty() && r.len() >= b.len() {
        let offset = r.len() - b.len();
        let f = w.keep(r.last().unwrap() / b.last().unwrap())?;
        for (i, c) in b.iter().enumerate() {
            let t = w.keep(&f * c)?;
            r[offset + i] = w.keep(&r[offset + i] - t)?;
        }
        trim(&mut r);
    }
    Ok(r)
}
pub(super) fn sturm(p: &[Q], w: &mut Work) -> Result<Vec<Vec<Q>>> {
    let mut chain = vec![p.to_vec(), derivative(p, w)?];
    loop {
        let n = chain.len();
        let r = remainder(&chain[n - 2], &chain[n - 1], w)?;
        if r.is_empty() {
            break;
        }
        let negative = r
            .into_iter()
            .map(|c| w.keep(-c))
            .collect::<Result<Vec<_>>>()?;
        chain.push(negative);
        check(
            chain.len() <= p.len() + 1,
            "Exact clock Sturm chain degree did not decrease",
        )?;
    }
    Ok(chain)
}
fn variations(chain: &[Vec<Q>], x: &Q, w: &mut Work) -> Result<usize> {
    let mut last = 0;
    let mut changes = 0;
    for p in chain {
        let v = evaluate(p, x, w)?;
        let sign = if v.is_positive() {
            1
        } else if v.is_negative() {
            -1
        } else {
            0
        };
        if sign != 0 {
            if last != 0 && last != sign {
                changes += 1;
            }
            last = sign;
        }
    }
    Ok(changes)
}
pub(super) fn count(chain: &[Vec<Q>], d: &[Q; 2], w: &mut Work) -> Result<usize> {
    let a = variations(chain, &d[0], w)?;
    let b = variations(chain, &d[1], w)?;
    a.checked_sub(b)
        .ok_or_else(|| crate::input("Exact clock Sturm variations inconsistent"))
}
