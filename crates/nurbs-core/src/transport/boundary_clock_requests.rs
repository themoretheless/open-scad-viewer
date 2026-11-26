use super::*;
use crate::analysis::exact_plane_intersection::{
    AlgebraicBoundaryClock, isolate_ordered_algebraic_boundary_clocks,
    order_algebraic_boundary_clocks,
};
use num_rational::BigRational;

fn rational(value: &Value, key: &str) -> Result<BigRational> {
    let text: String = field(value, key)?;
    check(!text.is_empty() && text.len() <= 512, "Contact clock ratio must have 1..512 characters")?;
    text.parse().map_err(|_| input("Expected a signed integer or numerator/denominator ratio"))
}

pub(super) fn inspect(value: &Value) -> Result<Value> {
    let data: Vec<Value> = field(value, "clocks")?;
    check((1..=4096).contains(&data.len()), "Contact ordering requires 1..4096 clock values")?;
    let clocks = data.iter().map(|clock| {
        if clock.get("rational").is_some() {
            Ok(AlgebraicBoundaryClock::Rational(rational(clock, "rational")?))
        } else {
            Ok(AlgebraicBoundaryClock::Quadratic {
                linear: rational(clock, "linear")?,
                constant: rational(clock, "constant")?,
                upper_root: field(clock, "upper_root")?,
            })
        }
    }).collect::<Result<Vec<_>>>()?;
    let max_work = field(value, "max_work")?;
    if field(value, "isolate")? {
        let report = isolate_ordered_algebraic_boundary_clocks(&clocks, max_work)?;
        return Ok(match report.outcome {
            Ok(result) => {
                let intervals: Vec<_> = result.clocks.iter().map(|clock| {
                    json!({"indices":clock.indices,
                        "bounds":clock.parameter.each_ref().map(ToString::to_string)})
                }).collect();
                let witnesses: Vec<_> = result.witnesses.iter().map(ToString::to_string).collect();
                json!({"complete":true,"intervals":intervals,"witnesses":witnesses,"workUsed":report.work_used})
            }
            Err(reason) => json!({"complete":false,"reason":format!("{reason:?}"),"workUsed":report.work_used}),
        });
    }
    let report = order_algebraic_boundary_clocks(&clocks, max_work)?;
    Ok(match report.outcome {
        Ok(groups) => json!({"complete":true,"groups":groups,"workUsed":report.work_used}),
        Err(reason) => json!({"complete":false,"reason":format!("{reason:?}"),"workUsed":report.work_used}),
    })
}
