//! Booleans of two original graph columns whose roofs never cross.
//!
//! Both columns share one exact XY footprint and one flat floor height. An
//! exact polynomial sign certificate proves one roof lies on or below the
//! other over the whole footprint, so the lower column is a subset of the upper
//! one. Union and intersection then return an original source unchanged and no
//! SS curve, sampled point or result triangulation takes part. Crossing roofs
//! stay with the closed-graph SS Boolean; a strict upper-minus-lower slab is
//! refused here because it needs new boundary construction.
use super::{GraphBooleanOperation, QualifiedGraphBody, error, require};
use crate::{Model, Result};
use nurbs_core::ss_intersection::{GraphOrderLimits, prove_polynomial_graph_boundary_side};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NestedGraphResult {
    /// The result is the unchanged original source of this operand.
    Operand(usize),
    Empty,
}

pub struct NestedGraphBoolean {
    model: Model,
    result: NestedGraphResult,
    upper: usize,
    coincident: bool,
    cells: usize,
}
impl NestedGraphBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn into_model(self) -> Model {
        self.model
    }
    pub fn result(&self) -> NestedGraphResult {
        self.result
    }
    /// Operand whose roof is proved on or above the other roof.
    pub fn upper(&self) -> usize {
        self.upper
    }
    /// Both order certificates hold, so the roofs coincide as point sets.
    pub fn coincident(&self) -> bool {
        self.coincident
    }
    /// Exact sign cells spent by the accepted order certificates.
    pub fn cells(&self) -> usize {
        self.cells
    }
}

