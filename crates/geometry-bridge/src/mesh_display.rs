use crate::{Result, encode, field, input};
use mesh_query::display;
use value_codec::{Value, json};
pub fn dispatch(v: Value) -> Result<Value> {
    let op = v["op"].as_str().unwrap_or("");
    if op == "mesh_measure" {
        let start: [f64; 3] = field(&v, "start")?;
        let end: [f64; 3] = field(&v, "end")?;
        if start.iter().chain(&end).any(|v| !v.is_finite()) {
            return Err(input("Measurement points must be finite"));
        }
        let delta = math_core::sub(end, start);
        return encode(
            json!({"start":start,"end":end,"delta":delta,"deltaX":delta[0],"deltaY":delta[1],"deltaZ":delta[2],"distance":delta[0].hypot(delta[1]).hypot(delta[2])}),
        );
    }
    let vertices: Vec<f32> = field::<Vec<u32>>(&v, "vertexBits")?.into_iter().map(f32::from_bits).collect();
    let matrix: display::Matrix = field(&v, "transform")?;
    if op == "mesh_display_inspect" {
        let bounds = display::bounds(&vertices, &matrix);
        return encode(match bounds {
            Some((min, max)) => {
                json!({"bounds":{"min":min,"max":max},"dimensions":math_core::sub(max,min),"center":std::array::from_fn::<_,3,_>(|k|(min[k]+max[k])/2.)})
            }
            None => json!({"bounds":null,"dimensions":[0,0,0],"center":null}),
        });
    }
    let indices: Vec<u32> = field(&v, "indices")?;
    let Some(face) = v["triangleIndex"]
        .as_u64()
        .and_then(|i| usize::try_from(i).ok())
    else {
        return Ok(Value::Null);
    };
    if op == "mesh_hit_point" {
        encode(display::hit_point(
            &vertices,
            &indices,
            face,
            &matrix,
            field(&v, "barycentric")?,
        ))
    } else {
        encode(display::hit_normal(&vertices, &indices, face, &matrix))
    }
}
