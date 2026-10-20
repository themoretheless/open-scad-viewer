//! Source-owned natural support edges, exact world nets and neutral auxiliary wires.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_ratio, ratio};
use nurbs_core::surface::Surface;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicSupportWireRole {
    Authored { wire: usize },
    NaturalSupport,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalNaturalBoundaryCurve {
    pub degree: usize,
    pub knots: Vec<ExactRatio>,
    /// Original exact homogeneous world controls; never rounded to f64 poles.
    pub controls: Vec<[ExactRatio; 4]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicNaturalBoundary {
    pub fixed_axis: usize,
    pub end: usize,
    pub source: usize,
    pub wire: usize,
    pub reversed: bool,
    pub level: ExactRatio,
    pub native_domain: [ExactRatio; 2],
    pub world: OriginalNaturalBoundaryCurve,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportBoundaries {
    original: Arc<OwnedOriginalPeriodicWires>,
    augmented: Arc<OwnedOriginalPeriodicWires>,
    max_work: usize,
    roles: Vec<OriginalPeriodicSupportWireRole>,
    boundaries: Vec<OriginalPeriodicNaturalBoundary>,
    work: usize,
}
fn bump(work: &mut usize, max: usize) -> Result<()> {
    if *work >= max {
        return Err(refused(
            "Complete natural support source construction exhausted its allowance",
        ));
    }
    *work += 1;
    Ok(())
}
fn exact(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| refused("Natural boundary source has a nonfinite coefficient"))
}
/// Closed native-domain endpoint convention: right limit at the lower bound,
/// left limit at the upper bound. Repeated zero spans do not become 0/0.
fn basis(degree: usize, knots: &[Q], t: &Q, work: &mut usize, max: usize) -> Result<Vec<Q>> {
    if knots.len() < degree + 2 || knots.windows(2).any(|k| k[0] > k[1]) {
        return Err(refused("Invalid original natural boundary knot vector"));
    }
    let n = knots.len() - degree - 1;
    if degree >= n || t < &knots[degree] || t > &knots[n] || knots[degree] >= knots[n] {
        return Err(refused(
            "Natural boundary query is outside the original active knot domain",
        ));
    }
    let span = if t == &knots[n] {
        (degree..n)
            .rev()
            .find(|&i| knots[i] < *t && *t <= knots[i + 1])
    } else {
        (degree..n).find(|&i| knots[i] <= *t && *t < knots[i + 1])
    }
    .ok_or_else(|| refused("Natural boundary has no nonempty original endpoint span"))?;
    let mut values = vec![Q::zero(); knots.len() - 1];
    values[span] = Q::one();
    for order in 1..=degree {
        let mut next = Vec::new();
        for i in 0..values.len() - 1 {
            bump(work, max)?;
            let mut v = Q::zero();
            let a = &knots[i + order] - &knots[i];
            if a > Q::zero() {
                v += (t - &knots[i]) / a * &values[i];
            }
            let b = &knots[i + order + 1] - &knots[i + 1];
            if b > Q::zero() {
                v += (&knots[i + order + 1] - t) / b * &values[i + 1];
            }
            next.push(v);
        }
        values = next;
    }
    if values.len() != n
        || values.iter().any(|v| v < &Q::zero())
        || values.iter().cloned().sum::<Q>() != Q::one()
    {
        return Err(refused(
            "Exact natural boundary basis does not partition the original source",
        ));
    }
    Ok(values)
}
fn world_curve(
    s: &Surface,
    axis: usize,
    end: usize,
    work: &mut usize,
    max: usize,
) -> Result<OriginalNaturalBoundaryCurve> {
    let degree = [s.degree_u, s.degree_v];
    let controls = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    let fixed = knots[axis]
        .iter()
        .copied()
        .map(exact)
        .collect::<Result<Vec<_>>>()?;
    let level = fixed[if end == 0 {
        degree[axis]
    } else {
        controls[axis]
    }]
    .clone();
    let values = basis(degree[axis], &fixed, &level, work, max)?;
    let varying = 1 - axis;
    let mut net = Vec::new();
    for j in 0..controls[varying] {
        let mut h = std::array::from_fn::<_, 4, _>(|_| Q::zero());
        for (i, b) in values.iter().enumerate() {
            bump(work, max)?;
            let (u, v) = if axis == 0 { (i, j) } else { (j, i) };
            let w = exact(s.weights[u][v])?;
            let bw = b * w;
            for c in 0..3 {
                h[c] += &bw * exact(s.control_points[u][v][c])?;
            }
            h[3] += bw;
        }
        if h[3] <= Q::zero() {
            return Err(refused(
                "Natural boundary world control has nonpositive exact weight",
            ));
        }
        net.push(h.map(|q| ratio(&q)));
    }
    Ok(OriginalNaturalBoundaryCurve {
        degree: degree[varying],
        knots: knots[varying]
            .iter()
            .copied()
            .map(exact)
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(ratio)
            .collect(),
        controls: net,
    })
}
fn scalar_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
}
fn row_bits(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| scalar_bits(a, b))
}
fn curves_bits(a: &[Curve], b: &[Curve]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.degree == b.degree
                && a.periodic == b.periodic
                && scalar_bits(&a.knots, &b.knots)
                && scalar_bits(&a.weights, &b.weights)
                && row_bits(&a.control_points, &b.control_points)
        })
}
fn surface_bits(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u
        && a.periodic_v == b.periodic_v
        && scalar_bits(&a.knots_u, &b.knots_u)
        && scalar_bits(&a.knots_v, &b.knots_v)
        && row_bits(&a.weights, &b.weights)
        && a.control_points.len() == b.control_points.len()
        && a.control_points
            .iter()
            .zip(&b.control_points)
            .all(|(a, b)| row_bits(a, b))
}
struct Plan {
    sources: Vec<Curve>,
    uses: Vec<Vec<OriginalPeriodicSourceUse>>,
    roles: Vec<OriginalPeriodicSupportWireRole>,
    boundaries: Vec<OriginalPeriodicNaturalBoundary>,
    work: usize,
}
fn plan(original: &Arc<OwnedOriginalPeriodicWires>, max: usize) -> Result<Plan> {
    if !(1..=1000000).contains(&max) {
        return Err(refused("Invalid complete natural boundary work allowance"));
    }
    super::periodic_replay::verify_arc(original, |x| x.reverify_source())?;
    let supporting = original.surface();
    let cover = supporting.pieces();
    let s = supporting.surface();
    if s.control_points.iter().flatten().any(|p| p.len() != 3) {
        return Err(refused(
            "Natural support world curves need an actual three-dimensional source surface",
        ));
    }
    let chart = cover.chart();
    let groups: Vec<Vec<(usize, usize, bool)>> = match cover.wrapped() {
        [false, false] => vec![vec![
            (1, 0, false),
            (0, 1, false),
            (1, 1, true),
            (0, 0, true),
        ]],
        [true, false] => vec![vec![(1, 0, false)], vec![(1, 1, true)]],
        [false, true] => vec![vec![(0, 0, true)], vec![(0, 1, false)]],
        [true, true] => vec![],
    };
    let mut p = Plan {
        sources: cover.sources().to_vec(),
        uses: original.source_uses().to_vec(),
        roles: Vec::new(),
        boundaries: Vec::new(),
        work: 0,
    };
    for wire in 0..p.uses.len() {
        bump(&mut p.work, max)?;
        p.roles
            .push(OriginalPeriodicSupportWireRole::Authored { wire });
    }
    for group in groups {
        let wire = p.uses.len();
        let mut uses = Vec::new();
        for (axis, end, reversed) in group {
            bump(&mut p.work, max)?;
            let varying = 1 - axis;
            let domain = chart[varying];
            let mut a = [0.; 2];
            a[axis] = chart[axis][end];
            a[varying] = domain[0];
            let mut b = a;
            b[varying] = domain[1];
            let curve = Curve {
                degree: 1,
                knots: vec![domain[0], domain[0], domain[1], domain[1]],
                control_points: vec![a.to_vec(), b.to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            };
            curve.validate()?;
            let source = p.sources.len();
            p.sources.push(curve);
            uses.push(OriginalPeriodicSourceUse { source, reversed });
            p.boundaries.push(OriginalPeriodicNaturalBoundary {
                fixed_axis: axis,
                end,
                source,
                wire,
                reversed,
                level: ratio(&exact(chart[axis][end])?),
                native_domain: [ratio(&exact(domain[0])?), ratio(&exact(domain[1])?)],
                world: world_curve(s, axis, end, &mut p.work, max)?,
            });
        }
        p.uses.push(uses);
        p.roles
            .push(OriginalPeriodicSupportWireRole::NaturalSupport);
    }
    if p.sources.len() > 32 || p.uses.len() > 32 {
        return Err(refused(
            "Authored plus natural support sources exceed the complete periodic census allowance",
        ));
    }
    Ok(p)
}
fn derive(
    o: &OwnedPeriodicSupportBoundaries,
) -> Result<(
    Vec<OriginalPeriodicSupportWireRole>,
    Vec<OriginalPeriodicNaturalBoundary>,
    usize,
)> {
    let p = plan(&o.original, o.max_work)?;
    super::periodic_replay::verify_arc(&o.augmented, |x| x.reverify_source())?;
    if !surface_bits(
        o.original.surface().surface(),
        o.augmented.surface().surface(),
    ) || o.original.surface().pieces().wrapped() != o.augmented.surface().pieces().wrapped()
        || o.augmented.source_uses() != p.uses
        || !curves_bits(&p.sources, o.augmented.surface().pieces().sources())
    {
        return Err(refused(
            "Natural support augmentation changed original surface bits, source curves or wire roles",
        ));
    }
    Ok((p.roles, p.boundaries, p.work))
}
impl OwnedPeriodicSupportBoundaries {
    pub fn original(&self) -> &Arc<OwnedOriginalPeriodicWires> {
        &self.original
    }
    pub fn augmented(&self) -> &Arc<OwnedOriginalPeriodicWires> {
        &self.augmented
    }
    pub fn roles(&self) -> &[OriginalPeriodicSupportWireRole] {
        &self.roles
    }
    pub fn boundaries(&self) -> &[OriginalPeriodicNaturalBoundary] {
        &self.boundaries
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (r, b, w) = derive(self)?;
        if r != self.roles || b != self.boundaries || w != self.work {
            return Err(refused(
                "Natural support source roles, original world nets or work changed",
            ));
        }
        Ok(())
    }
    /// Auxiliary support wires carry ZERO authored-material weight. The caller
    /// retains its original fill rule and declared base value unchanged.
    pub fn lift_operand(
        &self,
        operand: &OriginalPeriodicOperand,
    ) -> Result<OriginalPeriodicOperand> {
        self.reverify_source()?;
        if operand.wire_weights.len() != self.original.wires().len() {
            return Err(refused(
                "Material operand does not cover the original authored wire columns",
            ));
        }
        let mut out = operand.clone();
        out.wire_weights = self
            .roles
            .iter()
            .map(|r| match *r {
                OriginalPeriodicSupportWireRole::Authored { wire } => operand.wire_weights[wire],
                OriginalPeriodicSupportWireRole::NaturalSupport => 0,
            })
            .collect();
        Ok(out)
    }
    /// Evaluate the original surface at exact rational UV coordinates. Wrapped
    /// coordinates are reduced by exact whole periods; unwrapped coordinates
    /// must remain in the closed original active knot domain. This preserves
    /// the represented author function, without constructing rounded poles.
    /// A point query alone does not qualify a whole edge or publish a face.
    pub fn world_uv_point(&self, uv: &[ExactRatio; 2]) -> Result<[ExactRatio; 3]> {
        self.reverify_source()?;
        let source = self.original.surface();
        let s = source.surface();
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v]
            .map(|k| k.iter().copied().map(exact).collect::<Result<Vec<_>>>())
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let wrapped = source.pieces().wrapped();
        let mut work = 0;
        let mut values = Vec::new();
        for axis in 0..2 {
            let mut t = decode_ratio(&uv[axis])?;
            let lo = &knots[axis][degrees[axis]];
            let hi = &knots[axis][counts[axis]];
            if wrapped[axis] && (t < *lo || t > *hi) {
                let period = hi - lo;
                let turns = ((&t - lo) / &period).floor();
                t -= turns * period;
            }
            values.push(basis(
                degrees[axis],
                &knots[axis],
                &t,
                &mut work,
                self.max_work,
            )?);
        }
        let mut h = std::array::from_fn::<_, 4, _>(|_| Q::zero());
        for (u, a) in values[0].iter().enumerate() {
            for (v, b) in values[1].iter().enumerate() {
                bump(&mut work, self.max_work)?;
                let w = a * b * exact(s.weights[u][v])?;
                for axis in 0..3 {
                    h[axis] += &w * exact(s.control_points[u][v][axis])?;
                }
                h[3] += w;
            }
        }
        if h[3] <= Q::zero() {
            return Err(refused(
                "Original surface query denominator is not positive",
            ));
        }
        Ok(std::array::from_fn(|axis| ratio(&(&h[axis] / &h[3]))))
    }
    /// Exact world point in the ORIGINAL varying-axis clock, even for nonclamped
    /// fixed-axis boundaries whose world poles cannot be represented in f64.
    pub fn world_point(&self, boundary: usize, parameter: &ExactRatio) -> Result<[ExactRatio; 3]> {
        self.reverify_source()?;
        let b = self
            .boundaries
            .get(boundary)
            .ok_or_else(|| refused("Unknown actual natural boundary source"))?;
        let knots = b
            .world
            .knots
            .iter()
            .map(decode_ratio)
            .collect::<Result<Vec<_>>>()?;
        let mut work = 0;
        let values = basis(
            b.world.degree,
            &knots,
            &decode_ratio(parameter)?,
            &mut work,
            self.max_work,
        )?;
        let mut h = std::array::from_fn::<_, 4, _>(|_| Q::zero());
        for (w, c) in values.iter().zip(&b.world.controls) {
            bump(&mut work, self.max_work)?;
            for axis in 0..4 {
                h[axis] += w * decode_ratio(&c[axis])?;
            }
        }
        if h[3] <= Q::zero() {
            return Err(refused(
                "Original natural boundary denominator is not positive",
            ));
        }
        Ok(std::array::from_fn(|axis| ratio(&(&h[axis] / &h[3]))))
    }
}
/// Add every natural unwrapped-domain edge to the ORIGINAL source census.
/// UV lines use exact native axis clocks; world nets use exact homogeneous
/// source basis values. This is geometry/role ownership, not half-disc sectors,
/// manifold face publication or new supporting-surface embedding authority.
pub fn own_periodic_support_boundaries(
    original: Arc<OwnedOriginalPeriodicWires>,
    width: f64,
    limits: ParameterOrderLimits,
    max_levels: usize,
    max_work: usize,
) -> Result<OwnedPeriodicSupportBoundaries> {
    let _replay = super::periodic_replay::scope();
    if !width.is_finite()
        || !(1e-18..=1e-3).contains(&width)
        || limits.max_refinements > 512
        || !(1..=128).contains(&limits.max_resultant_size)
        || !(1..=4096).contains(&max_levels)
    {
        return Err(refused(
            "Invalid natural support source construction settings",
        ));
    }
    let p = plan(&original, max_work)?;
    let augmented = if p.boundaries.is_empty() {
        original.clone()
    } else {
        let source = original.surface();
        let cover = Arc::new(own_original_periodic_pieces(
            &p.sources,
            source.pieces().chart(),
            source.pieces().wrapped(),
            width,
            limits,
            max_levels,
        )?);
        let support = Arc::new(bind_original_periodic_pieces_to_surface(
            cover,
            source.surface(),
            limits,
        )?);
        Arc::new(own_original_periodic_wires(support, &p.uses)?)
    };
    let o = OwnedPeriodicSupportBoundaries {
        original,
        augmented,
        max_work,
        roles: p.roles,
        boundaries: p.boundaries,
        work: p.work,
    };
    o.reverify_source()?;
    Ok(o)
}
#[cfg(test)]
pub(in crate::profile_certificate::span_intersections::translated) mod tests {
    use super::*;
    fn q(n: i64) -> ExactRatio {
        [n.to_string(), "1".into()]
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn original(
        surface: &Surface,
        wrapped: [bool; 2],
        curves: &[Curve],
        uses: &[Vec<OriginalPeriodicSourceUse>],
    ) -> Arc<OwnedOriginalPeriodicWires> {
        let chart = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let cover = Arc::new(
            own_original_periodic_pieces(
                curves,
                chart,
                wrapped,
                1e-12,
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        );
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                cover,
                surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        Arc::new(own_original_periodic_wires(support, uses).unwrap())
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn graph(
        wires: Arc<OwnedOriginalPeriodicWires>,
    ) -> Arc<OwnedPeriodicWireJumps> {
        graph_with_pair_limits(wires, OriginalAtlasPairLimits::default())
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn graph_with_pair_limits(
        wires: Arc<OwnedOriginalPeriodicWires>,
        pair_limits: OriginalAtlasPairLimits,
    ) -> Arc<OwnedPeriodicWireJumps> {
        let _replay = super::super::periodic_replay::scope();
        let pieces = wires.surface().pieces().clone();
        let n = pieces.pieces().len();
        let count = n * (n + 1) / 2;
        let copies = 3usize.pow(pieces.wrapped().iter().filter(|w| **w).count() as u32);
        let embedding = Arc::new(
            own_original_periodic_embedding(
                wires,
                4096,
                OriginalInjectiveAtlasLimits::default(),
                pair_limits,
            )
            .unwrap(),
        );
        let base = Arc::new(
            own_periodic_arrangement_cuts(
                pieces,
                1e-12,
                ParameterOrderLimits::default(),
                count.max(1),
            )
            .unwrap(),
        );
        let seams = Arc::new(
            own_periodic_seam_contacts(
                embedding,
                1e-12,
                ParameterOrderLimits::default(),
                (count * (copies - 1)).max(1),
            )
            .unwrap(),
        );
        let combined = Arc::new(
            own_periodic_quotient_arrangement_cuts(
                base,
                seams,
                ParameterOrderLimits::default(),
                (count * copies).max(1),
            )
            .unwrap(),
        );
        let aligned = Arc::new(
            own_periodic_aligned_intervals(combined, ParameterOrderLimits::default(), 100000)
                .unwrap(),
        );
        let graph = Arc::new(
            own_periodic_quotient_graph(aligned, ParameterOrderLimits::default()).unwrap(),
        );
        let charts = Arc::new(own_periodic_vertex_charts(graph, 192, 100000).unwrap());
        Arc::new(own_periodic_wire_jumps(charts, 100000).unwrap())
    }
    #[test]
    fn untrimmed_support_keeps_its_base_fill_without_authored_wires_or_fake_torus_edges() {
        let plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let input = original(&plane, [false, false], &[], &[]);
        let support = own_periodic_support_boundaries(
            input,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        assert!(support.original().wires().is_empty());
        assert_eq!(support.boundaries().len(), 4);
        assert_eq!(
            support.roles(),
            &[OriginalPeriodicSupportWireRole::NaturalSupport]
        );
        let operand = OriginalPeriodicOperand {
            wire_weights: vec![],
            base_winding: "1".into(),
            fill: OriginalPeriodicFillRule::NonZero,
        };
        let lifted = support.lift_operand(&operand).unwrap();
        assert_eq!(lifted.wire_weights, [0]);
        assert_eq!(lifted.base_winding, "1");
        let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
        let input = original(&surface, [true, true], &[], &[]);
        let support = own_periodic_support_boundaries(
            input.clone(),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            1,
        )
        .unwrap();
        assert!(support.boundaries().is_empty());
        assert!(support.roles().is_empty());
        assert_eq!(support.work(), 0);
        assert!(Arc::ptr_eq(support.augmented(), &input));
        assert!(
            support
                .lift_operand(&operand)
                .unwrap()
                .wire_weights
                .is_empty()
        );
        support.reverify_source().unwrap();
    }
    #[test]
    fn augmented_natural_sources_reach_actual_embedding_complete_arrangement_and_unchanged_authored_fill()
     {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 1., 2., 3., 4., 5.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: (0..3)
                .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = Curve::from_polyline(vec![
            vec![2.25, 11.],
            vec![2.75, 11.],
            vec![2.75, 13.],
            vec![2.25, 13.],
            vec![2.25, 11.],
        ])
        .unwrap();
        let input = original(
            &surface,
            [false, false],
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let support = own_periodic_support_boundaries(
            input,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        let jumps = graph(support.augmented().clone());
        assert_eq!(jumps.owner().owner().edges().len(), 8);
        assert_eq!(jumps.wire_fluxes().len(), 2);
        let a = [["17".into(), "8".into()], ["41".into(), "4".into()]];
        let b = [["5".into(), "2".into()], ["12".into(), "1".into()]];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap(),
        );
        let operand = support
            .lift_operand(&OriginalPeriodicOperand {
                wire_weights: vec![1],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            })
            .unwrap();
        assert_eq!(operand.wire_weights, [1, 0]);
        let rule = Arc::new(
            own_periodic_material_rule(base, &[operand], &[OriginalPeriodicBooleanOp::Operand(0)])
                .unwrap(),
        );
        let query = Arc::new(
            own_periodic_relative_winding(jumps, &[a, b], ParameterOrderLimits::default(), 1000)
                .unwrap(),
        );
        assert_eq!(query.deltas(), &["1", "0"]);
        let sample = own_periodic_material_sample(rule, query).unwrap();
        assert!(sample.selected());
        assert_eq!(sample.windings(), &["1"]);
        sample.reverify_source().unwrap();
    }
    #[test]
    fn rational_uv_world_query_preserves_nonbinary_values_and_native_endpoint_conventions() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 1., 2., 3., 4., 5.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: (0..3)
                .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let o = own_periodic_support_boundaries(
            original(&surface, [false, false], &[], &[]),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        let third = ["1".into(), "3".into()];
        // At u=7/3 the unweighted basis is (4,13,1)/18 and the
        // homogeneous weighted x numerator/denominator is 28/31.
        assert_eq!(
            o.world_uv_point(&[["7".into(), "3".into()], ["34".into(), "3".into()]])
                .unwrap(),
            [["28".into(), "31".into()], third, q(0)]
        );
        for b in o.boundaries() {
            let varying = 1 - b.fixed_axis;
            let mut uv = [q(0), q(0)];
            uv[b.fixed_axis] = b.level.clone();
            uv[varying] = if varying == 0 {
                ["5".into(), "2".into()]
            } else {
                q(12)
            };
            let i = o.boundaries().iter().position(|v| v == b).unwrap();
            assert_eq!(
                o.world_uv_point(&uv).unwrap(),
                o.world_point(i, &uv[varying]).unwrap()
            );
        }
        assert!(o.world_uv_point(&[q(1), q(12)]).is_err());
        assert!(o.world_uv_point(&[q(2), ["1".into(), "0".into()]]).is_err());
    }
    #[test]
    fn rational_uv_world_query_uses_proved_seams_for_positive_and_negative_cover_tiles() {
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let s = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let o = own_periodic_support_boundaries(
            original(&s, [true, false], &[], &[]),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        let z = ["1".into(), "3".into()];
        for numerator in [1, 8001, -7999] {
            assert_eq!(
                o.world_uv_point(&[[numerator.to_string(), "2".into()], z.clone()])
                    .unwrap(),
                [["1".into(), "2".into()], q(0), z.clone()]
            );
        }
        assert_eq!(
            o.world_uv_point(&[q(0), z.clone()]).unwrap(),
            o.world_uv_point(&[q(4), z]).unwrap()
        );
        assert!(o.world_uv_point(&[q(0), q(2)]).is_err());
    }
    #[test]
    fn nonclamped_fixed_boundary_retains_exact_world_weights_and_four_native_source_sides() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 1., 2., 3., 4., 5.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: (0..3)
                .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = Curve::from_polyline(vec![
            vec![2.25, 11.],
            vec![2.75, 11.],
            vec![2.75, 13.],
            vec![2.25, 13.],
            vec![2.25, 11.],
        ])
        .unwrap();
        let original = original(
            &surface,
            [false, false],
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        assert!(
            own_periodic_support_boundaries(
                original.clone(),
                1e-12,
                ParameterOrderLimits::default(),
                64,
                1
            )
            .is_err()
        );
        let o = own_periodic_support_boundaries(
            original,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.boundaries().len(), 4);
        assert_eq!(o.augmented().wires().len(), 2);
        let index = o
            .boundaries()
            .iter()
            .position(|b| b.fixed_axis == 0 && b.end == 0)
            .unwrap();
        let b = &o.boundaries()[index];
        assert_eq!(b.native_domain, [q(10), q(14)]);
        assert_eq!(b.world.knots, vec![q(10), q(10), q(14), q(14)]);
        assert_eq!(
            b.world.controls[0],
            [q(1), q(0), q(0), ["3".into(), "2".into()]]
        );
        assert_eq!(
            o.world_point(index, &q(12)).unwrap(),
            [["2".into(), "3".into()], ["1".into(), "2".into()], q(0)]
        );
        assert!(o.world_point(index, &q(15)).is_err());
        let p = OriginalPeriodicOperand {
            wire_weights: vec![7],
            base_winding: "-2".into(),
            fill: OriginalPeriodicFillRule::EvenOdd,
        };
        let lifted = o.lift_operand(&p).unwrap();
        assert_eq!(lifted.wire_weights, [7, 0]);
        assert_eq!(lifted.base_winding, "-2");
        assert_eq!(lifted.fill, p.fill);
        let mut bad = o.clone();
        bad.boundaries[0].world.controls[0][0] = q(99);
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.roles[0] = OriginalPeriodicSupportWireRole::NaturalSupport;
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.work += 1;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn periodic_tube_adds_both_natural_rims_with_zero_material_weights_and_keeps_signed_source_bits()
     {
        let mut curve = Curve::from_polyline(vec![vec![0., 0.5], vec![4., 0.5]]).unwrap();
        curve.control_points[0][0] = -0.;
        let jumps = super::super::periodic_wire_jumps::tests::own(
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let input = jumps
            .owner()
            .owner()
            .seam_contacts()
            .embedding()
            .wires()
            .clone();
        let o = own_periodic_support_boundaries(
            input,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        assert_eq!(o.boundaries().len(), 2);
        assert_eq!(o.augmented().wires().len(), 3);
        assert_eq!(
            o.augmented().surface().pieces().sources()[0].control_points[0][0].to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(o.augmented().wires()[1].homology, [Some("1".into()), None]);
        assert_eq!(o.augmented().wires()[2].homology, [Some("-1".into()), None]);
        for (i, b) in o.boundaries().iter().enumerate() {
            assert_eq!(b.fixed_axis, 1);
            assert_eq!(
                o.world_point(i, &q(2)).unwrap(),
                [q(1), q(1), q(b.end as i64)]
            );
        }
        assert_eq!(
            o.lift_operand(&OriginalPeriodicOperand {
                wire_weights: vec![1],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero
            })
            .unwrap()
            .wire_weights,
            [1, 0, 0]
        );
    }
    #[test]
    fn closed_torus_has_no_fake_natural_edges_and_repeated_endpoint_basis_uses_the_original_left_limit()
     {
        let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
        let curve = Curve::from_polyline(vec![vec![0., 2.], vec![1., 2.]]).unwrap();
        let input = original(
            &surface,
            [true, true],
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let o = own_periodic_support_boundaries(
            input.clone(),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        assert!(o.boundaries().is_empty());
        assert!(Arc::ptr_eq(o.augmented(), &input));
        let knots = [0, 0, 1, 2, 3, 3, 4, 4].map(|n| Q::from_integer(n.into()));
        let mut work = 0;
        let values = basis(2, &knots, &Q::from_integer(3.into()), &mut work, 1000).unwrap();
        assert_eq!(
            values,
            vec![Q::zero(), Q::zero(), Q::zero(), Q::one(), Q::zero()]
        );
    }
}
