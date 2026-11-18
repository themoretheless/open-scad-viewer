use super::*;
pub(in crate::editing) fn accumulate(
    p: &mut Poly,
    key: (usize, usize),
    value: &Q,
    w: &mut Work,
) -> PResult<()> {
    let old = p.get(&key).cloned().unwrap_or_else(Q::zero);
    let q = w.op(&old, value, |a, b| a + b)?;
    if q.is_zero() {
        p.remove(&key);
    } else {
        p.insert(key, q);
    }
    Ok(())
}
pub(in crate::editing) fn multiply(a: &Poly, b: &Poly, w: &mut Work) -> PResult<Poly> {
    let mut out = Poly::new();
    for (&(i, j), a) in a {
        for (&(k, l), b) in b {
            let term = w.op(a, b, |a, b| a * b)?;
            accumulate(&mut out, (i + k, j + l), &term, w)?;
        }
    }
    Ok(out)
}
pub(in crate::editing) fn subtract(mut a: Poly, b: &Poly, w: &mut Work) -> PResult<Poly> {
    for (&key, value) in b {
        let value = w.op(&Q::zero(), value, |a, b| a - b)?;
        accumulate(&mut a, key, &value, w)?;
    }
    Ok(a)
}
pub(in crate::editing) fn derivative(p: &Poly, axis: Axis, width: &Q, w: &mut Work) -> PResult<Poly> {
    let mut out = Poly::new();
    for (&(i, j), value) in p {
        let n = match axis {
            Axis::U => i,
            Axis::V => j,
        };
        if n == 0 {
            continue;
        }
        let value = w.op(value, &Q::from_integer(n.into()), |a, b| a * b)?;
        let value = w.op(&value, width, |a, b| a / b)?;
        out.insert(
            match axis {
                Axis::U => (i - 1, j),
                Axis::V => (i, j - 1),
            },
            value,
        );
    }
    Ok(out)
}

/// Reduce a PROVEN constant vector field, including zero, before degree tests.
/// This is not general multivariate factor cancellation; the final stored-bit
/// proof still uses the complete original uncancelled derivative target.
pub(in crate::editing) fn constant_field(h: &[Poly], w: &mut Work) -> PResult<Option<Vec<Poly>>> {
    let (&key, d) = h[3].iter().next().ok_or(Refusal::InvalidOutput)?;
    let mut result = Vec::new();
    for n in &h[..3] {
        let first = n.get(&key).cloned().unwrap_or_else(Q::zero);
        let scale = w.op(&first, d, |a, b| a / b)?;
        for k in h[3].keys().chain(n.keys()) {
            let d = h[3].get(k).cloned().unwrap_or_else(Q::zero);
            let expected = w.op(&scale, &d, |a, b| a * b)?;
            if n.get(k).cloned().unwrap_or_else(Q::zero) != expected {
                return Ok(None);
            }
        }
        result.push(if scale.is_zero() {
            Poly::new()
        } else {
            BTreeMap::from([((0, 0), scale)])
        });
    }
    result.push(BTreeMap::from([((0, 0), Q::one())]));
    Ok(Some(result))
}
