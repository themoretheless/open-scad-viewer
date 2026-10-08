//! Transfer a fresh projected-region proof to an exact owned ruled neighbor.
use super::*;
use crate::source_ruled_projection_contact::exact_trace;
use nurbs_core::curve::Curve;
fn single_bezier(c: &Curve) -> bool {
    let n = c.control_points.len();
    !c.periodic
        && c.degree > 0
        && n == c.degree + 1
        && c.knots.len() == 2 * n
        && c.knots[..n].iter().all(|k| *k == c.knots[0])
        && c.knots[n..].iter().all(|k| *k == c.knots[n])
        && c.knots[0] < c.knots[n]
}
fn identical_trace(a: &Curve, b: &Curve) -> bool {
    if !single_bezier(a) || !single_bezier(b) || a.degree != b.degree {
        return false;
    }
    let n = a.control_points.len();
    (0..2).any(|reverse| {
        (0..n).all(|i| {
            let j = if reverse == 0 { i } else { n - 1 - i };
            a.control_points[i] == b.control_points[j] && a.weights[i] == b.weights[j]
        })
    })
}
pub struct TransitionRuledContact<'x> {
    owners: [&'x ShellFace; 2],
    rail: usize,
    trace: exact_trace::Certificate<'x>,
    edge: crate::source_shared_edge::SharedEdge,
}
impl TransitionRuledContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn rail(&self) -> usize {
        self.rail
    }
    pub fn edge(&self) -> &crate::source_shared_edge::SharedEdge {
        &self.edge
    }
    pub fn trace(&self) -> &Curve {
        self.trace.curve()
    }
}
pub struct TransitionRuledReport<'x> {
    pub certificate: Option<TransitionRuledContact<'x>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_transition_ruled_contact<'x>(
        &'x self,
        rail: usize,
        region: &OriginalProjectedRegion<'_, '_, '_, '_, '_>,
        max_controls: usize,
        max_work: u64,
        max_drivers: usize,
    ) -> Result<TransitionRuledReport<'x>> {
        if rail > 1 || max_controls > 1000000 || max_work > 100000000 || max_drivers > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound original ruled transition contact work",
            ));
        }
        let mut out = TransitionRuledReport {
            certificate: None,
            exact_work: 0,
            reason: "profile-transition-ruled-contact-unproven",
        };
        let boundary = self.assembly.patch.rails.face.boundary;
        if !std::ptr::eq(region.face().boundary(), boundary) || max_work == 0 || max_drivers == 0 {
            return Ok(out);
        }
        let Some(pair) = self
            .pairs
            .iter()
            .find(|p| p.kind == ShellPairKind::Rail(rail))
        else {
            return Ok(out);
        };
        let owners = pair.uses.map(|a| &self.faces[a.face]);
        let [ShellFaceKind::Transition, ShellFaceKind::Original(index)] = owners.map(|f| f.kind)
        else {
            return Ok(out);
        };
        let root = boundary.rooted[rail];
        let neighbor =
            root.restricted.proposal.body.geometry().shell().faces()[index][0].edges()[0].surface();
        let Some(trace) = exact_trace::certify(neighbor, [0, 1], 1 - root.axis, max_controls)?
        else {
            return Ok(out);
        };
        let natural = match (root.axis, rail) {
            (0, 0) => 0,
            (0, 1) => 2,
            (1, 0) => 3,
            _ => 1,
        };
        let original = region.original();
        let Some(curve) = original
            .boundary()
            .iter()
            .zip(original.boundary_indices())
            .find_map(|(c, &i)| (i == natural).then_some(c))
        else {
            return Ok(out);
        };
        if original.axes() != [0, 1] || !identical_trace(curve, trace.curve()) {
            return Ok(out);
        }
        // The whole wall projects onto this original Jordan boundary. The
        // source subregion (and the transition with the same planar image)
        // can meet it only on the selected natural rail. Fresh carrier proof
        // supplies its full 3D contact ownership, including both cut roots.
        let edge = root.qualify_incident_edge(max_work, max_drivers)?;
        out.exact_work = edge.work_used;
        let Some(edge) = edge.edge else {
            out.reason = edge.reason;
            return Ok(out);
        };
        out.certificate = Some(TransitionRuledContact {
            owners,
            rail,
            trace,
            edge,
        });
        out.reason = "profile-transition-ruled-contact-qualified";
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_boundary_trace_identity_preserves_domain_and_refuses_residuals() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 2.], vec![3., 4.]],
            weights: vec![1., 0.75, 1.25],
            periodic: false,
        };
        let mut b = a.clone();
        b.knots = vec![2., 2., 2., 5., 5., 5.];
        b.control_points.reverse();
        b.weights.reverse();
        assert!(identical_trace(&a, &b));
        b.control_points[1][0] = f64::from_bits(b.control_points[1][0].to_bits() + 1);
        assert!(!identical_trace(&a, &b));
        b = a.clone();
        b.weights[1] = f64::from_bits(b.weights[1].to_bits() + 1);
        assert!(!identical_trace(&a, &b));
        b = a.clone();
        b.periodic = true;
        assert!(!identical_trace(&a, &b));
    }
}
