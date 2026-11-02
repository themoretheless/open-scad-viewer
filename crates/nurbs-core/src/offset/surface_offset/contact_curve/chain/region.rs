//! Closed original-UV Jordan regions with literal world closure and direct lifts.
use super::super::lift;
use super::{Chain, build};
use crate::{
    Result, check,
    curve::Curve,
    surface_offset,
    trim_domain::{self, Classification, TrimDomain},
    trim_simplicity,
};
pub mod compound;
mod rails;
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_curves: usize,
    pub simplicity_pairs: usize,
    pub simplicity_cells: usize,
    /// Independent degree-refinement allowance for EACH supplied closing curve.
    pub closing_fit: lift::FitLimits,
    /// Independent final direct-lift allowance for EACH actual loop curve.
    pub final_lift: lift::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_curves: 256,
            simplicity_pairs: 100000,
            simplicity_cells: 100000,
            closing_fit: lift::FitLimits::default(),
            final_lift: lift::Limits {
                cells: 100000,
                spans: 128,
            },
        }
    }
}
pub struct Region {
    chain: Chain,
    side: usize,
    closure: Vec<Curve>,
    limits: Limits,
    uv: Vec<Curve>,
    world: Vec<Curve>,
    simple: trim_simplicity::Report,
    rail_fits: Vec<lift::FitReport>,
    lifts: Vec<lift::Report>,
}
impl Region {
    /// Consume the sealed outer region and add nested UV boundaries. Even/odd
    /// roles come from certified nesting, not from authored loop orientation.
    pub fn with_nested(
        self,
        loops: &[Vec<Curve>],
        limits: compound::Limits,
    ) -> Result<compound::Compound> {
        compound::build(self, loops, limits)
    }
    pub fn chain(&self) -> &Chain {
        &self.chain
    }
    pub fn source_side(&self) -> usize {
        self.side
    }
    pub fn uv_loop(&self) -> &[Curve] {
        &self.uv
    }
    pub fn world_loop(&self) -> &[Curve] {
        &self.world
    }
    pub fn simplicity(&self) -> &trim_simplicity::Report {
        &self.simple
    }
    pub fn rail_fitting(&self) -> &[lift::FitReport] {
        &self.rail_fits
    }
    pub fn lift_proofs(&self) -> &[lift::Report] {
        &self.lifts
    }
    pub fn classify(&self, rectangle: [[f64; 2]; 2], max_cells: usize) -> Result<Classification> {
        TrimDomain::new(&[self.uv.clone()], self.chain.tolerance_uv)?.classify(rectangle, max_cells)
    }
    pub fn reverify(&self) -> Result<()> {
        let r = with_closure(&self.chain, self.side, &self.closure, self.limits)?;
        check(
            r.uv == self.uv && r.world == self.world,
            "UV/world region differs from full original reconstruction",
        )
    }
}
fn reverse(mut c: Curve) -> Result<Curve> {
    let d = c.domain();
    let n = c.control_points.len();
    check(
        !c.periodic
            && n == c.degree + 1
            && c.knots[..n].iter().all(|t| *t == d[0])
            && c.knots[n..].iter().all(|t| *t == d[1]),
        "Literal reversal needs the qualified full Bezier piece",
    )?;
    c.control_points.reverse();
    c.weights.reverse();
    Ok(c)
}
fn traced(c: &Chain, side: usize) -> Result<(Vec<Curve>, Vec<Curve>)> {
    let mut uv = vec![];
    let mut world = vec![];
    for p in &c.pieces {
        let (u, w) = (p.curves[side].clone(), p.world.clone());
        if p.spec.reversed {
            uv.push(reverse(u)?);
            world.push(reverse(w)?);
        } else {
            uv.push(u);
            world.push(w);
        }
    }
    Ok((uv, world))
}
fn validate(l: Limits) -> Result<()> {
    check(
        (2..=256).contains(&l.max_curves)
            && (1..=100000).contains(&l.simplicity_pairs)
            && (1..=100000).contains(&l.simplicity_cells)
            && (1..=100000).contains(&l.final_lift.cells)
            && (1..=100000).contains(&l.final_lift.spans),
        "Choose bounded closed-region proof work",
    )?;
    lift::fitting::validate(l.closing_fit)
}
fn admit_uv<'a>(
    curves: impl IntoIterator<Item = &'a Curve>,
    source: &crate::surface::Surface,
) -> Result<()> {
    let natural = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    for c in curves {
        c.validate()?;
        check(
            c.control_points[0].len() == 2
                && c.weights.iter().all(|w| *w > 0.)
                && c.control_points
                    .iter()
                    .all(|p| (0..2).all(|k| p[k] >= natural[k][0] && p[k] <= natural[k][1])),
            "Whole UV loop control hull must stay on the original chart",
        )?;
    }
    Ok(())
}
/// Close a contact chain with caller-proposed original UV curves. All original
/// contact proofs are rebuilt. Exact UV closure, complete simplicity/inside-
/// chart gates, actual world closure and direct whole normal-offset lifts are
/// independent requirements. No endpoint welding or sampled polygon is proof.
///
/// This proves a Jordan UV region on ONE original chart with admitted world
/// lifts. Source/world embedding, holes/nesting, material sectors, complete
/// contact-root coverage, cap construction and solid/BRep sewing remain separate.
pub fn with_closure(
    chain: &Chain,
    side: usize,
    closure: &[Curve],
    limits: Limits,
) -> Result<Region> {
    validate(limits)?;
    check(
        side < 2 && closure.len() <= limits.max_curves,
        "Invalid region source or closure size",
    )?;
    let fresh = build(
        chain.sources.each_ref(),
        chain.distances,
        &chain.spans,
        chain.tolerance_uv,
        chain.tolerance_mm,
        chain.limits,
    )?;
    let (mut uv, mut world) = traced(&fresh, side)?;
    check(
        uv.len() <= limits.max_curves
            && closure.len() <= limits.max_curves - uv.len()
            && uv.len() + closure.len() >= 2,
        "Closed region exceeds the explicit curve allowance",
    )?;
    uv.extend_from_slice(closure);
    let source = &fresh.sources[side];
    admit_uv(&uv, source)?;
    check(
        trim_domain::exact_loop_joins(&uv)? == Some(true),
        "Closed region needs literal original-UV endpoint joins",
    )?;
    let simple = trim_simplicity::inspect(
        &uv,
        fresh.tolerance_uv,
        limits.simplicity_pairs,
        limits.simplicity_cells,
    )?;
    check(
        simple.proven_simple,
        "Complete original-UV Jordan simplicity remains unproved",
    )?;
    let rail_fits = rails::fit(&fresh, side, closure, &mut world, limits)?;
    rails::exact_world_joins(&world)?;
    let mut lifts = vec![];
    for (u, w) in uv.iter().zip(&world) {
        let proof = lift::certify(
            w,
            u,
            source,
            fresh.distances[side],
            fresh.tolerance_mm,
            limits.final_lift,
        )?;
        check(
            proof.lift_correspondence_proven,
            "Actual closed world/UV loop lift remains unproved",
        )?;
        lifts.push(proof);
    }
    Ok(Region {
        chain: fresh,
        side,
        closure: closure.to_vec(),
        limits,
        uv,
        world,
        simple,
        rail_fits,
        lifts,
    })
}
/// Propose three straight UV closing rails through one literal natural boundary
/// (0 v-low, 1 u-high, 2 v-high, 3 u-low). Degenerate rails are omitted exactly.
/// The chosen boundary is a region proposal, NOT automatic material selection.
pub fn to_boundary(chain: &Chain, side: usize, far_side: usize, limits: Limits) -> Result<Region> {
    validate(limits)?;
    check(
        side < 2 && far_side < 4,
        "Choose an original source and natural boundary",
    )?;
    let (trace, _) = traced(chain, side)?;
    let a = &trace[0].control_points[0];
    let b = trace.last().unwrap().control_points.last().unwrap();
    let start = [a[0], a[1]];
    let end = [b[0], b[1]];
    let source = &chain.sources[side];
    let fixed = if far_side == 0 || far_side == 2 { 1 } else { 0 };
    let natural = if fixed == 0 {
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ]
    } else {
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ]
    };
    let value = natural[usize::from(far_side == 1 || far_side == 2)];
    let mut a = start;
    let mut b = end;
    a[fixed] = value;
    b[fixed] = value;
    let vertices = [end, b, a, start];
    let mut closure = vec![];
    for p in vertices.windows(2) {
        if p[0] != p[1] {
            closure.push(Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: p.iter().map(|p| p.to_vec()).collect(),
                weights: vec![1.; 2],
                periodic: false,
            });
        }
    }
    with_closure(chain, side, &closure, limits)
}
