//! Admitted rational concatenation; all numerical edits remain proposals.
use super::*;
#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub curve: &'a Curve,
    pub reversed: bool,
}
#[derive(Clone, Debug)]
pub struct Segment {
    pub source_parameters: [f64; 2],
    pub output_parameters: [f64; 2],
    pub agreement: mapped::Report,
}
pub struct Concatenation {
    sources: Vec<Curve>,
    directions: Vec<bool>,
    curve: Curve,
    segments: Vec<Segment>,
    closed: bool,
    tolerance: f64,
    max_cells: usize,
    visited: usize,
}
impl Concatenation {
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    pub fn visited(&self) -> usize {
        self.visited
    }
    pub fn closed(&self) -> bool {
        self.closed
    }
    pub fn reverify(&self) -> Result<()> {
        let input = self
            .sources
            .iter()
            .zip(&self.directions)
            .map(|(curve, &reversed)| Input { curve, reversed })
            .collect::<Vec<_>>();
        let actual = concatenate(&input, self.closed, self.tolerance, self.max_cells)?;
        check(
            actual.curve == self.curve
                && actual.segments.iter().zip(&self.segments).all(|(a, b)| {
                    a.source_parameters == b.source_parameters
                        && a.output_parameters == b.output_parameters
                }),
            "Saved concatenation differs from current original reconstruction",
        )
    }
}
/// Each source receives one unit output interval, retaining an explicit affine
/// original-clock map. Degree elevation, weight-gauge matching, endpoint
/// coupling and reparameterization are numerical proposals. Every complete
/// actual output interval must pass fresh unchanged-original admission.
/// `closed` also proposes a common first/last endpoint; it does not assert
/// regularity, simplicity, tangency, winding or solid topology.
pub fn concatenate(
    input: &[Input<'_>],
    closed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Concatenation> {
    check(
        !input.is_empty() && input.len() <= 64,
        "Concatenation needs 1..64 original curves",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Concatenation needs bounded positive work and tolerance",
    )?;
    for c in input {
        c.curve.validate()?;
        check(
            continuous(c.curve),
            "Concatenation sources must be continuous",
        )?;
    }
    let degree = input.iter().map(|c| c.curve.degree).max().unwrap();
    let dimension = input[0].curve.control_points[0].len();
    check(
        input
            .iter()
            .all(|c| c.curve.control_points[0].len() == dimension),
        "Concatenation dimensions differ",
    )?;
    let mut result: Option<Curve> = None;
    for (i, source) in input.iter().enumerate() {
        let mut c = if source.reversed {
            source.curve.reverse()?
        } else {
            source.curve.clone()
        };
        let d = c.domain();
        c = c.trim(d[0], d[1])?.elevate(degree)?;
        let d = c.domain();
        for k in &mut c.knots {
            *k = if *k == d[0] {
                i as f64
            } else if *k == d[1] {
                (i + 1) as f64
            } else {
                i as f64 + (*k - d[0]) / (d[1] - d[0])
            };
        }
        c.periodic = false;
        c.validate()?;
        if let Some(r) = &mut result {
            let ratio = *r.weights.last().unwrap() / c.weights[0];
            for w in &mut c.weights {
                *w *= ratio;
            }
            c.control_points[0] = r.control_points.last().unwrap().clone();
            c.weights[0] = *r.weights.last().unwrap();
            c.validate()?;
            check(
                r.control_points.len() + c.control_points.len() - 1 <= 256,
                "Concatenation exceeds 256 controls",
            )?;
            r.knots.pop();
            r.knots.extend_from_slice(&c.knots[degree + 1..]);
            r.control_points
                .extend(c.control_points.into_iter().skip(1));
            r.weights.extend(c.weights.into_iter().skip(1));
            r.validate()?;
        } else {
            result = Some(c);
        }
    }
    let mut curve = result.unwrap();
    if closed {
        let last = curve.control_points.len() - 1;
        curve.control_points[last] = curve.control_points[0].clone();
    }
    curve.validate()?;
    let mut visited = 0;
    let mut segments = Vec::new();
    for (i, source) in input.iter().enumerate() {
        check(visited < max_cells, "Concatenation proof work exhausted")?;
        let mut d = source.curve.domain();
        if source.reversed {
            d.reverse();
        }
        let output = [i as f64, (i + 1) as f64];
        let agreement = mapped::verify(
            source.curve,
            &curve,
            d,
            output,
            tolerance,
            max_cells - visited,
        )?;
        visited += agreement.cells;
        check(
            agreement.accepted,
            "Actual concatenated interval exceeds tolerance or remains unproven",
        )?;
        segments.push(Segment {
            source_parameters: d,
            output_parameters: output,
            agreement,
        });
    }
    Ok(Concatenation {
        sources: input.iter().map(|i| i.curve.clone()).collect(),
        directions: input.iter().map(|i| i.reversed).collect(),
        curve,
        segments,
        closed,
        tolerance,
        max_cells,
        visited,
    })
}
