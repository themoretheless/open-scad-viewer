use super::*;
pub(super) fn assemble(
    outer: &closed::QualifiedClosedSolid,
    inner: &QualifiedPairedRootSolid,
) -> Result<MixedBrep> {
    let mut t = MixedBrep::from_closed_solid(outer)?.topology().clone();
    let mut b = inner.model().topology().clone();
    let (nv, ne, nl, nf, ns) = (
        t.vertices.len(),
        t.edges.len(),
        t.loops.len(),
        t.faces.len(),
        t.shells.len(),
    );
    for edge in &mut b.edges {
        edge.vertices = edge.vertices.map(|v| v + nv);
    }
    for wire in &mut b.loops {
        for c in &mut wire.coedges {
            c.edge += ne;
            if let MixedPcurve::Root(p) = &mut c.pcurve {
                p.world_edge += ne;
            }
        }
    }
    for f in &mut b.faces {
        f.outer += nl;
        for l in &mut f.holes {
            *l += nl;
        }
    }
    for s in &mut b.shells {
        for f in &mut s.faces {
            f.face += nf;
            f.reversed = !f.reversed;
        }
    }
    let cavity = b.bodies[0].outer_shell + ns;
    if !b.bodies[0].inner_shells.is_empty() {
        return Err(unproved("Paired cavity source has foreign nested shells"));
    }
    t.vertices.extend(b.vertices);
    t.edges.extend(b.edges);
    t.loops.extend(b.loops);
    t.faces.extend(b.faces);
    t.shells.extend(b.shells);
    t.bodies[0].inner_shells.push(cavity);
    t.tolerance_mm = t.tolerance_mm.max(b.tolerance_mm);
    MixedBrep::from_topology(t)
}
