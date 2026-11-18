//! Compatible section bases for native loft and curve-network callers.
use crate::{check, curve::Curve, Result};
/// Clamp the active domains, normalize to [0,1], elevate to the largest degree,
/// and unify exact knot multiplicities. No orientation or phase inference.
pub fn compatible(curves: &[Curve]) -> Result<Vec<Curve>> {
    check(
        (2..=32).contains(&curves.len()),
        "Compatibility requires 2..32 sections",
    )?;
    for c in curves {
        c.validate()?;
        check(
            c.control_points[0].len() == curves[0].control_points[0].len(),
            "Section dimensions must match",
        )?;
    }
    let degree = curves.iter().map(|c| c.degree).max().unwrap();
    let mut sections = Vec::new();
    for c in curves {
        let [a, b] = c.domain();
        let mut c = c.trim(a, b)?;
        let original = c.knots.clone();
        c.knots.iter_mut().for_each(|k| *k = (*k - a) / (b - a));
        check(
            original
                .windows(2)
                .zip(c.knots.windows(2))
                .all(|(x, y)| x[0] == x[1] || y[0] < y[1]),
            "Section normalization collapsed distinct knots",
        )?;
        c.validate()?;
        if c.degree < degree {
            c = c.elevate(degree)?;
        }
        sections.push(c);
    }
    let mut unique: Vec<f64> = sections
        .iter()
        .flat_map(|c| c.knots.iter().copied())
        .filter(|k| *k > 0. && *k < 1.)
        .collect();
    unique.sort_by(f64::total_cmp);
    unique.dedup();
    for k in unique {
        let count = sections
            .iter()
            .map(|c| c.knots.iter().filter(|&&v| v == k).count())
            .max()
            .unwrap();
        for c in &mut sections {
            let existing = c.knots.iter().filter(|&&v| v == k).count();
            if existing < count {
                *c = c.insert(k, count - existing)?;
            }
        }
    }
    Ok(sections)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Anchor,
    Forward,
    Reversed,
    Ambiguous,
}
pub struct OrientedSections {
    pub curves: Vec<Curve>,
    pub orientations: Vec<Orientation>,
}
/// Endpoint-distance heuristic relative to the first section. `ambiguity`
/// is a length threshold for the difference between the two matching costs.
/// Ambiguous sections are preserved. No closed-section phase inference.
pub fn orient(curves: &[Curve], ambiguity: f64) -> Result<OrientedSections> {
    check(
        (2..=32).contains(&curves.len()),
        "Orientation requires 2..32 sections",
    )?;
    check(
        ambiguity.is_finite() && ambiguity >= 0.,
        "Orientation ambiguity must be finite and nonnegative",
    )?;
    for c in curves {
        c.validate()?;
        check(
            c.control_points[0].len() == curves[0].control_points[0].len(),
            "Section dimensions must match",
        )?;
    }
    let ends = |c: &Curve| -> Result<(Vec<f64>, Vec<f64>)> {
        let [a, b] = c.domain();
        Ok((c.evaluate(a)?.point, c.evaluate(b)?.point))
    };
    let distance = |a: &[f64], b: &[f64]| a.iter().zip(b).fold(0_f64, |d, (x, y)| d.hypot(x - y));
    let (a, b) = ends(&curves[0])?;
    let mut output = OrientedSections {
        curves: vec![curves[0].clone()],
        orientations: vec![Orientation::Anchor],
    };
    for c in &curves[1..] {
        let (p, q) = ends(c)?;
        let forward = distance(&a, &p) + distance(&b, &q);
        let reversed = distance(&a, &q) + distance(&b, &p);
        let decision = if (forward - reversed).abs() <= ambiguity {
            Orientation::Ambiguous
        } else if reversed < forward {
            Orientation::Reversed
        } else {
            Orientation::Forward
        };
        output.curves.push(if decision == Orientation::Reversed {
            c.reverse()?
        } else {
            c.clone()
        });
        output.orientations.push(decision);
    }
    Ok(output)
}
