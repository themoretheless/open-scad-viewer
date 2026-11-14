//! Screen-size selection among existing viewer tessellation levels.
//!
//! Thin transport over `nurbs_core::tessellation::lod::select`; the caller owns
//! the levels and their error bounds, this operation never builds geometry.
use crate::*;
use nurbs_core::tessellation::lod::{self, Level, Reason, Request};

pub(crate) fn dispatch(v: Value) -> Result<Value> {
    let levels = v["levels"]
        .as_array()
        .ok_or_else(|| input("Viewer LOD requires a levels array"))?
        .iter()
        .map(|level| {
            Ok(Level {
                triangle_count: field(level, "triangleCount")?,
                error_upper: field(level, "errorUpper")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let current = optional_field::<usize>(&v, "current")?;
    let request = &v["request"];
    let selection = lod::select(
        &levels,
        current,
        Request {
            pixels_per_unit: field(request, "pixelsPerUnit")?,
            pixel_budget: field(request, "pixelBudget")?,
            triangle_budget: field(request, "triangleBudget")?,
            hysteresis: field(request, "hysteresis")?,
        },
    )?;
    Ok(json!({
        "level": selection.level,
        "projectedError": selection.projected_error,
        "withinPixelBudget": selection.within_pixel_budget,
        "reason": match selection.reason {
            Reason::Initial => "initial",
            Reason::PixelBudget => "pixel-budget",
            Reason::HysteresisHold => "hysteresis-hold",
            Reason::TriangleBudget => "triangle-budget",
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(levels: Value, current: Value, pixels_per_unit: f64) -> Value {
        json!({"levels": levels, "current": current, "request": {
            "pixelsPerUnit": pixels_per_unit, "pixelBudget": 1.0, "triangleBudget": 100_000, "hysteresis": 1.5}})
    }

    fn three() -> Value {
        json!([{"triangleCount": 100, "errorUpper": 0.4},
               {"triangleCount": 400, "errorUpper": 0.1},
               {"triangleCount": 1600, "errorUpper": 0.025}])
    }

    #[test]
    fn picks_finest_level_within_budgets_and_steps_down_when_far() {
        // 20 px/unit: projected 8, 2, 0.5 px; only the fine level fits 1 px.
        let near = dispatch(request(three(), Value::Null, 20.)).unwrap();
        assert_eq!(near["level"], 2);
        assert_eq!(near["reason"], "initial");
        // 0.5 px/unit: the fine level is far below budget / hysteresis and the
        // middle level still fits, so the view steps down one level per query.
        let far = dispatch(request(three(), json!(2), 0.5)).unwrap();
        assert_eq!(far["level"], 1);
        assert_eq!(far["reason"], "pixel-budget");
        assert_eq!(far["withinPixelBudget"], true);
    }

    #[test]
    fn holds_current_level_inside_hysteresis_band() {
        let held = dispatch(request(three(), json!(1), 12.)).unwrap();
        assert_eq!(held["level"], 1);
        assert_eq!(held["reason"], "hysteresis-hold");
    }

    #[test]
    fn refuses_malformed_levels() {
        assert!(dispatch(request(
            json!([{"triangleCount": 100, "errorUpper": 0.4}]),
            Value::Null,
            1.
        ))
        .is_err());
        assert!(dispatch(json!({"request": {}})).is_err());
    }
}
