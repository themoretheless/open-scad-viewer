//! Principal axes of area and thin-walled torsion constants, formulas only.
//!
//! Principal moments come from the same contour integrals as
//! `analyze_section` (area, centroid, Ixx/Iyy/Ixy), diagonalized analytically.
//! The inertia ellipse semi-axes are the radii of gyration √(I/A). Torsion
//! constants are thin-walled textbook formulas: open sections Σ b·t³/3 over
//! strips, single closed cells Bredt–Batho 4A²/∮ ds/t. `thin_walled_open`
//! extends the strip model to the full Vlasov set — shear center, warping
//! constant Cw, and shear areas — for open centerline trees (I, C, L, Z
//! sections); closed loops stay with `torsion_constant_closed`. Multi-cell
//! sections are out of scope. Not a certified section-table lookup.
use crate::{Error, LayerSection, Result};

/// Maximum strips or centerline edges in one torsion request.
pub const MAX_STRIPS: usize = 4096;

fn invalid(message: &str) -> Error {
    Error::new("SECTION_INVALID_INPUT", message)
}
fn numeric() -> Error {
    Error::new(
        "SECTION_NUMERIC_RANGE",
        "Section calculation exceeds finite numeric range",
    )
}

/// Principal axes of one already-cut section, all quantities centroidal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrincipalAxesReport {
    pub area_mm2: f64,
    pub centroid: [f64; 2],
    /// Direction of principal axis 1 (larger moment), from +x, in (−π/2, π/2].
    pub angle_rad: f64,
    /// Larger and smaller principal second moments of area.
    pub i1_mm4: f64,
    pub i2_mm4: f64,
    /// Radii of gyration √(I/A): the inertia ellipse semi-axes.
    pub r1_mm: f64,
    pub r2_mm: f64,
    /// Principal section moduli against the contour vertices.
    pub w1_mm3: f64,
    pub w2_mm3: f64,
    /// Every axis is principal (Ixx == Iyy, Ixy == 0); angle is reported as 0.
    pub isotropic: bool,
}

/// Principal moments and axes from the same contour integrals as
/// `analyze_section`: the centroidal tensor [[Ixx, −Ixy], [−Ixy, Iyy]]
/// diagonalized in closed form. Holes subtract by opposite winding.
pub fn principal_axes(section: &LayerSection) -> Result<PrincipalAxesReport> {
    crate::require_section(section)?;
    if section.contours.is_empty() {
        return Err(invalid("Section has no contours"));
    }
    let mut twice_area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut ixx_o = 0.0;
    let mut iyy_o = 0.0;
    let mut ixy_o = 0.0;
    for ring in &section.contours {
        let (da, dcx, dcy, dixx, diyy, dixy) = crate::polygon_integrals(ring);
        twice_area += da;
        cx += dcx;
        cy += dcy;
        ixx_o += dixx;
        iyy_o += diyy;
        ixy_o += dixy;
    }
    let area = twice_area * 0.5;
    if !area.is_finite() || area.abs() < 1e-12 {
        return Err(invalid("Section area is degenerate"));
    }
    let centroid = [cx / (6.0 * area), cy / (6.0 * area)];
    // Normalize winding: with every ring reversed the raw moments negate.
    let s = area.signum();
    let ixx = s * (ixx_o / 12.0 - area * centroid[1] * centroid[1]);
    let iyy = s * (iyy_o / 12.0 - area * centroid[0] * centroid[0]);
    let ixy = s * (ixy_o / 24.0 - area * centroid[0] * centroid[1]);
    if ![ixx, iyy, ixy].iter().all(|v| v.is_finite()) {
        return Err(numeric());
    }
    let area_abs = area.abs();
    let avg = (ixx + iyy) * 0.5;
    let dev = ((ixx - iyy) * 0.5, ixy);
    let d = dev.0.hypot(dev.1);
    let i1 = avg + d;
    let i2 = avg - d;
    if !i1.is_finite() || !i2.is_finite() || i2 < 0. {
        return Err(numeric());
    }
    let isotropic = d <= 1e-12 * avg.abs().max(1.);
    // Eigenvector of [[Ixx, −Ixy], [−Ixy, Iyy]] for the larger eigenvalue.
    let (v1, v2) = ((ixy, ixx - i1), (iyy - i1, ixy));
    let raw = if v1.0.hypot(v1.1) >= v2.0.hypot(v2.1) {
        v1
    } else {
        v2
    };
    let norm = raw.0.hypot(raw.1);
    let axis1 = if isotropic || norm == 0. {
        [1., 0.]
    } else {
        let mut a = [raw.0 / norm, raw.1 / norm];
        if a[0] < 0. || (a[0] == 0. && a[1] < 0.) {
            a = [-a[0], -a[1]];
        }
        a
    };
    let axis2 = [-axis1[1], axis1[0]];
    let angle_rad = if isotropic {
        0.
    } else {
        axis1[1].atan2(axis1[0])
    };
    // Principal section moduli against the contour vertices.
    let mut max_xi = 0.0_f64;
    let mut max_eta = 0.0_f64;
    for ring in &section.contours {
        for p in ring {
            let (dx, dy) = (p[0] - centroid[0], p[1] - centroid[1]);
            max_xi = max_xi.max((dx * axis1[0] + dy * axis1[1]).abs());
            max_eta = max_eta.max((dx * axis2[0] + dy * axis2[1]).abs());
        }
    }
    if !max_xi.is_finite() || !max_eta.is_finite() {
        return Err(numeric());
    }
    Ok(PrincipalAxesReport {
        area_mm2: area_abs,
        centroid,
        angle_rad,
        i1_mm4: i1,
        i2_mm4: i2,
        r1_mm: (i1 / area_abs).sqrt(),
        r2_mm: (i2 / area_abs).sqrt(),
        w1_mm3: if max_eta > 0. { i1 / max_eta } else { 0. },
        w2_mm3: if max_xi > 0. { i2 / max_xi } else { 0. },
        isotropic,
    })
}

