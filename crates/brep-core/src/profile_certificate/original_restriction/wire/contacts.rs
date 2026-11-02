//! Explicit exact contact of an editable junction with an original line.
use super::*;
#[derive(Clone, Debug)]
pub struct LineContact {
    after_piece: usize,
    line: Restriction,
}
impl LineContact {
    pub fn new(after_piece: usize, line: Restriction) -> Self {
        Self { after_piece, line }
    }
    pub fn after_piece(&self) -> usize {
        self.after_piece
    }
    pub fn line(&self) -> &Restriction {
        &self.line
    }
}
fn frame(c: &LineContact) -> Result<([Q; 2], [Q; 2])> {
    c.line.reverify()?;
    if c.line.spans.len() != 1 || c.line.spans[0].net.len() != 2 {
        return Err(refused(
            "Exact junction contact needs an original straight segment",
        ));
    }
    let a = endpoint(&c.line.spans, false);
    let b = endpoint(&c.line.spans, true);
    let d = [&b[0] - &a[0], &b[1] - &a[1]];
    if d.iter().all(Zero::is_zero) {
        return Err(refused("Collapsed original junction contact line"));
    }
    Ok((a, d))
}
pub(super) fn apply(
    contacts: &[LineContact],
    editable: &[bool],
    pieces: &mut [Vec<Span>],
) -> Result<()> {
    if contacts.len() > 256 {
        return Err(refused("Exact line contact budget exhausted"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for contact in contacts {
        let p = contact.after_piece;
        if p >= pieces.len() || !seen.insert(p) {
            return Err(refused("Invalid or duplicated exact junction contact"));
        }
        let next = (p + 1) % pieces.len();
        if !editable[p] || !editable[next] {
            return Err(refused("Exact contact cannot move a fixed source piece"));
        }
        let (a, d) = frame(contact)?;
        let point = endpoint(&pieces[p], true);
        let numerator = (0..2).map(|k| (&point[k] - &a[k]) * &d[k]).sum::<Q>();
        let denominator = d.iter().map(|v| v * v).sum::<Q>();
        let t = numerator / denominator;
        if t <= Q::zero() || t >= Q::one() {
            return Err(refused(
                "Exact junction contact leaves original open line segment",
            ));
        }
        let point = std::array::from_fn(|k| &a[k] + &t * &d[k]);
        set_endpoint(&mut pieces[p], true, &point);
        set_endpoint(&mut pieces[next], false, &point);
    }
    Ok(())
}
pub(super) fn direction(
    contacts: &[LineContact],
    left: (usize, usize),
    right: (usize, usize),
    sources: &[Restriction],
) -> Result<Option<[Q; 2]>> {
    if right.0 != (left.0 + 1) % sources.len()
        || right.1 != 0
        || left.1 + 1 != sources[left.0].spans.len()
    {
        return Ok(None);
    }
    contacts
        .iter()
        .find(|c| c.after_piece == left.0)
        .map(frame)
        .transpose()
        .map(|f| f.map(|(_, d)| d))
}
