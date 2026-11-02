//! Outward restriction of a compiled original polynomial to a child clock.
use super::*;

pub(crate) struct Restriction {
    prefix: Option<[I; 2]>,
    suffix: Option<[I; 2]>,
}
impl Restriction {
    pub(crate) fn new(parent: [f64; 2], child: [f64; 2], w: &mut Work) -> Result<Self> {
        check(
            parent[0] <= child[0] && child[0] < child[1] && child[1] <= parent[1],
            "Compiled trace child must lie inside its original clock",
        )?;
        let [a, b] = parent
            .map(authored)
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .ok()
            .unwrap();
        let [lo, hi] = child
            .map(authored)
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .ok()
            .unwrap();
        let width = &b - &a;
        w.charge(&width)?;
        let interval = |t: Q, w: &mut Work| -> Result<[I; 2]> {
            w.charge(&t)?;
            let complement = Q::one() - &t;
            w.charge(&complement)?;
            Ok([enclosure(&complement)?, enclosure(&t)?])
        };
        let prefix = if child[1] == parent[1] {
            None
        } else {
            Some(interval((&hi - &a) / &width, w)?)
        };
        // Restrict the prefix [parent.lo, child.hi] from its left end. These
        // exact rational ratios must not be rounded to a binary64 clock.
        let suffix = if child[0] == parent[0] {
            None
        } else {
            Some(interval((&lo - &a) / (&hi - &a), w)?)
        };
        Ok(Self { prefix, suffix })
    }
    pub(crate) fn apply(&self, p: &[I], w: &mut Work) -> Result<Vec<I>> {
        check(!p.is_empty(), "Empty compiled trace polynomial")?;
        w.operations(p.len())?;
        let mut out = p.to_vec();
        if out.len() == 1 {
            return Ok(out);
        }
        if let Some(t) = self.prefix {
            out = split(&out, t, false, w)?;
        }
        if let Some(t) = self.suffix {
            out = split(&out, t, true, w)?;
        }
        Ok(out)
    }
}
fn split(p: &[I], t: [I; 2], right: bool, w: &mut Work) -> Result<Vec<I>> {
    w.operations(p.len())?;
    let mut row = p.to_vec();
    let mut result = vec![if right { *row.last().unwrap() } else { row[0] }];
    while row.len() > 1 {
        w.operations(5 * row.len())?;
        row = row
            .windows(2)
            .map(|v| v[0].mul(t[0])?.add(v[1].mul(t[1])?))
            .collect::<Result<Vec<_>>>()?;
        result.push(if right { *row.last().unwrap() } else { row[0] });
    }
    if right {
        result.reverse();
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonbinary_child_clock_encloses_exact_quadratic_coefficients() {
        let parent = [0.1, 0.9];
        let child = [0.2, 0.7];
        let mut w = Work {
            used: 0,
            limit: 10000,
        };
        let map = Restriction::new(parent, child, &mut w).unwrap();
        let p = [I::point(0.), I::point(0.), I::point(1.)];
        let result = map.apply(&p, &mut w).unwrap();
        let a = authored(parent[0]).unwrap();
        let width = authored(parent[1]).unwrap() - &a;
        let lo = (authored(child[0]).unwrap() - &a) / &width;
        let hi = (authored(child[1]).unwrap() - &a) / &width;
        for (bound, value) in result.iter().zip([&lo * &lo, &lo * &hi, &hi * &hi]) {
            assert!(authored(bound.lo).unwrap() <= value && value <= authored(bound.hi).unwrap());
        }
        assert_eq!(
            Restriction::new(parent, parent, &mut w)
                .unwrap()
                .apply(&p, &mut w)
                .unwrap(),
            p
        );
        let uncertain = [I::new(-0.1, 0.1).unwrap(), I::new(0.9, 1.1).unwrap()];
        let result = map.apply(&uncertain, &mut w).unwrap();
        assert!(result[0].lo < result[0].hi && result[1].lo < result[1].hi);
        assert!(Restriction::new(parent, [0., 1.], &mut w).is_err());
        let mut none = Work { used: 0, limit: 0 };
        assert!(Restriction::new(parent, child, &mut none).is_err());
        assert_eq!(none.used, 0);
        assert!(map.apply(&p, &mut none).is_err());
        assert_eq!(none.used, 0);
    }
}
