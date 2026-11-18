use super::*;
pub(super) fn contains(a: Box3, b: Box3) -> bool {
    (0..3).all(|k| a[k][0] <= b[k][0] && b[k][1] <= a[k][1])
}
pub(super) fn disjoint(a: Box3, b: Box3) -> bool {
    (0..3).any(|k| a[k][1] < b[k][0] || b[k][1] < a[k][0])
}
/// Remove the closed proved box. Shared slab boundaries may be retained;
/// the unique owned root is strictly interior to its existence domain.
pub(super) fn subtract(mut a: Box3, b: Box3) -> Option<Vec<Box3>> {
    let x: Box3 = std::array::from_fn(|k| [a[k][0].max(b[k][0]), a[k][1].min(b[k][1])]);
    if x.iter().any(|d| d[0] >= d[1]) {
        return None;
    }
    let mut out = Vec::new();
    for k in 0..3 {
        if a[k][0] < x[k][0] {
            let mut c = a;
            c[k][1] = x[k][0];
            out.push(c);
        }
        if x[k][1] < a[k][1] {
            let mut c = a;
            c[k][0] = x[k][1];
            out.push(c);
        }
        a[k] = x[k];
    }
    Some(out)
}
pub(super) fn expanded(
    c: &Source<'_>,
    s: &Surface,
    a: Box3,
    window: Box3,
) -> Option<([usize; 3], Box3)> {
    let knots = [c.knots(), s.knots_u.as_slice(), s.knots_v.as_slice()];
    let p = [c.degree(), s.degree_u, s.degree_v];
    let n = [c.count(), s.control_points.len(), s.control_points[0].len()];
    let mut ids = [0; 3];
    let mut out = a;
    for k in 0..3 {
        let span = (p[k]..n[k]).find(|&i| {
            knots[k][i] < knots[k][i + 1] && knots[k][i] <= a[k][0] && a[k][1] <= knots[k][i + 1]
        })?;
        ids[k] = span;
        let width = a[k][1] - a[k][0];
        if !width.is_finite() || width <= 0. {
            return None;
        }
        out[k] = [
            (a[k][0] - width).max(knots[k][span]).max(window[k][0]),
            (a[k][1] + width).min(knots[k][span + 1]).min(window[k][1]),
        ];
    }
    Some((ids, out))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excision_slabs_cover_the_complement_with_disjoint_interiors() {
        let a = [[0., 1.]; 3];
        let b = [[0.25, 0.75], [0.125, 0.625], [0.375, 0.875]];
        let cells = subtract(a, b).unwrap();
        assert_eq!(cells.len(), 6);
        let volume = |a: Box3| a.iter().map(|d| d[1] - d[0]).product::<f64>();
        assert_eq!(
            cells.iter().copied().map(volume).sum::<f64>() + volume(b),
            1.
        );
        for (i, x) in cells.iter().enumerate() {
            assert!(contains(a, *x));
            assert!(subtract(*x, b).is_none());
            for y in &cells[i + 1..] {
                assert!((0..3).any(|k| x[k][1] <= y[k][0] || y[k][1] <= x[k][0]));
            }
        }
        for i in 0..=16 {
            for j in 0..=16 {
                for k in 0..=16 {
                    let p = [i as f64 / 16., j as f64 / 16., k as f64 / 16.];
                    let point = p.map(|x| [x, x]);
                    assert!(contains(b, point) || cells.iter().any(|a| contains(*a, point)));
                }
            }
        }
    }
}
