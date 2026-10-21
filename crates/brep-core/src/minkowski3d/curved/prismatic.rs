//! Ambient isotopy of a complete signed prism atlas, including nested voids.
//! A simultaneous Jordan-curve forest is checked in the original rational
//! profiles and in the *actual* approximation. One plane isotopy preserves all
//! profile memberships; its product with height preserves the complete filled
//! set and every component of its complement. No radial point is needed.
use super::*;
use crate::profile_certificate::topology_prisms as planar;
use nurbs_core::{curve::Curve, surface::Surface};
type Point = [Q; 2];
type Ring = Vec<Point>;

#[derive(Clone, Debug, PartialEq)]
pub struct ShellReport {
    pub shell: usize,
    pub interval: [Ratio; 2],
    pub curves: Vec<usize>,
    pub coefficients: Vec<i8>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub axis: usize,
    pub levels: Vec<Ratio>,
    pub shells: Vec<ShellReport>,
    pub parents: Vec<Option<usize>>,
    pub nominal_profile_vertices: usize,
}
#[derive(Clone)]
pub(super) struct Data {
    pub axis: usize,
    pub interval: [Q; 2],
    pub original: Option<Vec<Vec<Curve>>>,
    pub rings: Vec<Ring>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HoleReport {
    pub hole: usize,
    pub open: bool,
    pub depth: u8,
    pub inner_core: Vec<[Ratio; 2]>,
    pub outer_core: Vec<[Ratio; 2]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SumReport {
    pub axis: usize,
    pub interval: [Ratio; 2],
    pub holed_operand: usize,
    pub holes: Vec<HoleReport>,
    pub surviving_holes: usize,
}
fn axes(axis: usize) -> [usize; 2] {
    [(axis + 1) % 3, (axis + 2) % 3]
}
fn binary(q: &Q) -> Result<f64> {
    let x = q
        .to_f64()
        .ok_or_else(|| fail("prismatic profile exceeds binary range"))?;
    if Q::from_float(x).as_ref() != Some(q) {
        return Err(fail(
            "original prismatic profile transform is not exactly representable",
        ));
    }
    Ok(x)
}
fn reverse(c: &mut Curve) -> Result<()> {
    let d = c.domain();
    let sum = Q::from_float(d[0]).unwrap() + Q::from_float(d[1]).unwrap();
    c.knots = c
        .knots
        .iter()
        .rev()
        .map(|x| binary(&(&sum - Q::from_float(*x).unwrap())))
        .collect::<Result<_>>()?;
    c.control_points.reverse();
    c.weights.reverse();
    Ok(())
}
fn key(wire: &[Curve]) -> Result<Vec<u8>> {
    value_codec::encode_binary(&wire.to_vec().to_value()).map_err(|e| fail(&e.to_string()))
}
fn canonical_wire(mut wire: Vec<Curve>, ccw: bool) -> Result<Vec<Curve>> {
    if !ccw {
        for c in &mut wire {
            reverse(c)?;
        }
        wire.reverse();
    }
    let mut best = key(&wire)?;
    let mut shift = 0;
    for n in 1..wire.len() {
        wire.rotate_left(1);
        let k = key(&wire)?;
        if k < best {
            best = k;
            shift = n;
        }
    }
    // The loop above ends at rotation len-1.
    let rotation = (shift + 1) % wire.len();
    wire.rotate_left(rotation);
    Ok(wire)
}
fn cap(m: &Model, face: usize, axis: usize) -> Result<(Q, Vec<Vec<Curve>>, Vec<i8>)> {
    let f = &m.faces[face];
    let [x, y] = axes(axis);
    let a = sb::affine(&f.surface)?
        .ok_or_else(|| fail("prismatic cap is not an exact affine chart"))?;
    if !a[1][axis].is_zero()
        || !a[2][axis].is_zero()
        || (&a[1][x] * &a[2][y] - &a[1][y] * &a[2][x]).is_zero()
    {
        return Err(fail("prismatic cap is not transverse to its height axis"));
    }
    let mut loops = Vec::new();
    for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        let mut curves = Vec::new();
        for c in &m.loops[w].coedges {
            let mut pc = c.pcurve.clone();
            pc.control_points = pc
                .control_points
                .iter()
                .map(|p| {
                    [x, y]
                        .iter()
                        .map(|&k| {
                            binary(
                                &(&a[0][k]
                                    + Q::from_float(p[0]).unwrap() * &a[1][k]
                                    + Q::from_float(p[1]).unwrap() * &a[2][k]),
                            )
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()?;
            curves.push(pc);
        }
        loops.push(curves);
    }
    let (orientation, nesting) = planar::original_wires(&loops)?;
    if nesting
        .iter()
        .enumerate()
        .any(|(i, d)| *d != usize::from(i != 0))
        || orientation
            .iter()
            .enumerate()
            .any(|(i, o)| *o != orientation[0] * if i == 0 { 1 } else { -1 })
    {
        return Err(fail(
            "prismatic cap is not one oriented region with disjoint holes",
        ));
    }
    Ok((a[0][axis].clone(), loops, orientation))
}
fn ruled(s: &Surface, axis: usize, interval: &[Q; 2], transpose: bool) -> bool {
    let rows = if transpose {
        s.control_points[0].len()
    } else {
        s.control_points.len()
    };
    let columns = if transpose {
        s.control_points.len()
    } else {
        s.control_points[0].len()
    };
    let p = |i: usize, j: usize| {
        if transpose {
            &s.control_points[j][i]
        } else {
            &s.control_points[i][j]
        }
    };
    let w = |i: usize, j: usize| {
        if transpose {
            s.weights[j][i]
        } else {
            s.weights[i][j]
        }
    };
    let (degree, knots) = if transpose {
        (s.degree_u, &s.knots_u)
    } else {
        (s.degree_v, &s.knots_v)
    };
    let v0 = Q::from_float(knots[degree]).unwrap();
    let v1 = Q::from_float(knots[columns]).unwrap();
    for reversed in [false, true] {
        let [a, b] = if reversed {
            [interval[1].clone(), interval[0].clone()]
        } else {
            interval.clone()
        };
        let valid = (0..rows).all(|i| {
            (0..columns).all(|j| {
                let v = knots[j + 1..=j + degree]
                    .iter()
                    .map(|v| Q::from_float(*v).unwrap())
                    .sum::<Q>()
                    / integer(degree as i64);
                w(i, j) == w(i, 0)
                    && axes(axis).iter().all(|&k| p(i, j)[k] == p(i, 0)[k])
                    && Q::from_float(p(i, j)[axis]).unwrap()
                        == &a + (&b - &a) * (v - &v0) / (&v1 - &v0)
            })
        });
        if valid {
            return true;
        }
    }
    false
}
fn stations(m: &Model, depth: u8) -> Result<Vec<Q>> {
    let mut uses = Vec::new();
    for b in &m.bodies {
        for s in std::iter::once(b.outer_shell).chain(b.inner_shells.iter().copied()) {
            uses.extend(m.shells[s].faces.iter().cloned());
        }
    }
    let mut st = BTreeSet::from([Q::zero(), Q::one()]);
    for u in &uses {
        let s = &m.faces[u.face].surface;
        for (ks, d, n) in [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ] {
            let a = Q::from_float(ks[d]).unwrap();
            let b = Q::from_float(ks[n]).unwrap();
            for k in &ks[d..=n] {
                let t = (Q::from_float(*k).unwrap() - &a) / (&b - &a);
                st.insert(t.clone());
                st.insert(Q::one() - t);
            }
        }
    }
    trimmed::stations(m, &uses, &mut st)?;
    for i in 0..=1usize << depth {
        st.insert(Q::new(i.into(), (1usize << depth).into()));
    }
    if st.len() > 33 {
        return Err(fail("prismatic station budget exceeded"));
    }
    Ok(st.into_iter().collect())
}
fn sample(wire: &[Curve], stations: &[Q]) -> Result<Ring> {
    let mut ring = Vec::new();
    for c in wire {
        let d = c.domain();
        let lo = Q::from_float(d[0]).unwrap();
        let len = Q::from_float(d[1]).unwrap() - &lo;
        for t in &stations[..stations.len() - 1] {
            let t = &lo + &len * t;
            let net = sb::curve(c, &t, &t)?;
            let p = sb::point(&net[0]);
            let p = [p[0].clone(), p[1].clone()];
            if ring.last() != Some(&p) {
                ring.push(p)
            }
        }
    }
    if ring.first() == ring.last() {
        ring.pop();
    }
    Ok(ring)
}
fn extrusion_chain(shells: &[ShellReport], rings: &[Ring], axis: usize) -> Result<Input> {
    let [x, _] = axes(axis);
    let mut points = BTreeMap::<P, u32>::new();
    let mut triangles = Vec::new();
    let mut add_triangle = |mut p: [P; 3], sign: i8| {
        if sign < 0 {
            p.swap(1, 2)
        }
        if zero(&cross(&sub(&p[1], &p[0]), &sub(&p[2], &p[0]))) {
            return;
        }
        for p in p {
            let n = points.len() as u32;
            triangles.push(*points.entry(p).or_insert(n));
        }
    };
    for s in shells {
        let z = s
            .interval
            .clone()
            .map(|r| Q::new(r[0].parse().unwrap(), r[1].parse().unwrap()));
        for (&wire, &sign) in s.curves.iter().zip(&s.coefficients) {
            let r = &rings[wire];
            let center: Point = std::array::from_fn(|k| {
                r.iter().map(|p| &p[k]).sum::<Q>() / integer(r.len() as i64)
            });
            let point = |p: &Point, k: usize| {
                std::array::from_fn(|i| {
                    if i == axis {
                        z[k].clone()
                    } else if i == x {
                        p[0].clone()
                    } else {
                        p[1].clone()
                    }
                })
            };
            for i in 0..r.len() {
                let a = &r[i];
                let b = &r[(i + 1) % r.len()];
                add_triangle([point(a, 0), point(&center, 0), point(b, 0)], sign);
                add_triangle([point(a, 1), point(b, 1), point(&center, 1)], sign);
                add_triangle([point(a, 0), point(b, 0), point(b, 1)], sign);
                add_triangle([point(a, 0), point(b, 1), point(a, 1)], sign);
            }
        }
    }
    let mut p = vec![[Q::zero(), Q::zero(), Q::zero()]; points.len()];
    for (v, i) in points {
        p[i as usize] = v;
    }
    input_chain(&p, &triangles, false)
}
fn same_material(a: &[Poly], b: &[Poly], work: &mut Work) -> Result<()> {
    if boundary_dyadic(a, work)? != boundary_dyadic(b, work)? {
        return Err(fail(
            "actual nominal material is not the checked signed prism atlas",
        ));
    }
    Ok(())
}
fn inspect_brep(
    m: &Model,
    depth: u8,
    nominal: &[Poly],
    axis: usize,
    work: &mut Work,
) -> Result<(Report, Option<Data>)> {
    let st = stations(m, depth)?;
    let mut family = Vec::<Vec<Curve>>::new();
    let mut keys = BTreeMap::new();
    let mut shells = Vec::new();
    let mut levels = BTreeSet::new();
    let mut single = None;
    for b in &m.bodies {
        for shell in std::iter::once(b.outer_shell).chain(b.inner_shells.iter().copied()) {
            let uses = &m.shells[shell].faces;
            let horizontal = uses
                .iter()
                .filter(|u| {
                    let s = &m.faces[u.face].surface;
                    let z = s.control_points[0][0][axis];
                    s.control_points.iter().flatten().all(|p| p[axis] == z)
                })
                .collect::<Vec<_>>();
            if horizontal.len() != 2 {
                return Err(fail("prismatic shell needs exactly two horizontal caps"));
            }
            let mut caps = horizontal
                .iter()
                .map(|u| {
                    let (z, w, o) = cap(m, u.face, axis)?;
                    Ok((z, w, o, u.reversed))
                })
                .collect::<Result<Vec<_>>>()?;
            caps.sort_by(|a, b| a.0.cmp(&b.0));
            if caps[0].0 >= caps[1].0 {
                return Err(fail("prismatic height interval is empty"));
            }
            let interval = [caps[0].0.clone(), caps[1].0.clone()];
            for u in uses {
                if horizontal.iter().any(|h| h.face == u.face) {
                    continue;
                }
                rectangle(m, u.face)?;
                let s = &m.faces[u.face].surface;
                if !ruled(s, axis, &interval, false) && !ruled(s, axis, &interval, true) {
                    return Err(fail("source side is not an exact Cartesian extrusion"));
                }
            }
            let sign = caps[1].2[0] * if caps[1].3 { -1 } else { 1 };
            if caps[0].2[0] * if caps[0].3 { -1 } else { 1 } != -sign {
                return Err(fail("prismatic cap orientations disagree"));
            }
            let mut low = Vec::new();
            for (w, o) in caps[0].1.iter().zip(&caps[0].2) {
                low.push(key(&canonical_wire(w.clone(), *o > 0)?)?);
            }
            let mut ids = Vec::new();
            let mut coeff = Vec::new();
            let mut original = Vec::new();
            for (i, (w, o)) in caps[1].1.iter().zip(&caps[1].2).enumerate() {
                let w = canonical_wire(w.clone(), *o > 0)?;
                let k = key(&w)?;
                if !low.contains(&k) {
                    return Err(fail(
                        "prismatic caps have different original profile curves",
                    ));
                }
                let id = if let Some(id) = keys.get(&k) {
                    *id
                } else {
                    let n = family.len();
                    family.push(w.clone());
                    keys.insert(k, n);
                    n
                };
                ids.push(id);
                coeff.push(sign * if i == 0 { 1 } else { -1 });
                let mut w = w;
                if i > 0 {
                    for c in &mut w {
                        reverse(c)?;
                    }
                    w.reverse();
                }
                original.push(w);
            }
            if low.len() != ids.len() {
                return Err(fail("prismatic cap wire census differs"));
            }
            if shells.is_empty() && sign == 1 {
                single = Some((interval.clone(), original));
            } else {
                single = None;
            }
            levels.extend(interval.iter().cloned());
            shells.push(ShellReport {
                shell,
                interval: interval.map(|q| ratio(&q)),
                curves: ids,
                coefficients: coeff,
            });
        }
    }
    if shells.is_empty() {
        return Err(fail("prismatic source has no shells"));
    }
    let parents = planar::original_forest(&family)?;
    let rings = family
        .iter()
        .map(|w| sample(w, &st))
        .collect::<Result<Vec<_>>>()?;
    if planar::polygon_forest(&rings)? != parents {
        return Err(fail(
            "approximation changes the original planar containment forest",
        ));
    }
    let expected = decompose(&extrusion_chain(&shells, &rings, axis)?, work)?;
    same_material(&expected, nominal, work)?;
    let data = if shells.len() == 1 {
        single.map(|(interval, original)| Data {
            axis,
            interval,
            original: Some(original),
            rings: vec![],
        })
    } else {
        None
    };
    Ok((
        Report {
            axis,
            levels: levels.iter().map(ratio).collect(),
            shells,
            parents,
            nominal_profile_vertices: rings.iter().map(Vec::len).sum(),
        },
        data,
    ))
}
/// Reconstruct the exact positive-filled source and nominal chains. A failure
/// means no claim; legacy radial receipts and enclosure bounds stay separate.
pub(super) fn inspect(
    s: &Source,
    depth: u8,
    nominal: &[Poly],
    work: &mut Work,
) -> Option<(Report, Option<Data>)> {
    if let Source::Brep(m) = s {
        for axis in [2, 0, 1] {
            match inspect_brep(m, depth, nominal, axis, work) {
                Ok(r) => return Some(r),
                Err(e) => {
                    #[cfg(test)]
                    eprintln!("Prismatic axis {axis} unproved: {}", e.message);
                    #[cfg(not(test))]
                    let _ = e;
                }
            }
        }
    }
    None
}

fn projected_product(cells: &[Poly], axis: usize) -> Result<Data> {
    let [x, y] = axes(axis);
    let all = cells.iter().flat_map(Poly::vertices).collect::<Vec<_>>();
    let lo = all
        .iter()
        .map(|p| &p[axis])
        .min()
        .ok_or_else(|| fail("empty prismatic source"))?
        .clone();
    let hi = all.iter().map(|p| &p[axis]).max().unwrap().clone();
    if lo >= hi {
        return Err(fail("empty prismatic height"));
    }
    let mut profiles = Vec::new();
    for c in cells {
        let mut bottom = BTreeSet::new();
        let mut top = BTreeSet::new();
        for p in c.vertices() {
            let xy = [p[x].clone(), p[y].clone()];
            if p[axis] == lo {
                bottom.insert(xy);
            } else if p[axis] == hi {
                top.insert(xy);
            } else {
                return Err(fail("nominal convex cell is not a Cartesian prism"));
            }
        }
        if bottom != top {
            return Err(fail("nominal cell profile varies with height"));
        }
        profiles.push(planar::hull(bottom.into_iter().collect()));
    }
    let rings = planar::polygon_union(&profiles)?;
    planar::polygon_forest(&rings)?;
    Ok(Data {
        axis,
        interval: [lo, hi],
        original: None,
        rings,
    })
}
fn sum_profile(c: &Certificate, axis: usize, interval: &[Q; 2]) -> Result<Vec<Ring>> {
    let [x, y] = axes(axis);
    let p = c
        .vertices
        .iter()
        .map(|p| {
            p.clone()
                .map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap()))
        })
        .collect::<Vec<_>>();
    let mut rings = Vec::new();
    for t in &c.triangles {
        let a = t.map(|i| p[i as usize].clone());
        let n = cross(&sub(&a[1], &a[0]), &sub(&a[2], &a[0]));
        if !n[axis].is_zero() {
            if a.iter().all(|p| p[axis] == interval[1]) && n[axis] > Q::zero() {
                rings.push(a.iter().map(|p| [p[x].clone(), p[y].clone()]).collect());
            } else if !(a.iter().all(|p| p[axis] == interval[0]) && n[axis] < Q::zero()) {
                return Err(fail("nominal sum is not the claimed height product"));
            }
        }
    }
    planar::triangle_boundary(&rings)
}
fn convex_profile(d: &Data) -> Result<()> {
    if let Some(w) = &d.original {
        for wire in w {
            planar::convex_wire(wire)?;
        }
    } else {
        if planar::groups(&d.rings)?.len() != 1 {
            return Err(fail("prismatic convex source has disconnected material"));
        }
        for r in &d.rings {
            if planar::hull(r.clone()).len() < 3
                || planar::area(&planar::hull(r.clone())) != planar::area(r).abs()
            {
                return Err(fail("prismatic boundary is not convex"));
            }
        }
    }
    Ok(())
}
fn bounds(d: &Data, wire: usize, depth: u8) -> Result<(Ring, Ring)> {
    if let Some(w) = &d.original {
        planar::convex_bounds(&w[wire], depth)
    } else {
        let r = planar::hull(d.rings[wire].clone());
        Ok((r.clone(), r))
    }
}
fn try_sum(a: &Data, b: &Data, c: &Certificate, holed: usize) -> Result<SumReport> {
    if a.axis != b.axis {
        return Err(fail("prismatic source axes differ"));
    }
    convex_profile(a)?;
    convex_profile(b)?;
    let kernel_wires = b.original.as_ref().map_or(b.rings.len(), Vec::len);
    if kernel_wires != 1 {
        return Err(fail("prismatic sum kernel has holes"));
    }
    let wires = a.original.as_ref().map_or(a.rings.len(), Vec::len);
    let mut holes = Vec::new();
    for hole in 1..wires {
        let mut proof = None;
        for depth in 0..=6 {
            let (hi, ho) = bounds(a, hole, depth)?;
            let (ki, ko) = bounds(b, 0, depth)?;
            let reflect = |r: Ring| r.into_iter().map(|p| p.map(|q| -q)).collect::<Ring>();
            let inner = planar::erosion(&hi, &reflect(ko))?;
            let outer = planar::erosion(&ho, &reflect(ki))?;
            let open = inner.len() >= 3 && planar::area(&inner) > Q::zero();
            let closed = outer.len() < 3 || planar::area(&outer).is_zero();
            if open || closed {
                let encode = |r: Ring| r.iter().map(|p| p.clone().map(|q| ratio(&q))).collect();
                proof = Some(HoleReport {
                    hole,
                    open,
                    depth,
                    inner_core: encode(inner),
                    outer_core: encode(outer),
                });
                break;
            }
        }
        holes.push(
            proof.ok_or_else(|| {
                fail("prismatic hole erosion has an unproved critical transition")
            })?,
        );
    }
    let interval = [
        &a.interval[0] + &b.interval[0],
        &a.interval[1] + &b.interval[1],
    ];
    let n = holes.iter().filter(|h| h.open).count();
    if planar::groups(&sum_profile(c, a.axis, &interval)?)? != vec![n] {
        return Err(fail(
            "nominal sum changes the checked curved-source hole topology",
        ));
    }
    Ok(SumReport {
        axis: a.axis,
        interval: interval.map(|q| ratio(&q)),
        holed_operand: holed,
        holes,
        surviving_holes: n,
    })
}
pub(super) fn sum(
    a: &Source,
    b: &Source,
    an: &[Poly],
    bn: &[Poly],
    ad: Option<&Data>,
    bd: Option<&Data>,
    c: &Certificate,
) -> Option<SumReport> {
    for axis in [2, 0, 1] {
        let get = |s: &Source, n: &[Poly], d: Option<&Data>| match s {
            Source::Mesh(_) => projected_product(n, axis).ok(),
            Source::Brep(_) => d.filter(|d| d.axis == axis).cloned(),
        };
        let (Some(a), Some(b)) = (get(a, an, ad), get(b, bn, bd)) else {
            continue;
        };
        match try_sum(&a, &b, c, 0) {
            Ok(r) => return Some(r),
            Err(e) => {
                #[cfg(test)]
                eprintln!("Prismatic sum axis {axis} unproved: {}", e.message);
                #[cfg(not(test))]
                let _ = e;
            }
        }
        if let Ok(r) = try_sum(&b, &a, c, 1) {
            return Some(r);
        }
    }
    None
}

impl Serialize for ShellReport {
    fn to_value(&self) -> Value {
        json!({"shell":self.shell,"interval":self.interval,"curves":self.curves,"coefficients":self.coefficients})
    }
}
impl<'de> Deserialize<'de> for ShellReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(&v, &["shell", "interval", "curves", "coefficients"])?;
        Ok(Self {
            shell: value_codec::from_value(v["shell"].clone())?,
            interval: value_codec::from_value(v["interval"].clone())?,
            curves: value_codec::from_value(v["curves"].clone())?,
            coefficients: value_codec::from_value(v["coefficients"].clone())?,
        })
    }
}
fn schema(v: &Value, keys: &[&str]) -> value_codec::Result<()> {
    if v.as_object()
        .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
    {
        return Err(error("Prismatic topology schema mismatch"));
    }
    Ok(())
}
impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({"kind":"signed-prismatic-ambient-isotopy","axis":self.axis,"levels":self.levels,"shells":self.shells,"curveParents":self.parents,"nominalProfileVertices":self.nominal_profile_vertices,"preservesComplement":true})
    }
}
impl<'de> Deserialize<'de> for Report {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(
            &v,
            &[
                "kind",
                "axis",
                "levels",
                "shells",
                "curveParents",
                "nominalProfileVertices",
                "preservesComplement",
            ],
        )?;
        if v["kind"].as_str() != Some("signed-prismatic-ambient-isotopy")
            || v["preservesComplement"].as_bool() != Some(true)
        {
            return Err(error("Prismatic topology claim mismatch"));
        }
        Ok(Self {
            axis: value_codec::from_value(v["axis"].clone())?,
            levels: value_codec::from_value(v["levels"].clone())?,
            shells: value_codec::from_value(v["shells"].clone())?,
            parents: value_codec::from_value(v["curveParents"].clone())?,
            nominal_profile_vertices: value_codec::from_value(v["nominalProfileVertices"].clone())?,
        })
    }
}
impl Serialize for HoleReport {
    fn to_value(&self) -> Value {
        json!({"hole":self.hole,"open":self.open,"depth":self.depth,"innerCore":self.inner_core,"outerCore":self.outer_core})
    }
}
impl<'de> Deserialize<'de> for HoleReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(&v, &["hole", "open", "depth", "innerCore", "outerCore"])?;
        Ok(Self {
            hole: value_codec::from_value(v["hole"].clone())?,
            open: value_codec::from_value(v["open"].clone())?,
            depth: value_codec::from_value(v["depth"].clone())?,
            inner_core: value_codec::from_value(v["innerCore"].clone())?,
            outer_core: value_codec::from_value(v["outerCore"].clone())?,
        })
    }
}
impl Serialize for SumReport {
    fn to_value(&self) -> Value {
        json!({"kind":"convex-hole-prismatic-minkowski-homeomorphism","axis":self.axis,"interval":self.interval,"holedOperand":self.holed_operand,"holes":self.holes,"survivingHoles":self.surviving_holes})
    }
}
impl<'de> Deserialize<'de> for SumReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(
            &v,
            &[
                "kind",
                "axis",
                "interval",
                "holedOperand",
                "holes",
                "survivingHoles",
            ],
        )?;
        if v["kind"].as_str() != Some("convex-hole-prismatic-minkowski-homeomorphism") {
            return Err(error("Prismatic sum claim mismatch"));
        }
        Ok(Self {
            axis: value_codec::from_value(v["axis"].clone())?,
            interval: value_codec::from_value(v["interval"].clone())?,
            holed_operand: value_codec::from_value(v["holedOperand"].clone())?,
            holes: value_codec::from_value(v["holes"].clone())?,
            surviving_holes: value_codec::from_value(v["survivingHoles"].clone())?,
        })
    }
}

#[cfg(test)]
#[path = "prismatic/tests/tests_integration.rs"]
mod tests;