fn roof_below(
    bodies: [&QualifiedGraphBody<'_>; 2],
    lower: usize,
    limits: GraphOrderLimits,
) -> Option<usize> {
    let roof = |side: usize| bodies[side].equivalences()[bodies[side].roof()].polynomial();
    prove_polynomial_graph_boundary_side(roof(1 - lower), roof(lower), true, limits).ok()
}

pub fn nested_graph_column_boolean(
    bodies: [&QualifiedGraphBody<'_>; 2],
    operation: GraphBooleanOperation,
    limits: GraphOrderLimits,
) -> Result<NestedGraphBoolean> {
    if let GraphBooleanOperation::Difference { minuend } = operation {
        require(minuend <= 1, "Choose original graph minuend 0 or 1")?;
    }
    for body in bodies {
        body.reverify()?;
    }
    require(
        bodies[0].footprint() == bodies[1].footprint(),
        "Nested graph columns need one exact common footprint",
    )?;
    require(
        bodies[0].floor_height() == bodies[1].floor_height(),
        "Nested graph columns need one exact common floor",
    )?;
    let first_below = roof_below(bodies, 0, limits);
    let second_below = roof_below(bodies, 1, limits);
    let (upper, coincident, cells) = match (first_below, second_below) {
        (Some(a), Some(b)) => (1, true, a + b),
        (Some(a), None) => (1, false, a),
        (None, Some(b)) => (0, false, b),
        (None, None) => {
            return Err(error(
                "Graph roofs are not proved ordered; crossing roofs need the SS closed-graph Boolean",
            ));
        }
    };
    let lower = 1 - upper;
    let result = match operation {
        GraphBooleanOperation::Union => NestedGraphResult::Operand(upper),
        GraphBooleanOperation::Intersection => NestedGraphResult::Operand(lower),
        GraphBooleanOperation::Difference { minuend } if minuend == lower || coincident => {
            NestedGraphResult::Empty
        }
        GraphBooleanOperation::Difference { .. } => {
            return Err(error(
                "Upper-minus-lower graph slab needs new boundary construction",
            ));
        }
    };
    let model = match result {
        NestedGraphResult::Operand(side) => bodies[side].source().clone(),
        NestedGraphResult::Empty => Model::empty(
            bodies[0]
                .source()
                .tolerance_mm
                .max(bodies[1].source().tolerance_mm),
        )?,
    };
    model.validate()?;
    Ok(NestedGraphBoolean {
        model,
        result,
        upper,
        coincident,
        cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_body::{
        qualify,
        tests::{column, column_at_floor, limits},
    };
    use nurbs_core::surface::Surface;

    fn roof(lift: f64, tilt: f64) -> Surface {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let bump = [0.25, -0.25, 0.25][i] + [0.25, -0.25, 0.25][j];
                            vec![i as f64 / 2., j as f64 / 2., 2. + bump]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        for (i, row) in s.control_points.iter_mut().enumerate() {
            for p in row {
                p[2] += lift + tilt * (i as f64 - 1.);
            }
        }
        s
    }

    fn ops() -> [GraphBooleanOperation; 4] {
        [
            GraphBooleanOperation::Union,
            GraphBooleanOperation::Intersection,
            GraphBooleanOperation::Difference { minuend: 0 },
            GraphBooleanOperation::Difference { minuend: 1 },
        ]
    }

    #[test]
    fn ordered_roofs_return_original_columns_under_operand_swap() {
        let low = column(roof(0., 0.));
        let high = column(roof(0.5, 0.));
        let saved = [low.clone(), high.clone()];
        let budget = limits();
        for swapped in [false, true] {
            let models = if swapped { [&high, &low] } else { [&low, &high] };
            let a = qualify(models[0], 0, 1, &budget).unwrap();
            let b = qualify(models[1], 0, 1, &budget).unwrap();
            let upper = usize::from(!swapped);
            for op in ops() {
                let r = nested_graph_column_boolean([&a, &b], op, GraphOrderLimits::default());
                if op == (GraphBooleanOperation::Difference { minuend: upper }) {
                    assert!(r.is_err(), "{op:?}");
                    continue;
                }
                let r = r.unwrap();
                assert_eq!(r.upper(), upper);
                assert!(!r.coincident());
                assert!(r.cells() > 0);
                let expected = match op {
                    GraphBooleanOperation::Union => NestedGraphResult::Operand(upper),
                    GraphBooleanOperation::Intersection => NestedGraphResult::Operand(1 - upper),
                    _ => NestedGraphResult::Empty,
                };
                assert_eq!(r.result(), expected, "{op:?}");
                match expected {
                    NestedGraphResult::Operand(side) => assert_eq!(r.model(), models[side]),
                    NestedGraphResult::Empty => assert!(r.model().faces.is_empty()),
                }
            }
        }
        assert_eq!([low, high], saved);
    }

    #[test]
    fn coincident_roofs_make_every_difference_empty() {
        let a = column(roof(0., 0.));
        let b = a.clone();
        let budget = limits();
        let qa = qualify(&a, 0, 1, &budget).unwrap();
        let qb = qualify(&b, 0, 1, &budget).unwrap();
        for op in ops() {
            let r =
                nested_graph_column_boolean([&qa, &qb], op, GraphOrderLimits::default()).unwrap();
            assert!(r.coincident());
            match op {
                GraphBooleanOperation::Difference { .. } => {
                    assert_eq!(r.result(), NestedGraphResult::Empty)
                }
                _ => assert_eq!(r.model(), &a),
            }
        }
    }

    #[test]
    fn crossing_roofs_floors_minuends_and_budgets_are_refused() {
        let budget = limits();
        // Opposite tilts cross along the middle of the footprint.
        let left = column(roof(0., 0.125));
        let right = column(roof(0., -0.125));
        let ql = qualify(&left, 0, 1, &budget).unwrap();
        let qr = qualify(&right, 0, 1, &budget).unwrap();
        for op in ops() {
            assert!(
                nested_graph_column_boolean([&ql, &qr], op, GraphOrderLimits::default()).is_err()
            );
        }
        let raised = column_at_floor(roof(0.5, 0.), 0.25);
        let qh = qualify(&raised, 0, 1, &budget).unwrap();
        assert!(
            nested_graph_column_boolean(
                [&ql, &qh],
                GraphBooleanOperation::Union,
                GraphOrderLimits::default()
            )
            .is_err()
        );
        let low = column(roof(0., 0.));
        let high = column(roof(0.0009765625, 0.));
        let a = qualify(&low, 0, 1, &budget).unwrap();
        let b = qualify(&high, 0, 1, &budget).unwrap();
        let ok = nested_graph_column_boolean(
            [&a, &b],
            GraphBooleanOperation::Union,
            GraphOrderLimits::default(),
        )
        .unwrap();
        assert_eq!(ok.upper(), 1);
        assert!(
            nested_graph_column_boolean(
                [&a, &b],
                GraphBooleanOperation::Difference { minuend: 2 },
                GraphOrderLimits::default()
            )
            .is_err()
        );
        assert!(
            nested_graph_column_boolean(
                [&a, &b],
                GraphBooleanOperation::Union,
                GraphOrderLimits {
                    max_cells: 0,
                    max_depth: 24
                }
            )
            .is_err()
        );
    }
}