/// One straight thin-walled strip: centerline endpoints and uniform thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strip {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub thickness_mm: f64,
}

/// Open-section torsion constant J = Σ b·t³/3 over strips (I-beams, channels,
/// angles). Junction concentration factors are not applied.
pub fn torsion_constant_open(strips: &[Strip]) -> Result<f64> {
    if strips.is_empty() || strips.len() > MAX_STRIPS {
        return Err(invalid("Provide 1-4096 open strips"));
    }
    let mut j = 0.;
    for strip in strips {
        let dx = strip.b[0] - strip.a[0];
        let dy = strip.b[1] - strip.a[1];
        let length = dx.hypot(dy);
        if !length.is_finite() || length <= 0. || ![strip.a, strip.b].iter().flatten().all(|v| v.is_finite()) {
            return Err(invalid("Strip endpoints must be finite and distinct"));
        }
        if !strip.thickness_mm.is_finite() || strip.thickness_mm <= 0. {
            return Err(invalid("Strip thickness must be finite and positive"));
        }
        j += length * strip.thickness_mm.powi(3) / 3.;
    }
    if !j.is_finite() {
        return Err(numeric());
    }
    Ok(j)
}

/// Single-cell closed-section torsion constant (Bredt–Batho):
/// J = 4·A²/∮ ds/t over the wall centerline, one thickness per edge.
/// Multi-cell sections are out of scope.
pub fn torsion_constant_closed(centerline: &[[f64; 2]], thickness_mm: &[f64]) -> Result<f64> {
    if centerline.len() < 3
        || centerline.len() > MAX_STRIPS
        || thickness_mm.len() != centerline.len()
    {
        return Err(invalid(
            "Closed cell needs 3-4096 centerline points and one thickness per edge",
        ));
    }
    let mut twice_area = 0.;
    let mut perimeter_over_t = 0.;
    for i in 0..centerline.len() {
        let a = centerline[i];
        let b = centerline[(i + 1) % centerline.len()];
        let t = thickness_mm[i];
        if ![a, b].iter().flatten().all(|v| v.is_finite()) {
            return Err(invalid("Centerline coordinates must be finite"));
        }
        if !t.is_finite() || t <= 0. {
            return Err(invalid("Wall thickness must be finite and positive"));
        }
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        if length <= 0. {
            return Err(invalid("Centerline edges must have positive length"));
        }
        twice_area += a[0] * b[1] - b[0] * a[1];
        perimeter_over_t += length / t;
    }
    let area = twice_area.abs() * 0.5;
    if !area.is_finite() || area < 1e-12 {
        return Err(invalid("Centerline area is degenerate"));
    }
    let j = 4. * area * area / perimeter_over_t;
    if !j.is_finite() {
        return Err(numeric());
    }
    Ok(j)
}

