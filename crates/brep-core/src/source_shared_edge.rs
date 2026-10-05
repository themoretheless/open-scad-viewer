//! Exact canonical world ownership for two complete original source edges.
use crate::source_boundary_fragment::{Endpoint, Fragment};
use cad_predicates::BezierIdentity;
use nurbs_core::{curve::Curve, curve_surface_agreement, Error, Result};
#[derive(Clone)]
pub struct SharedEdge {
    world: Curve,
    uses: [Fragment; 2],
    reversed: [bool; 2],
}
impl SharedEdge {
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn uses(&self) -> &[Fragment; 2] {
        &self.uses
    }
    pub fn reversed(&self) -> [bool; 2] {
        self.reversed
    }
}
pub struct Report {
    pub edge: Option<SharedEdge>,
    pub work_used: u64,
    pub reason: &'static str,
}
/// `world_reversed` relates canonical world traversal to each forward UV curve.
/// No caller-supplied certificate, shared key or world proximity is authority.
pub fn qualify(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    max_work: u64,
) -> Result<Report> {
    if max_work == 0 || max_work > 100_000_000 {
        return Err(Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Choose bounded exact identity work",
        ));
    }
    world.validate()?;
    let mut out = Report {
        edge: None,
        work_used: 0,
        reason: "source-restriction-identity-unproven",
    };
    for edge in uses {
        let d = edge.curve().domain();
        let expected = if edge.reversed() { [d[1], d[0]] } else { d };
        if !(0..2).all(|i| matches!(&edge.endpoints()[i], Endpoint::Parameter(t) if t.to_bits() == expected[i].to_bits())) {
            return Ok(out);
        }
    }
    let effective = [
        uses[0].reversed() ^ world_reversed[0],
        uses[1].reversed() ^ world_reversed[1],
    ];
    if effective[0] == effective[1] {
        out.reason = "source-edge-uses-not-opposite";
        return Ok(out);
    }
    for (i, edge) in uses.iter().enumerate() {
        if out.work_used == max_work {
            out.reason = "source-edge-identity-work-limit";
            return Ok(out);
        }
        let Some(proof) = curve_surface_agreement::verify_exact(
            world,
            edge.curve(),
            edge.surface(),
            world_reversed[i],
            max_work - out.work_used,
        )?
        else {
            out.reason = "source-world-identity-layout-unproven";
            return Ok(out);
        };
        out.work_used += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            out.reason = "source-world-identity-unproven";
            return Ok(out);
        }
    }
    out.edge = Some(SharedEdge {
        world: world.clone(),
        uses: [uses[0].clone(), uses[1].clone()],
        reversed: effective,
    });
    out.reason = "source-shared-world-edge-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    #[test]
    fn different_surface_charts_share_only_exact_opposite_world_uses() {
        let surface = |z: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., z]],
                vec![vec![1., 0., 0.], vec![1., 1., z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let a = Fragment::new(
            &surface(0.),
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let b = Fragment::new(
            &surface(1.),
            &p,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let r = qualify(&c, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.edge.unwrap().reversed(), [false, true]);
        assert!(qualify(&c, [&a, &a], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let mut shifted = c.clone();
        shifted.control_points[0][2] = 1e-12;
        assert!(qualify(&shifted, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let partial = Fragment::new(
            a.surface(),
            &p,
            Endpoint::Parameter(0.2),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(qualify(&c, [&partial, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
}
