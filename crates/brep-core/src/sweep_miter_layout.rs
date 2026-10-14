//! Native layout of the complete progressive miter request and preview union.
//! This is construction/resource evidence, never a smoothness or Solid proof.
use crate::{Error, MAX_FACES, Result};
use nurbs_core::{curve::Curve, surface::Surface};

fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_MITER_LAYOUT_INVALID", message))
    }
}

pub struct Plan {
    pub edges: usize,
    pub spans: usize,
    pub max_steps: usize,
}

/// Count actual decomposed source spans before allocating any body stations.
pub fn plan(
    loops: &[Vec<Curve>],
    sites: usize,
    closed: bool,
    initial: usize,
    maximum: usize,
) -> Result<Plan> {
    require(
        (1..=16).contains(&loops.len()) && loops.iter().all(|r| !r.is_empty()),
        "Miter body needs 1..16 nonempty loops",
    )?;
    let profiles: usize = loops.iter().map(Vec::len).sum();
    require(
        profiles <= 64,
        "Progressive miter body exceeds its site/span budget",
    )?;
    require(
        if closed {
            (3..=16).contains(&sites)
        } else {
            (2..=17).contains(&sites)
        },
        "Miter needs 2..17 open or 3..16 cyclic sites",
    )?;
    let edges = sites - usize::from(!closed);
    let mut spans = 0;
    for curve in loops.iter().flatten() {
        spans += curve.decompose()?.len();
        require(
            spans <= 64,
            "Progressive miter body exceeds its site/span budget",
        )?;
    }
    require(
        spans > 0,
        "Progressive miter body exceeds its site/span budget",
    )?;
    let cap_faces = if closed { 0 } else { 2 };
    let max_steps = maximum
        .min(1024 / edges)
        .min((MAX_FACES - cap_faces) / (edges * spans));
    require(
        max_steps >= initial,
        "Progressive miter initial steps exceed face budget",
    )?;
    require(
        initial >= 1 && initial <= max_steps,
        "Miter refinement steps must be ordered within the body face budget",
    )?;
    Ok(Plan {
        edges,
        spans,
        max_steps,
    })
}

/// Every section must cover exactly the declared ring partition, with no tail.
pub fn partition(sections: &[Vec<Curve>], rings: &[usize]) -> Result<Vec<Vec<Vec<Curve>>>> {
    require(
        (1..=16).contains(&rings.len()) && rings.iter().all(|&n| (1..=64).contains(&n)),
        "Miter body needs 1..16 nonempty loops",
    )?;
    let profiles: usize = rings.iter().sum();
    require(
        profiles <= 64
            && (2..=1025).contains(&sections.len())
            && sections.iter().all(|r| r.len() == profiles),
        "Miter station ownership requires complete ring coverage",
    )?;
    Ok(sections
        .iter()
        .map(|row| {
            let mut offset = 0;
            rings
                .iter()
                .map(|&count| {
                    let ring = row[offset..offset + count].to_vec();
                    offset += count;
                    ring
                })
                .collect()
        })
        .collect())
}

/// Original polyline vertices retain independent one-sided jets, including
/// the cyclic seam. Uniform complete station coverage is required first.
pub fn sharp_stations(
    stations: usize,
    edges: usize,
    steps: usize,
    closed: bool,
) -> Result<Vec<usize>> {
    require(
        ((if closed { 3 } else { 1 })..=16).contains(&edges)
            && (1..=1024 / edges.max(1)).contains(&steps)
            && stations == edges * steps + 1
            && stations <= 1025,
        "Miter station ownership requires complete uniform span coverage",
    )?;
    Ok(if closed {
        (0..edges).map(|i| i * steps).collect()
    } else {
        (1..edges).map(|i| i * steps).collect()
    })
}

pub struct Preview {
    pub patches: Vec<Surface>,
    pub profile_patch_ranges: Vec<[usize; 2]>,
}

