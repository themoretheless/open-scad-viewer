//! Bounded reuse of immutable original chart polynomials, never proof receipts.
use super::super::exact_trace::Restriction;
use super::*;
pub(super) type Jets = [GraphTrace; 2];
const MAX_ENTRIES: usize = 16;
const MAX_COEFFICIENTS: usize = 65536;
struct Cell {
    range: [f64; 2],
    spans: [[usize; 2]; 2],
    jets: Jets,
    count: usize,
}
#[derive(Default)]
pub(super) struct Cache {
    curves: Option<[Curve; 2]>,
    cells: std::collections::VecDeque<Cell>,
    count: usize,
    reversed: [bool; 2],
}
fn curve_cost(c: &Curve) -> usize {
    c.control_points.iter().fold(
        c.knots
            .len()
            .saturating_add(c.weights.len())
            .saturating_add(2),
        |n, p| n.saturating_add(p.len()),
    )
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    let same = |a: &[f64], b: &[f64]| {
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
    };
    a.degree == b.degree
        && a.periodic == b.periodic
        && same(&a.knots, &b.knots)
        && same(&a.weights, &b.weights)
        && a.control_points.len() == b.control_points.len()
        && a.control_points
            .iter()
            .zip(&b.control_points)
            .all(|(a, b)| same(a, b))
}
impl Cache {
    #[cfg(test)]
    pub(super) fn get(
        &self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        w: &mut Work,
    ) -> Result<Option<Jets>> {
        self.get_oriented(p, range, spans, w, [false; 2])
    }
    pub(super) fn get_oriented(
        &self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        w: &mut Work,
        reversed: [bool; 2],
    ) -> Result<Option<Jets>> {
        let Some(curves) = &self.curves else {
            return Ok(None);
        };
        w.operations(curve_cost(p[0]).saturating_add(curve_cost(p[1])))?;
        if self.reversed != reversed
            || !same_curve(&curves[0], p[0])
            || !same_curve(&curves[1], p[1])
        {
            return Ok(None);
        }
        w.operations(8 * self.cells.len())?;
        let cell = self.cells.iter().rev().find(|c| {
            c.spans==spans && c.range[0]<=range[0]
            && range[0]<range[1] && range[1]<=c.range[1]
            // Periodically rebuild from exact authored coefficients so inherited
            // interval error cannot accumulate down arbitrarily deep trees.
            && range[1]-range[0]>=(c.range[1]-c.range[0])/16.
        });
        let Some(cell) = cell else {
            return Ok(None);
        };
        let map = Restriction::new(cell.range, range, w)?;
        let mut result = Vec::new();
        for side in &cell.jets {
            result.push(side.restrict(&map, w)?);
        }
        Ok(Some(result.try_into().ok().unwrap()))
    }
    #[cfg(test)]
    pub(super) fn store(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        jets: &Jets,
        w: &mut Work,
    ) -> Result<()> {
        self.store_oriented(p, range, spans, jets, w, [false; 2])
    }
    pub(super) fn store_oriented(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        jets: &Jets,
        w: &mut Work,
        reversed: [bool; 2],
    ) -> Result<()> {
        let count = jets
            .iter()
            .flat_map(|j| j.polynomials())
            .map(Vec::len)
            .sum::<usize>();
        let identity_cost = curve_cost(p[0]).saturating_add(curve_cost(p[1]));
        // Caching is optional. Never spend beyond the remaining caller ledger
        // merely to retain an optimization after successful fresh composition.
        let cost = count
            .saturating_add(2 * identity_cost)
            .saturating_add(MAX_ENTRIES);
        if count > MAX_COEFFICIENTS || cost > w.limit.saturating_sub(w.used) {
            return Ok(());
        }
        w.operations(cost)?;
        if self.reversed != reversed
            || self
                .curves
                .as_ref()
                .is_none_or(|old| !same_curve(&old[0], p[0]) || !same_curve(&old[1], p[1]))
        {
            self.cells.clear();
            self.count = 0;
            self.reversed = reversed;
            self.curves = Some([p[0].clone(), p[1].clone()]);
        }
        while self.cells.len() >= MAX_ENTRIES || self.count + count > MAX_COEFFICIENTS {
            self.count -= self.cells.pop_front().unwrap().count;
        }
        self.cells.push_back(Cell {
            range,
            spans,
            jets: jets.clone(),
            count,
        });
        self.count += count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_cells_bind_literal_curves_spans_clock_and_bounded_storage() {
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let polynomial = vec![I::point(0.), I::point(0.), I::point(1.)];
        let jets: Jets = std::array::from_fn(|_| GraphTrace {
            position: Some(std::array::from_fn(|_| polynomial.clone())),
            weight: polynomial.clone(),
            derivatives: std::array::from_fn(|_| vec![polynomial.clone(); 4]),
        });
        let mut w = Work {
            used: 0,
            limit: 100_000_000,
        };
        let mut cache = Cache::default();
        cache
            .store([&p, &p], [0., 1.], [[2, 2]; 2], &jets, &mut w)
            .unwrap();
        let child = cache
            .get([&p, &p], [0.25, 0.5], [[2, 2]; 2], &mut w)
            .unwrap()
            .unwrap();
        for (value, expected) in child[0].position.as_ref().unwrap()[0]
            .iter()
            .zip([0.0625, 0.125, 0.25])
        {
            assert!(value.lo <= expected && expected <= value.hi);
        }
        assert!(
            cache
                .get([&p, &p], [0., 1. / 1024.], [[2, 2]; 2], &mut w)
                .unwrap()
                .is_none()
        );
        assert!(
            cache
                .get([&p, &p], [0.25, 0.5], [[3, 2]; 2], &mut w)
                .unwrap()
                .is_none()
        );
        assert!(
            cache
                .get_oriented([&p, &p], [0.25, 0.5], [[2, 2]; 2], &mut w, [true, false])
                .unwrap()
                .is_none()
        );
        let mut changed = p.clone();
        changed.control_points[1][1] = 1f64.next_up();
        assert!(
            cache
                .get([&p, &changed], [0.25, 0.5], [[2, 2]; 2], &mut w)
                .unwrap()
                .is_none()
        );
        let mut none = Work { used: 0, limit: 0 };
        assert!(
            cache
                .get([&p, &p], [0.25, 0.5], [[2, 2]; 2], &mut none)
                .is_err()
        );
        assert_eq!(none.used, 0);
        for i in 0..20 {
            cache
                .store(
                    [&p, &p],
                    [i as f64 / 64., (i + 1) as f64 / 64.],
                    [[2, 2]; 2],
                    &jets,
                    &mut w,
                )
                .unwrap();
        }
        assert_eq!(cache.cells.len(), MAX_ENTRIES);
        assert!(cache.count <= MAX_COEFFICIENTS);
        assert!(
            cache
                .get([&p, &p], [0., 1. / 64.], [[2, 2]; 2], &mut w)
                .unwrap()
                .is_none()
        );
        cache
            .store([&p, &changed], [0., 1.], [[2, 2]; 2], &jets, &mut w)
            .unwrap();
        assert_eq!(cache.cells.len(), 1);
        assert!(
            cache
                .get([&p, &p], [0.25, 0.5], [[2, 2]; 2], &mut w)
                .unwrap()
                .is_none()
        );
    }
}
