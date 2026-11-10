//! profile certificate algorithm: pieces.
use super::*;
pub fn pieces(loops: &[Vec<Curve>], depth: u8) -> Result<Vec<Piece>> {
    if depth > MAX_DEPTH {
        return Err(refused(
            "Exact subdivision certificate exceeds its piece budget",
        ));
    }
    let source_nets = extraction::nets(loops)?;
    if source_nets.len() * (1usize << depth) > MAX_PIECES {
        return Err(refused(
            "Exact subdivision certificate exceeds its piece budget",
        ));
    }
    let mut out = Vec::with_capacity(source_nets.len() * (1usize << depth));
    for (wire, net) in source_nets {
        let mut nets = vec![net];
        for _ in 0..depth {
            nets = nets
                .into_iter()
                .flat_map(|n| {
                    let (a, b) = split(&n);
                    [a, b]
                })
                .collect();
        }
        out.extend(nets.into_iter().map(|n| Piece::from_net(wire, n)));
    }
    // These are exact rational equalities, including joins across source spans.
    for w in 0..loops.len() {
        let ring: Vec<_> = out.iter().filter(|p| p.wire == w).collect();
        for i in 0..ring.len() {
            if ring[i].points.last().unwrap() != &ring[(i + 1) % ring.len()].points[0] {
                return Err(refused("Certificate boundary is not exactly closed"));
            }
        }
    }
    Ok(out)
}