/// Warping and shear properties of an open thin-walled section, centroidal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThinWalledOpenReport {
    pub area_mm2: f64,
    pub centroid: [f64; 2],
    /// Direction of principal axis 1 (larger moment), from +x, in (−π/2, π/2].
    pub angle_rad: f64,
    pub i1_mm4: f64,
    pub i2_mm4: f64,
    /// Saint-Venant torsion constant Σ b·t³/3.
    pub j_mm4: f64,
    /// Warping constant ∫ ω̄² t ds over the centerline.
    pub cw_mm6: f64,
    /// Shear center in the input coordinates.
    pub shear_center: [f64; 2],
    /// Shear area for a shear force along principal axis 1 (V²/∫q²/t ds).
    pub shear_area_1_mm2: f64,
    /// Shear area for a shear force along principal axis 2.
    pub shear_area_2_mm2: f64,
}

/// One directed centerline edge during the shear-flow pass.
struct DirectedEdge {
    /// Node ids, `a` on the leaf side.
    a: usize,
    b: usize,
    /// q(s) = q0 + q1·s + q2·s² for unit shear along v (s from node a).
    qv: [f64; 3],
    /// Same for unit shear along u.
    qu: [f64; 3],
    /// Twice the signed triangle area (u_a·Δv − v_a·Δu); torque lever × L.
    lever_l: f64,
    length: f64,
    thickness: f64,
}

