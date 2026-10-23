//! Bounded proposal generation only. Original trimmed-volume audits own proof.
use crate::Model;
use nurbs_core::{Error, Result};
#[derive(Clone, Copy)]
pub struct Query {
    pub face: usize,
    pub uv: [f64; 2],
}
pub struct Plan {
    pub targets: Vec<Query>,
    pub sources: Vec<Query>,
    pub diagonal: f64,
    pub tolerance: f64,
    pub budget: usize,
}
#[derive(Clone, Copy)]
pub struct Sample {
    pub point: [f64; 3],
    pub normal: Option<[f64; 3]>,
}
pub struct Candidate {
    pub face: usize,
    pub uv: [f64; 2],
    pub origin: [f64; 3],
    pub direction: [f64; 3],
}
fn fail(s: &str) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", s)
}
fn norm(p: [f64; 3]) -> f64 {
    p[0].hypot(p[1]).hypot(p[2])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
pub fn plan(model: &Model, groups: Option<[Vec<usize>; 2]>, budget: usize) -> Result<Plan> {
    if !(1..=256).contains(&budget) {
        return Err(fail("Invalid wall search budget"));
    }
    if model.faces.is_empty() || model.faces.len() > 10000 {
        return Err(fail("Invalid wall search source"));
    }
    let groups = if let Some(g) = groups {
        if g.iter().any(|g| {
            g.is_empty()
                || g.iter().any(|i| *i >= model.faces.len())
                || g.iter().enumerate().any(|(k, i)| g[..k].contains(i))
        }) || g[0].iter().any(|i| g[1].contains(i))
        {
            return Err(fail("Invalid wall search groups"));
        }
        g
    } else {
        let faces = (0..model.faces.len()).collect::<Vec<_>>();
        [faces.clone(), faces]
    };
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for f in &model.faces {
        for p in f.surface.control_points.iter().flatten() {
            if p.len() != 3 || !p.iter().all(|x| x.is_finite()) {
                return Err(fail("Invalid wall search bounds"));
            }
            for k in 0..3 {
                bounds[k][0] = bounds[k][0].min(p[k]);
                bounds[k][1] = bounds[k][1].max(p[k]);
            }
        }
    }
    let diagonal = norm(bounds.map(|x| x[1] - x[0]));
    if !diagonal.is_finite() || diagonal <= 0. {
        return Err(fail("Invalid wall search bounds"));
    }
    let query = |face: usize, a: f64, b: f64| -> Result<Query> {
        let s = &model.faces[face].surface;
        s.validate()?;
        let u = [
            s.knots_u[s.degree_u],
            s.knots_u[s.knots_u.len() - s.degree_u - 1],
        ];
        let v = [
            s.knots_v[s.degree_v],
            s.knots_v[s.knots_v.len() - s.degree_v - 1],
        ];
        Ok(Query {
            face,
            uv: [u[0] * (1. - a) + u[1] * a, v[0] * (1. - b) + v[1] * b],
        })
    };
    let targets = groups[1]
        .iter()
        .take(budget)
        .map(|i| query(*i, 0.5, 0.5))
        .collect::<Result<Vec<_>>>()?;
    let mut sources = Vec::new();
    'samples: for a in [0.5, 0.25, 0.75] {
        for b in [0.5, 0.25, 0.75] {
            for face in &groups[0] {
                if sources.len() == 4 * budget {
                    break 'samples;
                }
                sources.push(query(*face, a, b)?);
            }
        }
    }
    Ok(Plan {
        targets,
        sources,
        diagonal,
        tolerance: model.tolerance_mm,
        budget,
    })
}
pub fn evaluate(model: &Model, queries: &[Query]) -> Result<Vec<Sample>> {
    queries
        .iter()
        .map(|q| {
            let e = model.faces[q.face].surface.evaluate(q.uv[0], q.uv[1])?;
            Ok(Sample {
                point: e.point,
                normal: e.unit_normal(),
            })
        })
        .collect()
}
pub fn choose(plan: &Plan, targets: &[Sample], sources: &[Sample]) -> Result<Vec<Candidate>> {
    if targets.len() != plan.targets.len() || sources.len() != plan.sources.len() {
        return Err(fail("Invalid wall search samples"));
    }
    let mut result = Vec::new();
    for (q, s) in plan.sources.iter().zip(sources) {
        let Some(n) = s.normal else { continue };
        if !n.iter().chain(s.point.iter()).all(|x| x.is_finite()) {
            continue;
        }
        let length = norm(n);
        if length <= 0. || !length.is_finite() {
            continue;
        }
        let unit = n.map(|x| x / length);
        let aligned = targets
            .iter()
            .filter(|t| {
                t.normal
                    .is_some_and(|n| dot(n, unit).abs() / norm(n) > 1. - 1e-6)
            })
            .collect::<Vec<_>>();
        let target = if aligned.is_empty() {
            targets.iter().collect::<Vec<_>>()
        } else {
            aligned
        };
        let projection = target
            .iter()
            .map(|t| dot(std::array::from_fn(|k| t.point[k] - s.point[k]), unit))
            .filter(|p| p.is_finite() && p.abs() > plan.tolerance)
            .reduce(|a, b| if b.abs() < a.abs() { b } else { a });
        let Some(projection) = projection else {
            continue;
        };
        let pad = (plan.tolerance * 10.).max(projection.abs() * 0.05);
        let ray = unit.map(|x| x * projection.signum());
        let offset = 2. * plan.diagonal;
        result.push(Candidate {
            face: q.face,
            uv: q.uv,
            origin: std::array::from_fn(|k| s.point[k] - ray[k] * offset),
            direction: ray.map(|x| x * (offset + projection.abs() + pad)),
        });
        if result.len() == plan.budget {
            break;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidates_are_bounded_source_preserving_and_never_a_coverage_proof() {
        let model = crate::analytic::cylinder(10., 20.).unwrap();
        let before = model.clone();
        let p = plan(&model, None, 8).unwrap();
        assert_eq!(p.targets.len(), model.faces.len().min(8));
        assert_eq!(p.sources.len(), 32);
        let targets = evaluate(&model, &p.targets).unwrap();
        let sources = evaluate(&model, &p.sources).unwrap();
        let proposals = choose(&p, &targets, &sources).unwrap();
        assert!(!proposals.is_empty());
        assert!(proposals.len() <= 8);
        assert!(proposals.iter().all(|c| {
            c.origin
                .iter()
                .chain(c.direction.iter())
                .all(|x| x.is_finite())
        }));
        let singular = sources
            .iter()
            .map(|s| Sample {
                point: s.point,
                normal: None,
            })
            .collect::<Vec<_>>();
        assert!(choose(&p, &targets, &singular).unwrap().is_empty());
        assert!(plan(&model, None, 0).is_err());
        assert!(plan(&model, Some([vec![0], vec![0]]), 8).is_err());
        assert_eq!(model, before);
    }
}
