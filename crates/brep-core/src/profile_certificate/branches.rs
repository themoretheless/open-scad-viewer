//! Exact cylindrical branch graph. Critical fibers use algebraic Sturm
//! arithmetic; open-slab branches are ordered real root sections, with checked
//! one-sided incidences. No sampled continuation decides connectivity.
use super::*;
use algebraic_intersections::{ExactRatio, decode_ratio, ratio};
use arithmetic::algebraic_events::{self as roots, fibers};
use arithmetic::bivariate::{self as bi, Bi};
use arithmetic::{Poly, at, derivative, gcd, product, quotient, sub};
#[derive(Clone, Debug, PartialEq)]
pub struct BranchCut {
    pub parameter: [ExactRatio; 2],
    pub fibers: Vec<[ExactRatio; 2]>,
    pub vertical: bool,
    pub vertical_arcs: Vec<[usize; 2]>,
    /// Ordered branch ranks in the adjacent open slab -> ordered fiber node.
    pub left: Vec<usize>,
    pub right: Vec<usize>,
    pub excluded_nodes: Vec<bool>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BranchSlab {
    pub sample: ExactRatio,
    pub roots: Vec<[ExactRatio; 2]>,
    /// -1/+1: global first parameter minus global second. 0: deleted diagonal.
    pub diagonal_side: Vec<i8>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterBranchDecomposition {
    pub dimension: u8,
    pub projection: Vec<ExactRatio>,
    pub fiber_relation: Vec<Vec<ExactRatio>>,
    pub cuts: Vec<BranchCut>,
    pub slabs: Vec<BranchSlab>,
}
fn radical(p: &Poly) -> Result<Poly> {
    if p.is_empty() {
        return Ok(Vec::new());
    }
    quotient(p, &gcd(p, &derivative(p))?)
}
fn union(a: &Poly, b: &Poly) -> Result<Poly> {
    if b.len() < 2 {
        return Ok(a.clone());
    }
    let r = product(a, &quotient(&radical(b)?, &gcd(a, &radical(b)?)?)?);
    if r.len() > 257
        || r.iter()
            .any(|q| q.numer().bits() > 16384 || q.denom().bits() > 16384)
    {
        return Err(refused("Branch projection budget exceeded"));
    }
    Ok(r)
}
fn derivative_t(p: &Bi) -> Bi {
    p.iter()
        .enumerate()
        .skip(1)
        .map(|(i, r)| r.iter().map(|x| x * Q::from_integer(i.into())).collect())
        .collect()
}
fn at_t(p: &Bi, t: &Poly) -> Poly {
    p.iter().rev().fold(Vec::new(), |v, c| {
        sub(c, &product(&v, t).iter().map(|x| -x).collect::<Poly>())
    })
}
fn multiply_t(a: &Bi, b: &Bi) -> Bi {
    let mut p = vec![Vec::new(); a.len() + b.len() - 1];
    for (i, a) in a.iter().enumerate() {
        for (j, b) in b.iter().enumerate() {
            p[i + j] = sub(
                &p[i + j],
                &product(a, b).iter().map(|x| -x).collect::<Poly>(),
            )
        }
    }
    p
}
fn enc(i: &[Q; 2]) -> [ExactRatio; 2] {
    [ratio(&i[0]), ratio(&i[1])]
}
/// Rational samples in every open sign-invariant interval of an exact finite
/// family. The boundaries stay algebraic; no rounded critical parameter is used.
pub(crate) fn critical_samples(polynomials: &[Vec<Q>]) -> Result<Vec<Q>> {
    let mut p = vec![Q::zero(), -Q::one(), Q::one()];
    for q in polynomials {
        let mut q = q.clone();
        while q.last().is_some_and(Zero::is_zero) {
            q.pop();
        }
        p = union(&p, &q)?
    }
    let mut width = Q::new(1.into(), (1u64 << 20).into());
    for _ in 0..256 {
        let cuts = exact_roots(&p, &width)?;
        if cuts.windows(2).all(|v| v[0][1] < v[1][0]) {
            return Ok(cuts
                .windows(2)
                .map(|v| (&v[0][1] + &v[1][0]) / Q::from_integer(2.into()))
                .collect());
        }
        width /= Q::from_integer(2.into());
    }
    Err(refused("Piecewise trace projection budget exhausted"))
}
fn exact_roots(p: &Poly, width: &Q) -> Result<Vec<[Q; 2]>> {
    if p.is_empty() {
        return Err(refused("A branch sample became an entire fiber"));
    }
    roots::isolates(p, Q::zero(), Q::one(), width)
}
fn incidence(
    p: &Bi,
    projection: &Poly,
    cut: &[Q; 2],
    nodes: &[[Q; 2]],
    previous: Option<&[Q; 2]>,
    next: Option<&[Q; 2]>,
    width: &Q,
) -> Result<(Vec<usize>, Vec<usize>)> {
    let guards = nodes
        .windows(2)
        .map(|v| (&v[0][1] + &v[1][0]) / Q::from_integer(2.into()))
        .collect::<Vec<_>>();
    let mut i = cut.clone();
    let mut l = previous.map(|p| (&p[1] + &i[0]) / Q::from_integer(2.into()));
    let mut r = next.map(|p| (&p[0] + &i[1]) / Q::from_integer(2.into()));
    let stable = |a: &Q, b: &Q| -> Result<bool> {
        for g in &guards {
            let f = at_t(p, &vec![g.clone()]);
            if f.is_empty()
                || at(&f, a).is_zero()
                || at(&f, b).is_zero()
                || roots::count(&f, a, b)? > 0
            {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let rank = |s: &Q| -> Result<Vec<usize>> {
        let f = bi::fiber(p, s);
        let mut w = width.clone();
        for _ in 0..192 {
            let values = exact_roots(&f, &w)?;
            let mut out = Vec::new();
            let mut complete = true;
            for value in values {
                if guards.iter().any(|g| value[0] <= *g && *g <= value[1]) {
                    complete = false;
                    break;
                }
                out.push(guards.iter().take_while(|g| **g < value[0]).count());
            }
            if complete {
                if out.iter().any(|j| *j >= nodes.len()) {
                    return Err(refused("Branch limit has no critical fiber node"));
                }
                return Ok(out);
            }
            w /= Q::from_integer(2.into());
        }
        Err(refused("Branch incidence isolation budget exhausted"))
    };
    for _ in 0..256 {
        let a = l.clone().unwrap_or_else(|| i[0].clone());
        let b = r.clone().unwrap_or_else(|| i[1].clone());
        if stable(&a, &b)? {
            return Ok((
                l.as_ref().map(rank).transpose()?.unwrap_or_default(),
                r.as_ref().map(rank).transpose()?.unwrap_or_default(),
            ));
        }
        if cut[0] == cut[1] {
            if let Some(q) = &mut l {
                *q = (&*q + &cut[0]) / Q::from_integer(2.into())
            }
            if let Some(q) = &mut r {
                *q = (&*q + &cut[0]) / Q::from_integer(2.into())
            }
        } else {
            i = fibers::refine_root(projection, &i)?;
            l = previous.map(|_| i[0].clone());
            r = next.map(|_| i[1].clone());
        }
    }
    Err(refused("Branch one-sided incidence proof budget exhausted"))
}
pub(super) fn decompose(
    h: &Bi,
    diagonal: Option<[Q; 2]>,
    width: &Q,
) -> Result<ParameterBranchDecomposition> {
    if h.is_empty() {
        return Ok(ParameterBranchDecomposition {
            dimension: 2,
            projection: Vec::new(),
            fiber_relation: Vec::new(),
            cuts: Vec::new(),
            slabs: Vec::new(),
        });
    }
    let content = h.iter().try_fold(Vec::new(), |g, p| gcd(&g, p))?;
    let p = bi::primitive(h.clone())?;
    let p = bi::exact_bivariate_quotient(&p, &bi::common(p.clone(), derivative_t(&p))?)?;
    let mut projection = vec![Q::zero(), -Q::one(), Q::one()];
    projection = union(&projection, &content)?;
    projection = union(&projection, p.last().unwrap())?;
    if p.len() > 1 {
        projection = union(&projection, &bi::resultant(&p, &derivative_t(&p))?)?;
    }
    for t in [Q::zero(), Q::one()] {
        projection = union(&projection, &at_t(&p, &vec![t]))?;
    }
    let dpoly = diagonal.as_ref().map(|d| d.to_vec());
    let diagonal_projection = dpoly.as_ref().map(|d| at_t(&p, d));
    if let Some(d) = &diagonal_projection {
        projection = union(&projection, d)?;
    }
    let leading = projection.last().unwrap().clone();
    for x in &mut projection {
        *x /= &leading
    }
    let mut w = width.clone();
    let mut cuts;
    loop {
        cuts = exact_roots(&projection, &w)?;
        if cuts.windows(2).all(|v| v[0][1] < v[1][0]) {
            break;
        }
        w /= Q::from_integer(2.into());
        if w.denom().bits() > 1024 {
            return Err(refused("Critical branch roots cannot be separated"));
        }
    }
    let mut out = Vec::new();
    for (k, i) in cuts.iter().enumerate() {
        let vertical = fibers::zero(&content, &projection, i)?;
        let mut fiber = p.clone();
        if vertical {
            fiber = multiply_t(&fiber, &vec![Vec::new(), vec![Q::one()]]);
            fiber = multiply_t(&fiber, &vec![vec![-Q::one()], vec![Q::one()]]);
            if let Some(d) = &dpoly {
                fiber = multiply_t(
                    &fiber,
                    &vec![d.iter().map(|q| -q).collect(), vec![Q::one()]],
                )
            }
        }
        let mut fw = width.clone();
        let mut nodes;
        loop {
            nodes = fibers::isolate_at(&fiber, &projection, i, &fw)?;
            if nodes.windows(2).all(|v| v[0][1] < v[1][0]) {
                break;
            }
            fw /= Q::from_integer(2.into());
            if fw.denom().bits() > 1024 {
                return Err(refused("Critical fiber nodes cannot be separated"));
            }
        }
        let mut excluded_nodes = vec![false; nodes.len()];
        if let Some(d) = &dpoly {
            if vertical || fibers::zero(diagonal_projection.as_ref().unwrap(), &projection, i)? {
                let linear = vec![d.iter().map(|q| -q).collect(), vec![Q::one()]];
                let mut accounted = false;
                for _ in 0..192 {
                    let dr = fibers::isolate_at(&linear, &projection, i, &fw)?;
                    if dr.is_empty() {
                        accounted = true;
                        break;
                    }
                    let candidates = nodes
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| n[0] <= dr[0][1] && dr[0][0] <= n[1])
                        .map(|(j, _)| j)
                        .collect::<Vec<_>>();
                    if candidates.len() == 1 {
                        excluded_nodes[candidates[0]] = true;
                        accounted = true;
                        break;
                    }
                    fw /= Q::from_integer(2.into());
                    nodes = fibers::isolate_at(&fiber, &projection, i, &fw)?;
                    if fw.denom().bits() > 1024 {
                        return Err(refused("Diagonal critical node is unproved"));
                    }
                }
                if !accounted {
                    return Err(refused("Diagonal critical node isolation exhausted"));
                }
            }
        }
        let (left, right) = incidence(
            &p,
            &projection,
            i,
            &nodes,
            k.checked_sub(1).map(|j| &cuts[j]),
            cuts.get(k + 1),
            &fw,
        )?;
        let vertical_arcs = if vertical {
            (0..nodes.len().saturating_sub(1))
                .map(|j| [j, j + 1])
                .collect()
        } else {
            Vec::new()
        };
        out.push(BranchCut {
            parameter: enc(i),
            fibers: nodes.iter().map(enc).collect(),
            vertical,
            vertical_arcs,
            left,
            right,
            excluded_nodes,
        });
    }
    let mut slabs = Vec::new();
    for (k, cut) in cuts.windows(2).enumerate() {
        let sample = (&cut[0][1] + &cut[1][0]) / Q::from_integer(2.into());
        let f = bi::fiber(&p, &sample);
        let mut fw = width.clone();
        let mut values;
        let sides;
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 256 {
                return Err(refused("Diagonal branch classification exhausted"));
            }
            values = exact_roots(&f, &fw)?;
            if let Some(d) = &dpoly {
                let t = at(d, &sample);
                let on = at(&f, &t).is_zero();
                if values.iter().any(|i| i[0] <= t && t <= i[1] && !on) {
                    fw /= Q::from_integer(2.into());
                    continue;
                }
                sides = values
                    .iter()
                    .map(|i| {
                        if i[1] < t {
                            1
                        } else if i[0] > t {
                            -1
                        } else {
                            0
                        }
                    })
                    .collect();
            } else {
                sides = Vec::new()
            }
            break;
        }
        if out[k].right.len() != values.len() || out[k + 1].left.len() != values.len() {
            return Err(refused("Branch graph incidence omits a root section"));
        }
        slabs.push(BranchSlab {
            sample: ratio(&sample),
            roots: values.iter().map(enc).collect(),
            diagonal_side: sides,
        });
    }
    Ok(ParameterBranchDecomposition {
        dimension: 1,
        projection: projection.iter().map(ratio).collect(),
        fiber_relation: p.iter().map(|r| r.iter().map(ratio).collect()).collect(),
        cuts: out,
        slabs,
    })
}

#[cfg(feature = "codec")]
mod codec {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for BranchCut {
        fn to_value(&self) -> Value {
            json!({"parameter":self.parameter,"fibers":self.fibers,"vertical":self.vertical,"verticalArcs":self.vertical_arcs,"left":self.left,"right":self.right,"excludedNodes":self.excluded_nodes})
        }
    }
    impl Serialize for BranchSlab {
        fn to_value(&self) -> Value {
            json!({"sample":self.sample,"roots":self.roots,"diagonalSide":self.diagonal_side})
        }
    }
    impl Serialize for ParameterBranchDecomposition {
        fn to_value(&self) -> Value {
            json!({"dimension":self.dimension,"projection":self.projection,"fiberRelation":self.fiber_relation,"cuts":self.cuts,"slabs":self.slabs})
        }
    }
    fn schema(v: &Value, keys: &[&str]) -> value_codec::Result<()> {
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
        {
            return Err(error("Branch decomposition schema mismatch"));
        }
        Ok(())
    }
    impl<'de> Deserialize<'de> for BranchCut {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(
                &v,
                &[
                    "parameter",
                    "fibers",
                    "vertical",
                    "verticalArcs",
                    "left",
                    "right",
                    "excludedNodes",
                ],
            )?;
            if v["fibers"].as_array().is_none_or(|a| a.len() > 64) {
                return Err(error("Branch fiber budget"));
            }
            Ok(Self {
                parameter: Deserialize::from_value(v["parameter"].clone())?,
                fibers: Deserialize::from_value(v["fibers"].clone())?,
                vertical: Deserialize::from_value(v["vertical"].clone())?,
                vertical_arcs: Deserialize::from_value(v["verticalArcs"].clone())?,
                left: Deserialize::from_value(v["left"].clone())?,
                right: Deserialize::from_value(v["right"].clone())?,
                excluded_nodes: Deserialize::from_value(v["excludedNodes"].clone())?,
            })
        }
    }
    impl<'de> Deserialize<'de> for BranchSlab {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(&v, &["sample", "roots", "diagonalSide"])?;
            Ok(Self {
                sample: Deserialize::from_value(v["sample"].clone())?,
                roots: Deserialize::from_value(v["roots"].clone())?,
                diagonal_side: Deserialize::from_value(v["diagonalSide"].clone())?,
            })
        }
    }
    impl<'de> Deserialize<'de> for ParameterBranchDecomposition {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(
                &v,
                &["dimension", "projection", "fiberRelation", "cuts", "slabs"],
            )?;
            if v["cuts"].as_array().is_none_or(|a| a.len() > 257)
                || v["slabs"].as_array().is_none_or(|a| a.len() > 256)
            {
                return Err(error("Branch census budget"));
            }
            let out = Self {
                dimension: Deserialize::from_value(v["dimension"].clone())?,
                projection: Deserialize::from_value(v["projection"].clone())?,
                fiber_relation: Deserialize::from_value(v["fiberRelation"].clone())?,
                cuts: Deserialize::from_value(v["cuts"].clone())?,
                slabs: Deserialize::from_value(v["slabs"].clone())?,
            };
            for r in &out.projection {
                decode_ratio(r).map_err(|e| error(e.message))?;
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(n: i64) -> Q {
        Q::from_integer(n.into())
    }
    fn w() -> Q {
        Q::new(1.into(), (1u64 << 30).into())
    }
    #[test]
    fn irrational_circle_extrema_have_complete_two_sided_incidence() {
        let h = vec![
            vec![Q::new(3.into(), 8.into()), q(-1), q(1)],
            vec![q(-1)],
            vec![q(1)],
        ];
        let g = decompose(&h, None, &w()).unwrap();
        assert_eq!(g.cuts.len(), 4);
        assert_eq!(
            g.slabs.iter().map(|s| s.roots.len()).collect::<Vec<_>>(),
            vec![0, 2, 0]
        );
        assert_eq!(g.cuts[1].right, vec![0, 0]);
        assert_eq!(g.cuts[2].left, vec![0, 0]);
        assert_ne!(g.cuts[1].parameter[0], g.cuts[1].parameter[1]);
    }
    #[test]
    fn nodes_cusps_and_isolated_real_points_do_not_use_sampled_connectivity() {
        let h = vec![vec![q(0), q(1), q(-1)], vec![q(-1)], vec![q(1)]];
        let g = decompose(&h, None, &w()).unwrap();
        assert_eq!(g.cuts.len(), 3);
        assert_eq!(g.cuts[1].left, vec![0, 0]);
        assert_eq!(g.cuts[1].right, vec![0, 0]);
        let h = vec![
            vec![Q::new(1.into(), 2.into()), q(-1), q(1)],
            vec![q(-1)],
            vec![q(1)],
        ];
        let g = decompose(&h, None, &w()).unwrap();
        assert_eq!(g.cuts[1].fibers.len(), 1);
        assert!(g.slabs.iter().all(|s| s.roots.is_empty()));
        // Semicubical cusp t²=(s-1/2)³, with t=0 on the domain boundary.
        let h = vec![
            vec![
                Q::new(1.into(), 8.into()),
                Q::new((-3).into(), 4.into()),
                Q::new(3.into(), 2.into()),
                q(-1),
            ],
            vec![],
            vec![q(1)],
        ];
        let g = decompose(&h, None, &w()).unwrap();
        assert!(g.slabs.iter().any(|s| s.roots.len() == 1));
    }
    #[test]
    fn irrational_vertical_component_is_split_at_its_branch_joint() {
        let c = vec![Q::new((-1).into(), 2.into()), q(0), q(1)];
        let h = vec![product(&c, &vec![q(0), q(-1)]), c];
        let g = decompose(&h, Some([q(0), q(1)]), &w()).unwrap();
        let c = g.cuts.iter().find(|c| c.vertical).unwrap();
        assert_eq!(c.fibers.len(), 3);
        assert_eq!(c.excluded_nodes, vec![false, true, false]);
        assert_eq!(c.left, vec![1]);
        assert_eq!(c.right, vec![1]);
    }
    #[test]
    fn graph_witness_is_bound_to_source_and_every_incidence() {
        assert_eq!(
            critical_samples(&[vec![q(0), q(0)], vec![q(1), q(0)]]).unwrap(),
            vec![Q::new(1.into(), 2.into())]
        );
        let a = Curve {
            degree: 2,
            control_points: vec![vec![0., 0.], vec![2., 0.], vec![0., 0.]],
            weights: vec![1.; 3],
            knots: vec![0., 0., 0., 1., 1., 1.],
            periodic: false,
        };
        let c = certify_span_intersections(&a, &a, true, 1e-10).unwrap();
        assert!(c.pairs.iter().any(|p| p.branches.is_some()));
        let mut bad = c.clone();
        bad.pairs
            .iter_mut()
            .find_map(|p| p.branches.as_mut())
            .unwrap()
            .cuts[0]
            .right
            .clear();
        assert!(verify_span_intersections(&a, &a, &bad).is_err());
        let mut bad = c.clone();
        bad.pairs[0].relation = vec![vec![["0".into(), "1".into()]]];
        assert!(verify_span_intersections(&a, &a, &bad).is_err());
        let mut legacy = c;
        legacy.version = 2;
        for p in &mut legacy.pairs {
            p.branches = None
        }
        assert!(verify_span_intersections(&a, &a, &legacy).is_ok());
    }
}