/// Full Vlasov property set of an open thin-walled section from its wall
/// centerline graph: strips may branch (I/H sections) and endpoints lying on
/// another strip's interior split it. The graph must be one connected tree —
/// closed loops are refused (`torsion_constant_closed` covers single cells).
/// Shear flows satisfy q = 0 at free ends and conservation at junctions, in
/// the centroidal principal frame, so shear along one principal axis decouples
/// from the other. The sectorial coordinate ω runs about the shear center from
/// an arbitrary free end and is mean-normalized; Cw = ∫ ω̄² t ds. Strip
/// endpoints must coincide to 1e-9 of the section span. Not a certified
/// section-table lookup.
pub fn thin_walled_open(strips: &[Strip]) -> Result<ThinWalledOpenReport> {
    if strips.is_empty() || strips.len() > MAX_STRIPS {
        return Err(invalid("Provide 1-4096 thin-walled strips"));
    }
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for strip in strips {
        let dx = strip.b[0] - strip.a[0];
        let dy = strip.b[1] - strip.a[1];
        if !dx.hypot(dy).is_finite() || dx.hypot(dy) <= 0. {
            return Err(invalid("Strip endpoints must be finite and distinct"));
        }
        if !strip.thickness_mm.is_finite() || strip.thickness_mm <= 0. {
            return Err(invalid("Strip thickness must be finite and positive"));
        }
        for k in 0..2 {
            lo[k] = lo[k].min(strip.a[k].min(strip.b[k]));
            hi[k] = hi[k].max(strip.a[k].max(strip.b[k]));
        }
    }
    let span = (hi[0] - lo[0]).hypot(hi[1] - lo[1]);
    if !span.is_finite() || span <= 0. {
        return Err(invalid("Strip coordinates must be finite"));
    }
    let tol = 1e-9 * span;
    // Cluster coincident endpoints onto shared node ids (grid rounding).
    let mut nodes: Vec<[f64; 2]> = Vec::new();
    let mut ids: std::collections::HashMap<(i64, i64), usize> = Default::default();
    fn node_of(
        nodes: &mut Vec<[f64; 2]>,
        ids: &mut std::collections::HashMap<(i64, i64), usize>,
        tol: f64,
        p: [f64; 2],
    ) -> usize {
        let key = ((p[0] / tol).round() as i64, (p[1] / tol).round() as i64);
        *ids.entry(key).or_insert_with(|| {
            nodes.push(p);
            nodes.len() - 1
        })
    }
    let mut strips_n: Vec<(usize, usize, f64, [f64; 2], [f64; 2])> = Vec::new();
    for strip in strips {
        let (a, b) = (
            node_of(&mut nodes, &mut ids, tol, strip.a),
            node_of(&mut nodes, &mut ids, tol, strip.b),
        );
        if a == b {
            return Err(invalid("Strip endpoints must be distinct"));
        }
        strips_n.push((a, b, strip.thickness_mm, strip.a, strip.b));
    }
    // Split strips at nodes lying in their interior (T junctions); sub-edges
    // keep the junction's node id so the graph stays connected exactly.
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    for &(a, b, t, pa, pb) in &strips_n {
        let d = [pb[0] - pa[0], pb[1] - pa[1]];
        let l2 = d[0] * d[0] + d[1] * d[1];
        let mut params: Vec<(f64, usize)> = vec![(0., a), (1., b)];
        for (id, &p) in nodes.iter().enumerate() {
            if id == a || id == b {
                continue;
            }
            let cross = d[0] * (p[1] - pa[1]) - d[1] * (p[0] - pa[0]);
            if cross.abs() > tol * l2.sqrt() {
                continue;
            }
            let s = ((p[0] - pa[0]) * d[0] + (p[1] - pa[1]) * d[1]) / l2;
            if s > 1e-12 && s < 1. - 1e-12 {
                params.push((s, id));
            }
        }
        params.sort_by(|x, y| x.0.total_cmp(&y.0));
        params.dedup_by(|x, y| (x.0 - y.0).abs() < 1e-9);
        for pair in params.windows(2) {
            edges.push((pair[0].1, pair[1].1, t));
        }
    }
    // The centerline graph must be a single tree: no loops, no islands.
    let mut parent: Vec<usize> = (0..nodes.len()).collect();
    fn root(parent: &mut Vec<usize>, mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for &(a, b, _) in &edges {
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        if ra == rb {
            return Err(invalid(
                "Centerline graph has a closed loop; closed cells use torsion_constant_closed",
            ));
        }
        parent[ra] = rb;
    }
    let first = root(&mut parent, edges[0].0);
    if (0..nodes.len()).any(|i| root(&mut parent, i) != first) {
        return Err(invalid("Centerline graph is disconnected"));
    }
    // Section integrals over the walls (global frame).
    let mut area = 0.;
    let (mut sx, mut sy) = (0., 0.);
    let (mut ix2, mut iy2, mut ixy) = (0., 0., 0.);
    let mut j = 0.;
    for &(a, b, t) in &edges {
        let (pa, pb) = (nodes[a], nodes[b]);
        let l = (pb[0] - pa[0]).hypot(pb[1] - pa[1]);
        let w = t * l;
        area += w;
        sx += w * (pa[0] + pb[0]) / 2.;
        sy += w * (pa[1] + pb[1]) / 2.;
        ix2 += w * (pa[1] * pa[1] + pa[1] * pb[1] + pb[1] * pb[1]) / 3.;
        iy2 += w * (pa[0] * pa[0] + pa[0] * pb[0] + pb[0] * pb[0]) / 3.;
        ixy += w
            * (2. * pa[0] * pa[1] + pa[0] * pb[1] + pb[0] * pa[1] + 2. * pb[0] * pb[1])
            / 6.;
        j += l * t.powi(3) / 3.;
    }
    if !area.is_finite() || area <= 0. {
        return Err(numeric());
    }
    let centroid = [sx / area, sy / area];
    let ixx = ix2 - area * centroid[1].powi(2);
    let iyy = iy2 - area * centroid[0].powi(2);
    let ixy = ixy - area * centroid[0] * centroid[1];
    if ![ixx, iyy, ixy].iter().all(|v| v.is_finite()) {
        return Err(numeric());
    }
    // Principal axes, same closed-form diagonalization as `principal_axes`.
    let avg = (ixx + iyy) * 0.5;
    let d = ((ixx - iyy) * 0.5).hypot(ixy);
    let (i1, i2) = (avg + d, avg - d);
    if !i1.is_finite() || !i2.is_finite() || i2 <= 1e-12 * i1.max(1.) {
        return Err(invalid("Section is degenerate (a straight line)"));
    }
    let isotropic = d <= 1e-12 * avg.abs().max(1.);
    let (v1, v2) = ((ixy, ixx - i1), (iyy - i1, ixy));
    let raw = if v1.0.hypot(v1.1) >= v2.0.hypot(v2.1) {
        v1
    } else {
        v2
    };
    let norm = raw.0.hypot(raw.1);
    let axis1 = if isotropic || norm == 0. {
        [1., 0.]
    } else {
        let mut a = [raw.0 / norm, raw.1 / norm];
        if a[0] < 0. || (a[0] == 0. && a[1] < 0.) {
            a = [-a[0], -a[1]];
        }
        a
    };
    let axis2 = [-axis1[1], axis1[0]];
    let angle_rad = if isotropic {
        0.
    } else {
        axis1[1].atan2(axis1[0])
    };
    // Principal centroidal node coordinates (u along axis1, v along axis2).
    let uv: Vec<[f64; 2]> = nodes
        .iter()
        .map(|p| {
            let d = [p[0] - centroid[0], p[1] - centroid[1]];
            [d[0] * axis1[0] + d[1] * axis1[1], d[0] * axis2[0] + d[1] * axis2[1]]
        })
        .collect();
    // Peel the tree from the leaves, accumulating the first moments Q of the
    // consumed subtree; q(s) on each edge follows from its leaf-side end.
    let mut adjacency: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nodes.len()];
    for (id, &(a, b, _)) in edges.iter().enumerate() {
        adjacency[a].push((id, b));
        adjacency[b].push((id, a));
    }
    let mut degree: Vec<usize> = adjacency.iter().map(Vec::len).collect();
    let mut queue: std::collections::VecDeque<usize> = (0..nodes.len())
        .filter(|&i| degree[i] == 1)
        .collect();
    let mut accum_u = vec![0.; nodes.len()]; // Σ∫u t ds of the consumed subtree
    let mut accum_v = vec![0.; nodes.len()]; // Σ∫v t ds
    let mut directed: Vec<Option<DirectedEdge>> = (0..edges.len()).map(|_| None).collect();
    while let Some(leaf) = queue.pop_front() {
        if degree[leaf] != 1 {
            continue;
        }
        let (id, inward) = adjacency[leaf]
            .iter()
            .find(|&&(id, _)| directed[id].is_none())
            .copied()
            .ok_or_else(numeric)?;
        let (_, _, t) = edges[id];
        let (a, b) = (leaf, inward);
        let (ua, va) = (uv[a][0], uv[a][1]);
        let (ub, vb) = (uv[b][0], uv[b][1]);
        let (du, dv) = (ub - ua, vb - va);
        let l = du.hypot(dv);
        // q(s) = −(Q_in + ∫₀ˢ coord·t dσ)/I for each principal shear.
        let qv = [
            -accum_v[a] / i1,
            -t * va / i1,
            -t * dv / (2. * l * i1),
        ];
        let qu = [
            -accum_u[a] / i2,
            -t * ua / i2,
            -t * du / (2. * l * i2),
        ];
        accum_v[b] += accum_v[a] + t * l * (va + vb) / 2.;
        accum_u[b] += accum_u[a] + t * l * (ua + ub) / 2.;
        directed[id] = Some(DirectedEdge {
            a,
            b,
            qv,
            qu,
            lever_l: ua * dv - va * du,
            length: l,
            thickness: t,
        });
        degree[a] -= 1;
        degree[b] -= 1;
        if degree[b] == 1 {
            queue.push_back(b);
        }
    }
    if directed.iter().any(Option::is_none) {
        return Err(numeric());
    }
    // Torque of the unit-shear flows about the centroid and the flow energy.
    let (mut torque_v, mut torque_u) = (0., 0.);
    let (mut energy_v, mut energy_u) = (0., 0.);
    for edge in directed.iter().flatten() {
        let l = edge.length;
        let int_q = |q: [f64; 3]| q[0] * l + q[1] * l * l / 2. + q[2] * l.powi(3) / 3.;
        let int_q2 = |q: [f64; 3]| {
            q[0].powi(2) * l + q[0] * q[1] * l * l
                + (q[1].powi(2) / 3. + 2. * q[0] * q[2] / 3.) * l.powi(3)
                + q[1] * q[2] * l.powi(4) / 2.
                + q[2].powi(2) * l.powi(5) / 5.
        };
        torque_v += edge.lever_l / l * int_q(edge.qv);
        torque_u += edge.lever_l / l * int_q(edge.qu);
        energy_v += int_q2(edge.qv) / edge.thickness;
        energy_u += int_q2(edge.qu) / edge.thickness;
    }
    if ![torque_v, torque_u, energy_v, energy_u]
        .iter()
        .all(|v| v.is_finite())
        || energy_v <= 0.
        || energy_u <= 0.
    {
        return Err(numeric());
    }
    // Flow torque balances the applied unit shear at the shear center.
    let u_sc = torque_v;
    let v_sc = -torque_u;
    // Sectorial coordinate about the shear center, mean-normalized; the graph
    // is a tree, so propagation from one free end is path-independent.
    let mut omega = vec![f64::NAN; nodes.len()];
    omega[directed[0].as_ref().ok_or_else(numeric)?.a] = 0.;
    let mut stack = vec![directed[0].as_ref().ok_or_else(numeric)?.a];
    while let Some(n) = stack.pop() {
        for &(id, other) in &adjacency[n] {
            if omega[other].is_finite() {
                continue;
            }
            let edge = directed[id].as_ref().ok_or_else(numeric)?;
            let (ua, va) = (uv[edge.a][0], uv[edge.a][1]);
            let (ub, vb) = (uv[edge.b][0], uv[edge.b][1]);
            // dω from a to b: ((u−u_sc)·Δv − (v−v_sc)·Δu), constant per edge.
            let d_omega = (ua - u_sc) * (vb - va) - (va - v_sc) * (ub - ua);
            omega[other] = if edge.a == n {
                omega[n] + d_omega
            } else {
                omega[n] - d_omega
            };
            stack.push(other);
        }
    }
    if omega.iter().any(|o| !o.is_finite()) {
        return Err(numeric());
    }
    let mut mean = 0.;
    let mut cw = 0.;
    for edge in directed.iter().flatten() {
        let (wa, wb) = (omega[edge.a], omega[edge.b]);
        let w = edge.thickness * edge.length;
        mean += w * (wa + wb) / 2.;
    }
    mean /= area;
    for edge in directed.iter().flatten() {
        let wa = omega[edge.a] - mean;
        let wb = omega[edge.b] - mean;
        cw += edge.thickness * edge.length * (wa * wa + wa * wb + wb * wb) / 3.;
    }
    if !cw.is_finite() || cw < 0. || !mean.is_finite() {
        return Err(numeric());
    }
    let shear_center = [
        centroid[0] + u_sc * axis1[0] + v_sc * axis2[0],
        centroid[1] + u_sc * axis1[1] + v_sc * axis2[1],
    ];
    Ok(ThinWalledOpenReport {
        area_mm2: area,
        centroid,
        angle_rad,
        i1_mm4: i1,
        i2_mm4: i2,
        j_mm4: j,
        cw_mm6: cw,
        shear_center,
        shear_area_1_mm2: 1. / energy_u,
        shear_area_2_mm2: 1. / energy_v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(contours: Vec<Vec<[f64; 2]>>) -> LayerSection {
        LayerSection {
            z_mm: 0.,
            contours,
        }
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-9 * b.abs().max(1.), "{a} != {b}");
    }

    #[test]
    fn rectangle_principal_axes_match_handbook() {
        // 40 x 20 rectangle centered at the origin, CCW. The maximum moment
        // is about the vertical axis (material spreads along x).
        let r = principal_axes(&section(vec![vec![
            [-20., -10.],
            [20., -10.],
            [20., 10.],
            [-20., 10.],
        ]]))
        .unwrap();
        close(r.i1_mm4, 20. * 40f64.powi(3) / 12.);
        close(r.i2_mm4, 40. * 20f64.powi(3) / 12.);
        close(r.angle_rad, std::f64::consts::FRAC_PI_2);
        close(r.area_mm2, 800.);
        close(r.r1_mm, (r.i1_mm4 / 800.).sqrt());
        close(r.w1_mm3, r.i1_mm4 / 20.);
        close(r.w2_mm3, r.i2_mm4 / 10.);
        assert!(!r.isotropic);
        assert_eq!(r.centroid, [0., 0.]);
    }

    #[test]
    fn rotated_rectangle_keeps_moments_and_reports_rotation() {
        let angle = std::f64::consts::FRAC_PI_4;
        let (c, s) = (angle.cos(), angle.sin());
        let rect = [[-20., -10.], [20., -10.], [20., 10.], [-20., 10.]]
            .map(|[x, y]| [x * c - y * s, x * s + y * c]);
        let r = principal_axes(&section(vec![rect.to_vec()])).unwrap();
        close(r.i1_mm4, 20. * 40f64.powi(3) / 12.);
        close(r.i2_mm4, 40. * 20f64.powi(3) / 12.);
        // The maximum-moment axis is perpendicular to the long direction.
        close(r.angle_rad, -angle);
    }

    #[test]
    fn symmetric_angle_section_points_at_45_degrees() {
        // Equal-leg L, symmetric about y = x.
        let l = vec![vec![
            [0., 0.],
            [10., 0.],
            [10., 2.],
            [2., 2.],
            [2., 10.],
            [0., 10.],
        ]];
        let r = principal_axes(&section(l.clone())).unwrap();
        close(r.angle_rad, std::f64::consts::FRAC_PI_4);
        // Invariants against the axes-aligned integrals.
        let base = crate::analyze_section(
            &section(l),
            &crate::LoadCase {
                moment_x_nmm: 0.,
                moment_y_nmm: 0.,
                axial_n: 0.,
                allowable_mpa: 1.,
            },
        )
        .unwrap();
        close(r.i1_mm4 + r.i2_mm4, base.ixx_mm4 + base.iyy_mm4);
        close(
            r.i1_mm4 * r.i2_mm4,
            base.ixx_mm4 * base.iyy_mm4 - base.ixy_mm4.powi(2),
        );
        assert!(r.i1_mm4 > r.i2_mm4);
    }

    #[test]
    fn square_is_isotropic_and_tube_hole_subtracts() {
        let square = principal_axes(&section(vec![vec![
            [-5., -5.],
            [5., -5.],
            [5., 5.],
            [-5., 5.],
        ]]))
        .unwrap();
        assert!(square.isotropic);
        close(square.i1_mm4, 10. * 10f64.powi(3) / 12.);
        close(square.i2_mm4, square.i1_mm4);
        // Tube 100 x 50 with a 90 x 40 hole (opposite winding).
        let tube = principal_axes(&section(vec![
            vec![[-50., -25.], [50., -25.], [50., 25.], [-50., 25.]],
            vec![[-45., -20.], [-45., 20.], [45., 20.], [45., -20.]],
        ]))
        .unwrap();
        close(tube.i1_mm4, (50. * 100f64.powi(3) - 40. * 90f64.powi(3)) / 12.);
        close(tube.i2_mm4, (100. * 50f64.powi(3) - 90. * 40f64.powi(3)) / 12.);
        close(tube.area_mm2, 5000. - 3600.);
        assert!(!tube.isotropic);
    }

    #[test]
    fn open_section_torsion_sums_strip_powers() {
        // I-beam approximation: two flanges 100 x 5, web 95 x 4.
        let j = torsion_constant_open(&[
            Strip {
                a: [0., 0.],
                b: [100., 0.],
                thickness_mm: 5.,
            },
            Strip {
                a: [0., 0.],
                b: [100., 0.],
                thickness_mm: 5.,
            },
            Strip {
                a: [0., 0.],
                b: [0., 95.],
                thickness_mm: 4.,
            },
        ])
        .unwrap();
        close(j, 2. * 100. * 125. / 3. + 95. * 64. / 3.);
    }

    #[test]
    fn closed_cell_torsion_matches_bredt() {
        // Rectangular tube, centerline 100 x 50, wall 5.
        let j = torsion_constant_closed(
            &[[0., 0.], [100., 0.], [100., 50.], [0., 50.]],
            &[5.; 4],
        )
        .unwrap();
        close(j, 4. * 5000. * 5000. / 60.);
        // A closed tube dwarfs the same walls opened up.
        let open = torsion_constant_open(&[Strip {
            a: [0., 0.],
            b: [300., 0.],
            thickness_mm: 5.,
        }])
        .unwrap();
        assert!(j > 100. * open);
    }

    #[test]
    fn validation_rejects_bad_geometry() {
        assert!(principal_axes(&section(vec![])).is_err());
        assert!(principal_axes(&section(vec![vec![[0., 0.], [1., 0.]]])).is_err());
        assert!(torsion_constant_open(&[]).is_err());
        assert!(
            torsion_constant_open(&[Strip {
                a: [0., 0.],
                b: [0., 0.],
                thickness_mm: 1.,
            }])
            .is_err()
        );
        assert!(
            torsion_constant_open(&[Strip {
                a: [0., 0.],
                b: [1., 0.],
                thickness_mm: -1.,
            }])
            .is_err()
        );
        assert!(torsion_constant_closed(&[[0., 0.], [1., 0.]], &[1.; 2]).is_err());
        assert!(
            torsion_constant_closed(&[[0., 0.], [1., 0.], [1., 1.]], &[1.; 2]).is_err()
        );
        // Collinear centerline: zero area.
        assert!(
            torsion_constant_closed(&[[0., 0.], [1., 0.], [2., 0.]], &[1.; 3]).is_err()
        );
    }

    fn strip(a: [f64; 2], b: [f64; 2], t: f64) -> Strip {
        Strip {
            a,
            b,
            thickness_mm: t,
        }
    }

    #[test]
    fn thin_walled_i_beam_matches_closed_forms() {
        // Web 100 x 4 between flange centerlines, flanges 100 x 5; the web
        // endpoints split the flange strips at mid-span (T junctions).
        let r = thin_walled_open(&[
            strip([0., -50.], [0., 50.], 4.),
            strip([-50., -50.], [50., -50.], 5.),
            strip([-50., 50.], [50., 50.], 5.),
        ])
        .unwrap();
        close(r.area_mm2, 1400.);
        close(r.centroid[0], 0.);
        close(r.centroid[1], 0.);
        close(r.i1_mm4, 4. * 100f64.powi(3) / 12. + 2. * 5. * 100. * 2500.);
        close(r.i2_mm4, 2. * 5. * 100f64.powi(3) / 12.);
        close(r.j_mm4, (2. * 100. * 125. + 100. * 64.) / 3.);
        // Doubly symmetric: shear center coincides with the centroid, and
        // Cw = Iy·h²/4 with h the flange-centerline distance.
        assert!(r.shear_center[0].abs() < 1e-6);
        assert!(r.shear_center[1].abs() < 1e-6);
        close(r.cw_mm6, r.i2_mm4 * 100. * 100. / 4.);
        // Vertical shear rides the web; horizontal rides the flanges.
        assert!(r.shear_area_2_mm2 > 330. && r.shear_area_2_mm2 < 400.);
        assert!(r.shear_area_1_mm2 > 100. && r.shear_area_1_mm2 < 1400.);
    }

    #[test]
    fn thin_walled_channel_matches_textbook() {
        // Centerline b = 50 flanges, h = 100 web, uniform t = 5, opening +x.
        let (b, h, t) = (50., 100., 5.);
        let r = thin_walled_open(&[
            strip([b, h / 2.], [0., h / 2.], t),
            strip([0., h / 2.], [0., -h / 2.], t),
            strip([0., -h / 2.], [b, -h / 2.], t),
        ])
        .unwrap();
        close(r.area_mm2, t * (2. * b + h));
        close(r.centroid[0], 2. * b * t * (b / 2.) / r.area_mm2);
        close(r.centroid[1], 0.);
        // Shear center outside the web, opposite the opening: e = 3b²/(h+6b).
        close(r.shear_center[0], -3. * b * b / (h + 6. * b));
        assert!(r.shear_center[1].abs() < 1e-6);
        // Cw = t·b³h²/12·(2h+3b)/(h+6b).
        close(
            r.cw_mm6,
            t * b.powi(3) * h * h / 12. * (2. * h + 3. * b) / (h + 6. * b),
        );
        close(r.j_mm4, (2. * b + h) * t.powi(3) / 3.);
    }

    #[test]
    fn thin_walled_angle_shear_center_at_corner() {
        let r = thin_walled_open(&[
            strip([0., 50.], [0., 0.], 5.),
            strip([0., 0.], [50., 0.], 5.),
        ])
        .unwrap();
        // Both legs run through the corner: the shear center sits there and
        // the sectorial coordinate vanishes (no warping stiffness).
        assert!((r.shear_center[0]).abs() < 1e-6, "{}", r.shear_center[0]);
        assert!((r.shear_center[1]).abs() < 1e-6);
        assert!(r.cw_mm6.abs() < 1e-3, "{}", r.cw_mm6);
        close(r.centroid[0], 12.5);
        close(r.centroid[1], 12.5);
        close(r.angle_rad, std::f64::consts::FRAC_PI_4);
    }

    #[test]
    fn thin_walled_validates_graph_topology() {
        assert!(thin_walled_open(&[]).is_err());
        assert!(thin_walled_open(&[strip([0., 0.], [0., 0.], 1.)]).is_err());
        assert!(thin_walled_open(&[strip([0., 0.], [1., 0.], -1.)]).is_err());
        // A single straight strip has zero lateral inertia.
        assert!(thin_walled_open(&[strip([0., 0.], [100., 0.], 5.)]).is_err());
        // Closed loop: square of strips.
        assert!(thin_walled_open(&[
            strip([0., 0.], [100., 0.], 5.),
            strip([100., 0.], [100., 100.], 5.),
            strip([100., 100.], [0., 100.], 5.),
            strip([0., 100.], [0., 0.], 5.),
        ])
        .is_err());
        // Two disjoint strips.
        assert!(thin_walled_open(&[
            strip([0., 0.], [0., 100.], 5.),
            strip([10., 0.], [60., 0.], 5.),
        ])
        .is_err());
    }
}