/// Construct the whole declared preview wall set, in profile/station/span
/// order. Geometry remains a candidate: decomposition is not an exact bound.
pub fn preview(sections: &[Vec<Curve>]) -> Result<Preview> {
    require(
        (2..=1025).contains(&sections.len()),
        "Miter preview requires 2..1025 stations",
    )?;
    let profiles = sections[0].len();
    require(
        (1..=64).contains(&profiles) && sections.iter().all(|r| r.len() == profiles),
        "Miter preview requires complete profile coverage",
    )?;
    let mut rows = Vec::with_capacity(sections.len());
    let mut expected = None;
    for row in sections {
        let parts = row
            .iter()
            .map(|c| {
                c.decompose().map(|p| {
                    p.into_iter()
                        .map(|s| s.definition().clone())
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let counts = parts.iter().map(Vec::len).collect::<Vec<_>>();
        let spans: usize = counts.iter().sum();
        require(
            spans > 0 && spans <= 64 && spans * (sections.len() - 1) <= MAX_FACES,
            "Miter preview exceeds the body face budget",
        )?;
        require(
            expected.as_ref().is_none_or(|e| e == &counts),
            "Miter preview span partition mismatch",
        )?;
        expected = Some(counts);
        rows.push(parts);
    }
    let mut patches = Vec::new();
    let mut ranges = Vec::with_capacity(profiles);
    for p in 0..profiles {
        let start = patches.len();
        for station in 0..rows.len() - 1 {
            for (a, b) in rows[station][p].iter().zip(&rows[station + 1][p]) {
                require(
                    a.degree == b.degree
                        && a.knots == b.knots
                        && a.control_points.len() == b.control_points.len(),
                    "Miter preview curve family mismatch",
                )?;
                let patch = Surface {
                    degree_u: a.degree,
                    degree_v: 1,
                    knots_u: a.knots.clone(),
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: a
                        .control_points
                        .iter()
                        .zip(&b.control_points)
                        .map(|(a, b)| vec![a.clone(), b.clone()])
                        .collect(),
                    weights: a
                        .weights
                        .iter()
                        .zip(&b.weights)
                        .map(|(&a, &b)| vec![a, b])
                        .collect(),
                    periodic_u: false,
                    periodic_v: false,
                };
                patch.validate()?;
                patches.push(patch);
            }
        }
        ranges.push([start, patches.len()]);
    }
    Ok(Preview {
        patches,
        profile_patch_ranges: ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn circle(z: f64, r: f64) -> Curve {
        nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], r).unwrap()
    }
    #[test]
    fn native_plan_counts_source_spans_and_reserves_all_caps() {
        let loops = vec![vec![circle(0., 1.)], vec![circle(0., 0.25)]];
        let p = plan(&loops, 17, false, 1, 64).unwrap();
        assert_eq!((p.edges, p.spans, p.max_steps), (16, 8, 7));
        assert!(plan(&loops, 17, false, 8, 64).is_err());
        assert_eq!(plan(&loops, 16, true, 1, 64).unwrap().max_steps, 8);
        // A one-edge source can validly refine beyond 64, up to its real
        // face budget. Preserve that existing native sweep capability.
        assert_eq!(
            plan(&[vec![circle(0., 1.)]], 2, false, 1, 256)
                .unwrap()
                .max_steps,
            255
        );
        assert_eq!(
            sharp_stations(256, 1, 255, false).unwrap(),
            Vec::<usize>::new()
        );
        assert!(plan(&[], 2, false, 1, 64).is_err());
        assert!(plan(&[vec![]], 2, false, 1, 64).is_err());
        assert!(plan(&loops, 1, false, 1, 64).is_err());
        assert!(plan(&loops, 2, false, 0, 64).is_err());
        assert!(plan(&loops, 2, false, 3, 2).is_err());
        let mut bad = loops;
        bad[0][0].weights[0] = 0.;
        assert!(plan(&bad, 2, false, 1, 64).is_err());
    }
    #[test]
    fn native_partition_and_sharp_vertices_require_whole_uniform_coverage() {
        let flat = (0..7)
            .map(|z| vec![circle(z as f64, 1.), circle(z as f64, 0.25)])
            .collect::<Vec<_>>();
        let rings = partition(&flat, &[1, 1]).unwrap();
        assert_eq!(rings.len(), 7);
        assert_eq!(rings[6][1][0], flat[6][1]);
        assert!(partition(&flat, &[1]).is_err());
        assert!(partition(&flat, &[1, 0, 1]).is_err());
        let mut truncated = flat.clone();
        truncated[3].pop();
        assert!(partition(&truncated, &[1, 1]).is_err());
        assert_eq!(sharp_stations(7, 3, 2, false).unwrap(), vec![2, 4]);
        assert_eq!(sharp_stations(7, 3, 2, true).unwrap(), vec![0, 2, 4]);
        assert!(sharp_stations(6, 3, 2, true).is_err());
        assert!(sharp_stations(3, 1, 2, true).is_err());
    }
    #[test]
    fn preview_covers_every_hollow_wall_and_refuses_missing_or_changed_spans() {
        let flat = (0..3)
            .map(|z| vec![circle(z as f64, 1.), circle(z as f64, 0.25)])
            .collect::<Vec<_>>();
        let p = preview(&flat).unwrap();
        assert_eq!(p.patches.len(), 16);
        assert_eq!(p.profile_patch_ranges, vec![[0, 8], [8, 16]]);
        assert_eq!(
            p.patches[0].control_points[0][0],
            flat[0][0].decompose().unwrap()[0]
                .definition()
                .control_points[0]
        );
        assert!(preview(&flat[..1]).is_err());
        let mut missing = flat.clone();
        missing[1].pop();
        assert!(preview(&missing).is_err());
        let mut changed = flat.clone();
        changed[1][0] =
            nurbs_core::paths::bezier(vec![vec![1., 0., 1.], vec![0., 1., 1.]], None).unwrap();
        assert!(preview(&changed).is_err());
        let over = (0..130)
            .map(|z| vec![circle(z as f64, 1.), circle(z as f64, 0.25)])
            .collect::<Vec<_>>();
        assert!(preview(&over).is_err());
    }
}
