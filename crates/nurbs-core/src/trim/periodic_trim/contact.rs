//! Exact angular ownership at balanced endpoint contacts of oriented chains.
use super::chart::{ChartEdge, vertices};
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
    exact_products::sum_sign,
};
use std::cmp::Ordering;
mod coefficient;
pub use coefficient::PoweredBendCoefficient;
#[derive(Clone, Debug)]
pub struct ContactPort {
    pub edge: usize,
    pub incoming: bool,
    /// First control distinct from the endpoint in its clamped knot span.
    /// The outward ray is this point minus ContactOwnership::point, exactly.
    pub outward_control: [f64; 2],
    pub vanishing_order: usize,
    /// Exact side of the first nonzero normal term: -1 clockwise, +1 CCW.
    /// Zero means the complete endpoint span is collinear with its leading ray.
    pub bend_side: i8,
    pub normal_vanishing_order: Option<usize>,
    /// Outward signed curvature/2 enclosure for regular k=1, m=2 branches.
    /// It is the leading angular coefficient at equal radial distance.
    pub regular_bend_coefficient: Option<[f64; 2]>,
    pub radial_coefficient_power: Option<PoweredBendCoefficient>,
}
impl ContactPort {
    /// Exact rational exponent of the angular bend as a function of radial
    /// distance from the vertex: angle ~ signed_constant * radius^exponent.
    /// This is invariant under independent one-sided reparameterizations.
    pub fn radial_bend_exponent(&self) -> Option<[usize; 2]> {
        let k = self.vanishing_order;
        if k == 0 {
            return None;
        }
        let numerator = self.normal_vanishing_order?.checked_sub(k)?;
        (numerator > 0).then_some([numerator, k])
    }
}
#[derive(Clone, Debug)]
pub struct ContactOwnership {
    pub point: [f64; 2],
    /// Counterclockwise exact order, without materialized vector subtraction.
    pub ports: Vec<ContactPort>,
    /// Incoming -> outgoing, bounding the intervening oriented left sector.
    pub pairs: Vec<[usize; 2]>,
}
#[derive(Clone, Debug)]
pub struct ContactOwnershipReport {
    pub complete: bool,
    pub contacts: Vec<ContactOwnership>,
    /// Full edge permutation only after every vertex is admitted.
    pub successors: Option<Vec<usize>>,
    pub ports: usize,
    pub reason: Option<&'static str>,
}
fn ray(c: &Curve, incoming: bool) -> Option<([f64; 2], usize)> {
    let n = c.control_points.len();
    let origin = &c.control_points[if incoming { n - 1 } else { 0 }];
    (1..=c.degree).find_map(|order| {
        let p = &c.control_points[if incoming { n - 1 - order } else { order }];
        (p != origin).then_some(([p[0], p[1]], order))
    })
}
fn half(a: [f64; 2], o: [f64; 2]) -> bool {
    let y = sum_sign(&[(a[1], 1.), (-o[1], 1.)]);
    y == Ordering::Less
        || (y == Ordering::Equal && sum_sign(&[(a[0], 1.), (-o[0], 1.)]) == Ordering::Less)
}
fn compare(a: [f64; 2], b: [f64; 2], o: [f64; 2]) -> Ordering {
    let ha = half(a, o);
    let hb = half(b, o);
    if ha != hb {
        return ha.cmp(&hb);
    }
    cross(a, b, o).reverse()
}
fn cross(a: [f64; 2], b: [f64; 2], o: [f64; 2]) -> Ordering {
    // Exact cross((a-o),(b-o)), including cancellation and subnormal products.
    sum_sign(&[
        (a[0], b[1]),
        (-a[0], o[1]),
        (-o[0], b[1]),
        (o[0], o[1]),
        (-a[1], b[0]),
        (a[1], o[0]),
        (o[1], b[0]),
        (-o[1], o[0]),
    ])
}
fn bend(c: &Curve, incoming: bool, leading: [f64; 2], o: [f64; 2]) -> (i8, Option<usize>) {
    let n = c.control_points.len();
    for order in 1..=c.degree {
        let p = &c.control_points[if incoming { n - 1 - order } else { order }];
        match cross(leading, [p[0], p[1]], o) {
            Ordering::Less => return (-1, Some(order)),
            Ordering::Greater => return (1, Some(order)),
            Ordering::Equal => {}
        }
    }
    (0, None)
}
fn regular_coefficient(
    c: &Curve,
    incoming: bool,
    order: usize,
    normal: Option<usize>,
) -> Option<[f64; 2]> {
    if order != 1 || normal != Some(2) {
        return None;
    }
    let calculate = || -> Result<I> {
        let n = c.control_points.len();
        let p = c.degree;
        let indices = if incoming {
            [n - 1, n - 2, n - 3]
        } else {
            [0, 1, 2]
        };
        let [w0, w1, w2] = indices.map(|i| I::point(c.weights[i]));
        let point = |i: usize| {
            [
                I::point(c.control_points[i][0]),
                I::point(c.control_points[i][1]),
            ]
        };
        let o = point(indices[0]);
        let a = point(indices[1]);
        let b = point(indices[2]);
        let r = [a[0].sub(o[0])?, a[1].sub(o[1])?];
        let q = [b[0].sub(o[0])?, b[1].sub(o[1])?];
        let (lo, hi) = box_distance(&r, &[I::point(0.); 2])?;
        let norm = I::new(lo, hi)?;
        let cross = r[0].mul(q[1])?.sub(r[1].mul(q[0])?)?;
        let [a, b] = c.domain();
        let (h1, h2) = if incoming {
            (
                I::point(b).sub(I::point(c.knots[n - 1]))?,
                I::point(b).sub(I::point(c.knots[n - 2]))?,
            )
        } else {
            (
                I::point(c.knots[p + 1]).sub(I::point(a))?,
                I::point(c.knots[p + 2]).sub(I::point(a))?,
            )
        };
        // For a clamped rational B-spline endpoint, geometric curvature/2 is
        // (p-1)/(2p) * h1/h2 * w0*w2/w1^2 * cross(r,q)/|r|^3.
        // All terms collinear with r cancel before this outward evaluation.
        I::point((p - 1) as f64)
            .div(I::point((2 * p) as f64))?
            .mul(h1.div(h2)?)?
            .mul(w0.mul(w2)?.div(w1.mul(w1)?)?)?
            .mul(cross)?
            .div(norm.mul(norm)?.mul(norm)?)
    };
    calculate().ok().map(|v| [v.lo, v.hi])
}
fn compare_ports(a: &ContactPort, b: &ContactPort, o: [f64; 2]) -> Ordering {
    compare(a.outward_control, b.outward_control, o)
        .then_with(|| a.bend_side.cmp(&b.bend_side))
        .then_with(|| {
            if a.bend_side == 0 {
                return Ordering::Equal;
            }
            let (Some([an, ad]), Some([bn, bd])) =
                (a.radial_bend_exponent(), b.radial_bend_exponent())
            else {
                return Ordering::Equal;
            };
            // Orders are <= degree <= 25. A smaller positive exponent bends
            // farther from the common ray at sufficiently small equal radius.
            let exponent = (an * bd).cmp(&(bn * ad));
            if a.bend_side > 0 {
                exponent.reverse()
            } else {
                exponent
            }
        })
}
fn coefficient_interval(port: &ContactPort) -> Option<(usize, [f64; 2])> {
    if let Some(v) = port.regular_bend_coefficient {
        return Some((1, v));
    }
    let v = port.radial_coefficient_power?;
    if port.bend_side > 0 {
        Some((v.power, v.magnitude))
    } else if port.bend_side < 0 {
        Some((v.power, [-v.magnitude[1], -v.magnitude[0]]))
    } else {
        None
    }
}
/// Resolve balanced multibranch endpoint contacts with alternating one-sided
/// incoming/outgoing branches. A clamped positive-weight rational curve's first
/// distinct control determines its exact one-sided leading ray; higher-order
/// vanishing derivatives are retained. Equal leading rays are separated when
/// their exact bend sides differ (including a straight branch between them).
/// Equal bend sides are separated by different radial bend exponents, using
/// exact integer comparisons of normal/tangential vanishing orders. Equal
/// exponents are separated by disjoint outward powered radial coefficient
/// bounds; fractional powers are never estimated. Regular curvature/2 bounds
/// are retained as a tighter special case. Overlapping coefficient intervals and stationary spans
/// remain unproven. The permutation preserves the entire oriented
/// chain and its summed winding, not a regular face or a nonzero-fill owner.
/// The local convention pairs an outgoing ray with the next CCW incoming ray.
pub fn inspect_contact_ownership(
    edges: &[ChartEdge],
    max_ports: usize,
) -> Result<ContactOwnershipReport> {
    check(
        !edges.is_empty() && edges.len() <= 512 && (1..=1024).contains(&max_ports),
        "Invalid contact edges or port budget",
    )?;
    for edge in edges {
        let c = &edge.curve;
        c.validate()?;
        let [a, b] = c.domain();
        check(
            !c.periodic
                && c.control_points[0].len() == 2
                && c.knots[..=c.degree].iter().all(|k| *k == a)
                && c.knots[c.control_points.len()..].iter().all(|k| *k == b),
            "Contact ownership requires clamped nonperiodic UV curves",
        )?;
    }
    let mut out = ContactOwnershipReport {
        complete: false,
        contacts: Vec::new(),
        successors: None,
        ports: 0,
        reason: None,
    };
    let mut next = vec![usize::MAX; edges.len()];
    for vertex in vertices(edges) {
        if vertex.starts.is_empty() || vertex.starts.len() != vertex.ends.len() {
            out.reason = Some("unbalanced-contact-vertex");
            return Ok(out);
        }
        let count = vertex.starts.len() + vertex.ends.len();
        if out.ports + count > max_ports {
            out.reason = Some("contact-port-budget");
            return Ok(out);
        }
        out.ports += count;
        if vertex.starts.len() == 1 {
            next[vertex.ends[0]] = vertex.starts[0];
            continue;
        }
        let mut ports = Vec::with_capacity(count);
        for (incoming, indices) in [(false, &vertex.starts), (true, &vertex.ends)] {
            for &index in indices {
                let Some((control, order)) = ray(&edges[index].curve, incoming) else {
                    out.reason = Some("stationary-contact-branch");
                    return Ok(out);
                };
                let (bend_side, normal_vanishing_order) =
                    bend(&edges[index].curve, incoming, control, vertex.point);
                let regular = regular_coefficient(
                    &edges[index].curve,
                    incoming,
                    order,
                    normal_vanishing_order,
                );
                let powered = coefficient::enclose(
                    &edges[index].curve,
                    incoming,
                    order,
                    normal_vanishing_order,
                    regular,
                );
                ports.push(ContactPort {
                    edge: index,
                    incoming,
                    outward_control: control,
                    vanishing_order: order,
                    bend_side,
                    normal_vanishing_order,
                    regular_bend_coefficient: regular,
                    radial_coefficient_power: powered,
                });
            }
        }
        ports.sort_by(|a, b| {
            compare_ports(a, b, vertex.point).then_with(|| {
                // A total sorting key, not a proof: interval overlap is not a
                // transitive equality relation. Prove all adjacent gaps below.
                match (coefficient_interval(a), coefficient_interval(b)) {
                    (Some((ap, a)), Some((bp, b))) => {
                        ap.cmp(&bp).then_with(|| a[0].total_cmp(&b[0]))
                    }
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            })
        });
        for i in 0..ports.len() {
            let a = &ports[i];
            let b = &ports[(i + 1) % ports.len()];
            if compare_ports(a, b, vertex.point) == Ordering::Equal {
                let ordered = match (coefficient_interval(a), coefficient_interval(b)) {
                    (Some((ap, a)), Some((bp, b))) if ap == bp => {
                        if i + 1 == ports.len() {
                            b[1] < a[0]
                        } else {
                            a[1] < b[0]
                        }
                    }
                    _ => false,
                };
                if !ordered {
                    out.reason = Some("coincident-contact-rays");
                    return Ok(out);
                }
            }
        }
        if (0..ports.len()).any(|i| ports[i].incoming == ports[(i + 1) % ports.len()].incoming) {
            out.reason = Some("nonalternating-contact-rays");
            return Ok(out);
        }
        let mut pairs = Vec::new();
        for i in 0..ports.len() {
            if !ports[i].incoming {
                let incoming = ports[(i + 1) % ports.len()].edge;
                next[incoming] = ports[i].edge;
                pairs.push([incoming, ports[i].edge]);
            }
        }
        out.contacts.push(ContactOwnership {
            point: vertex.point,
            ports,
            pairs,
        });
    }
    out.complete = true;
    out.successors = Some(next);
    Ok(out)
}
