//! Thin display-math protocol over the native viewport implementation.
use super::{Result, Value, field, input};
use math_core::viewport;
use value_codec::json;
pub fn dispatch(v: Value) -> Result<Value> {
    if v["action"].as_str() == Some("orbit") {
        use math_core::orbit_camera::{Options, frame};
        let projection: String = field(&v, "projection")?;
        if projection != "perspective" && projection != "orthographic" {
            return Err(input("Invalid orbit projection mode"));
        }
        let options = Options {
            yaw: field(&v, "yaw")?,
            pitch: field(&v, "pitch")?,
            distance: field(&v, "distance")?,
            target: field(&v, "target")?,
            aspect: field(&v, "aspect")?,
            fov_y: field(&v, "fovY")?,
            perspective: projection == "perspective",
            background_radius: field(&v, "backgroundRadius")?,
            bounds: if v["bounds"].is_null() {
                None
            } else {
                Some((field(&v["bounds"], "min")?, field(&v["bounds"], "max")?))
            },
        };
        let f = frame(&options).ok_or_else(|| input("Invalid or unrepresentable orbit camera"))?;
        return Ok(
            json!({"eye":f.eye,"viewProjection":f.view_projection,"eyeDistance":f.eye_distance,"near":f.near,"far":f.far}),
        );
    }
    let matrix = field(&v, "matrix")?;
    match v["action"].as_str() {
        Some(action @ ("inverse" | "multiply" | "transpose")) => {
            let result = match action {
                "inverse" => viewport::inverse(&matrix)
                    .ok_or_else(|| input("Singular or unrepresentable matrix inverse"))?,
                "multiply" => viewport::multiply(&matrix, &field(&v, "other")?),
                _ => viewport::transpose(&matrix),
            }
            .map(|v| v as f32 as f64);
            if !result.iter().all(|v| v.is_finite()) {
                return Err(input("Matrix result exceeds finite GPU storage"));
            }
            Ok(json!(result))
        }
        Some("client_ray") => Ok(
            match viewport::client_ray(&matrix, field(&v, "rect")?, field(&v, "client")?) {
                Some(ray) => json!({"origin":ray.origin,"direction":ray.direction}),
                None => Value::Null,
            },
        ),
        Some("unproject") => Ok(
            match viewport::unproject(&matrix, field(&v, "x")?, field(&v, "y")?) {
                Some(ray) => json!({"origin":ray.origin,"direction":ray.direction}),
                None => Value::Null,
            },
        ),
        Some("project") => Ok(
            match viewport::project(&matrix, field(&v, "point")?, field(&v, "size")?) {
                Some(p) => json!(p),
                None => Value::Null,
            },
        ),
        Some("corner") => Ok(
            match viewport::selected_corner(&matrix, field(&v, "vertices")?, field(&v, "weights")?)
            {
                Some(p) => json!(p),
                None => Value::Null,
            },
        ),
        _ => Err(input("Unknown viewport operation")),
    }
}
