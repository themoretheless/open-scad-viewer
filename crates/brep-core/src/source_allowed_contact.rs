//! Sufficient exact shared-line contact proof for qualified planar source faces.
//! Failure is unresolved; no tolerance weld or topology-only pair exclusion.
use crate::{
    source_boundary_fragment::Endpoint,
    source_contour_proposal::SourceRegion,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    edges: Vec<SharedEdge>,
    plane: [[f64; 3]; 3],
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub spans: usize,
    pub reason: &'static str,
}
fn orient(
    points: &[[f64; 3]],
    axes: Option<[usize; 2]>,
    used: &mut u64,
    max_work: u64,
) -> Result<Option<Sign>> {
    if *used == max_work {
        return Ok(None);
    }
    let arena = SourceArena::authored(
        "source-allowed-contact",
        1,
        points
            .iter()
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("BREP_SOURCE_CONTACT", "Invalid exact contact source"))?;
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: (max_work - *used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let point = |i: usize| std::array::from_fn(|k| arena.leaf(3 * i + k).unwrap());
    let decision = if let Some(axes) = axes {
        let p = |i: usize| axes.map(|k| arena.leaf(3 * i + k).unwrap());
        cad_predicates::orient2d(&mut ctx, p(0), p(1), p(2))
    } else {
        cad_predicates::orient3d(&mut ctx, point(0), point(1), point(2), point(3))
    }
    .map_err(|_| Error::new("BREP_SOURCE_CONTACT", "Invalid exact contact predicate"))?;
    *used += decision.work_used;
    Ok(match decision.outcome {
        Outcome::Sign(s) => Some(s),
        _ => None,
    })
}
fn independent(a: [f64; 3], b: [f64; 3], c: [f64; 3], used: &mut u64, budget: u64) -> Result<bool> {
    for axes in [[0, 1], [0, 2], [1, 2]] {
        match orient(&[a, b, c], Some(axes), used, budget)? {
            Some(Sign::Zero) => {}
            Some(_) => return Ok(true),
            None => return Ok(false),
        }
    }
    Ok(false)
}
fn plane(s: &Surface, used: &mut u64, budget: u64) -> Result<Option<[[f64; 3]; 3]>> {
    let poles = s
        .control_points
        .iter()
        .flatten()
        .map(|p| [p[0], p[1], p[2]])
        .collect::<Vec<_>>();
    let a = poles[0];
    let Some(&b) = poles.iter().find(|&&p| p != a) else {
        return Ok(None);
    };
    for &c in &poles {
        if !independent(a, b, c, used, budget)? {
            continue;
        }
        for &p in &poles {
            if orient(&[a, b, c, p], None, used, budget)? != Some(Sign::Zero) {
                return Ok(None);
            }
        }
        return Ok(Some([a, b, c]));
    }
    Ok(None)
}
fn complete(shell: &Shell, indices: &[usize], face: usize) -> bool {
    let mut addresses = indices
        .iter()
        .filter_map(|&i| shell.uses()[i].iter().find(|a| a.face == face).copied())
        .collect::<Vec<Address>>();
    addresses.sort_by_key(|a| (a.wire, a.edge));
    if addresses.is_empty() {
        return false;
    }
    let first = addresses[0];
    let get = |a: Address| &shell.faces()[a.face][a.wire].edges()[a.edge];
    let source = get(first);
    let d = source.curve().domain();
    let ends = if source.reversed() { [d[1], d[0]] } else { d };
    if !matches!(source.endpoints()[0],Endpoint::Parameter(t) if t.to_bits()==ends[0].to_bits()) {
        return false;
    }
    for pair in addresses.windows(2) {
        if pair[1].wire != first.wire
            || pair[1].edge != pair[0].edge + 1
            || get(pair[1]).curve() != source.curve()
            || get(pair[1]).reversed() != source.reversed()
            || !get(pair[0]).joins(get(pair[1]))
        {
            return false;
        }
    }
    matches!(get(*addresses.last().unwrap()).endpoints()[1],Endpoint::Parameter(t) if t.to_bits()==ends[1].to_bits())
}
fn linear(c: &Curve) -> bool {
    let d = c.domain();
    c.degree == 1
        && c.control_points.len() == 2
        && !c.periodic
        && c.knots[..2].iter().all(|&t| t == d[0])
        && c.knots[2..].iter().all(|&t| t == d[1])
        && c.control_points[0] != c.control_points[1]
}
/// Fresh chart injectivity, exact source planarity and original boundary hulls.
/// Both faces must cover the complete shared line, including root partitions.
pub fn certify(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_spans: usize,
) -> Result<Report> {
    if faces[0] == faces[1]
        || faces.iter().any(|&i| i >= shell.faces().len())
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_spans)
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose distinct faces and bounded exact/chart work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_CONTACT",
            "Allowed contacts require qualified original regions",
        )
    })?;
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        spans: 0,
        reason: "source-contact-chart-unproven",
    };
    for &f in &faces {
        if out.spans == max_spans {
            return Ok(out);
        }
        let report = nurbs_core::surface_injectivity::certify_contraction(
            regions[f].loops()[0][0].surface(),
            max_spans - out.spans,
        )?;
        out.spans += report.spans;
        if !report.proven {
            return Ok(out);
        }
    }
    out.reason = "source-contact-planarity-unproven";
    let Some(a_plane) = plane(
        regions[faces[0]].loops()[0][0].surface(),
        &mut out.exact_work,
        max_work,
    )?
    else {
        return Ok(out);
    };
    let Some(b_plane) = plane(
        regions[faces[1]].loops()[0][0].surface(),
        &mut out.exact_work,
        max_work,
    )?
    else {
        return Ok(out);
    };
    out.reason = "source-contact-support-unproven";
    for (index, edge) in shell.edges().iter().enumerate() {
        if !shell.uses()[index].iter().all(|a| faces.contains(&a.face)) || !linear(edge.world()) {
            continue;
        }
        let indices = (0..shell.edges().len())
            .filter(|&i| {
                shell.edges()[i].world() == edge.world()
                    && shell.uses()[i].iter().all(|a| faces.contains(&a.face))
            })
            .collect::<Vec<_>>();
        if indices[0] != index || !faces.iter().all(|&f| complete(shell, &indices, f)) {
            continue;
        }
        let endpoints: [[_; 3]; 2] =
            std::array::from_fn(|i| std::array::from_fn(|k| edge.world().control_points[i][k]));
        let [a, b] = endpoints;
        for c in a_plane.into_iter().chain(b_plane) {
            if !independent(a, b, c, &mut out.exact_work, max_work)? {
                continue;
            }
            let mut sides = [None; 2];
            let mut zeros: [Vec<[f64; 3]>; 2] = [Vec::new(), Vec::new()];
            let mut valid = true;
            for (which, &face) in faces.iter().enumerate() {
                for (i, e) in shell.edges().iter().enumerate() {
                    if !shell.uses()[i].iter().any(|a| a.face == face) {
                        continue;
                    }
                    for p in &e.world().control_points {
                        let p = [p[0], p[1], p[2]];
                        match orient(&[a, b, c, p], None, &mut out.exact_work, max_work)? {
                            Some(Sign::Zero) => {
                                zeros[which].push(p);
                            }
                            Some(sign) => {
                                if sides[which].is_some_and(|s| s != sign) {
                                    valid = false;
                                    break;
                                }
                                sides[which] = Some(sign);
                            }
                            None => {
                                valid = false;
                                break;
                            }
                        }
                        if !valid {
                            break;
                        }
                    }
                    if !valid {
                        break;
                    }
                }
                if !valid {
                    break;
                }
            }
            if !valid || sides == [None, None] || (sides[0].is_some() && sides[0] == sides[1]) {
                continue;
            }
            // Contact belongs to the zero support hulls of both faces.
            // One non-coplanar side confined to the complete owned line is
            // enough; the other face may lie wholly in the separator plane.
            let mut confined = false;
            for side in 0..2 {
                if sides[side].is_none() {
                    continue;
                }
                let mut on_line = true;
                for &p in &zeros[side] {
                    if (0..3).any(|k| p[k] < a[k].min(b[k]) || p[k] > a[k].max(b[k])) {
                        on_line = false;
                        break;
                    }
                    for axes in [[0, 1], [0, 2], [1, 2]] {
                        if orient(&[a, b, p], Some(axes), &mut out.exact_work, max_work)?
                            != Some(Sign::Zero)
                        {
                            on_line = false;
                            break;
                        }
                    }
                    if !on_line {
                        break;
                    }
                }
                confined |= on_line;
                if confined {
                    break;
                }
            }
            if !confined {
                continue;
            }
            out.certificate = Some(Certificate {
                faces,
                regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                edges: indices.iter().map(|&i| shell.edges()[i].clone()).collect(),
                plane: [a, b, c],
            });
            out.reason = "source-shared-line-contact-qualified";
            return Ok(out);
        }
    }
    Ok(out)
}
