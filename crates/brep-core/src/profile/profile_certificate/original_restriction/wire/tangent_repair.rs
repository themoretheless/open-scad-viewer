//! Exact tangent-line intersections for editable rational quadratic spans.
//! Fixed original geometry and all weights remain unchanged.
use super::*;
fn xy(h: &[Q; 3]) -> [Q; 2] {
    [&h[0] / &h[2], &h[1] / &h[2]]
}
fn tangent(span: &Span, end: bool) -> [Q; 2] {
    let n = span.net.len();
    let (a, b) = if end {
        (&span.net[n - 2], &span.net[n - 1])
    } else {
        (&span.net[0], &span.net[1])
    };
    std::array::from_fn(|k| &b[k] * &a[2] - &a[k] * &b[2])
}
fn cross(a: &[Q; 2], b: &[Q; 2]) -> Q {
    &a[0] * &b[1] - &a[1] * &b[0]
}
pub(super) fn apply(
    sources: &[Restriction],
    editable: &[bool],
    pieces: &mut [Vec<Span>],
    contacts: &[LineContact],
) -> Result<()> {
    let cells = sources
        .iter()
        .enumerate()
        .flat_map(|(p, s)| (0..s.spans.len()).map(move |i| (p, i)))
        .collect::<Vec<_>>();
    let direction = |left: usize, right: usize| -> Result<[Q; 2]> {
        if let Some(direction) = contacts::direction(contacts, cells[left], cells[right], sources)?
        {
            return Ok(direction);
        }
        let (p, i) = cells[left];
        let (q, j) = cells[right];
        if !editable[q] {
            Ok(tangent(&sources[q].spans[j], false))
        } else {
            Ok(tangent(&sources[p].spans[i], true))
        }
    };
    for (k, &(p, i)) in cells.iter().enumerate() {
        if !editable[p] {
            continue;
        }
        let span = &mut pieces[p][i];
        if span.net.len() != 3 {
            return Err(refused(
                "Exact tangent repair requires quadratic editable spans",
            ));
        }
        let d0 = direction((k + cells.len() - 1) % cells.len(), k)?;
        let d1 = direction(k, (k + 1) % cells.len())?;
        let a = xy(&span.net[0]);
        let b = xy(&span.net[2]);
        let chord = [&b[0] - &a[0], &b[1] - &a[1]];
        let det = cross(&d0, &d1);
        if det.is_zero() {
            return Err(refused(
                "Editable tangent lines do not have a unique intersection",
            ));
        }
        let alpha = cross(&chord, &d1) / &det;
        let beta = cross(&d0, &chord) / &det;
        if alpha <= Q::zero() || beta <= Q::zero() {
            return Err(refused(
                "Editable tangent intersection reverses an endpoint direction",
            ));
        }
        for axis in 0..2 {
            span.net[1][axis] = (&a[axis] + &alpha * &d0[axis]) * &span.net[1][2];
        }
    }
    Ok(())
}
