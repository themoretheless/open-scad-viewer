//! Browser WebGPU SDF sweep: prepare/finish split over the value dispatch.
//! The kernel flattens the field and ships the shared shader text; the host
//! samples the grid on the GPU; extraction finishes here on the CPU reference
//! path. Ineligible fields return null and the caller uses `sdf_tessellate`.
use crate::{encode, field, input};
use sdf_kernel::{Field, Grid, SDF_WGSL};
use std::cell::RefCell;
use std::collections::BTreeMap;
use value_codec::{json, Value};

thread_local! {
    static PENDING: RefCell<(u64, BTreeMap<u64, (Field, Grid)>)> =
        RefCell::new((0, BTreeMap::new()));
}

pub fn prepare(v: &Value) -> crate::Result<Value> {
    let sdf: Field = field(v, "field")?;
    let grid: Grid = field(v, "grid")?;
    sdf_kernel::check_grid_budget(&sdf, &grid)?;
    let Some(flat) = sdf.to_flat() else {
        return encode(Value::Null);
    };
    let id = PENDING.with(|pending| {
        let mut pending = pending.borrow_mut();
        pending.0 += 1;
        let id = pending.0;
        pending.1.insert(id, (sdf, grid.clone()));
        id
    });
    encode(json!({
        "id": id,
        "kinds": flat.kinds,
        "params": flat.params,
        "aux": flat.aux,
        "triangles": flat.triangles,
        "min": grid.min,
        "max": grid.max,
        "cells": grid.cells,
        "wgsl": SDF_WGSL,
    }))
}

pub fn finish(v: &Value) -> crate::Result<Value> {
    let id = field::<u64>(v, "id")?;
    let values64 = field::<Vec<f64>>(v, "values")?;
    let values: Vec<f32> = values64.iter().map(|&x| x as f32).collect();
    let (field, grid) = PENDING
        .with(|pending| pending.borrow_mut().1.remove(&id))
        .ok_or_else(|| input("Unknown SDF sweep handle"))?;
    let mesh = sdf_kernel::polygonize_with_values(&field, &grid, &values)?;
    let report = mesh.inspect()?;
    encode(polygon_kernel::BuiltMesh { mesh, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;

    #[test]
    fn prepare_finish_roundtrip_matches_reference_extraction() {
        // Sphere radius 10 at the origin, small grid.
        let field = json!({"kind": "sphere", "center": [0., 0., 0.], "radius": 10.});
        let grid = json!({"min": [-12., -12., -12.], "max": [12., 12., 12.], "cells": [16, 16, 16]});
        let prepared = prepare(&json!({"field": field, "grid": grid})).unwrap();
        let id = prepared["id"].as_f64().unwrap() as u64;
        assert!(prepared["wgsl"].as_str().unwrap().contains("@compute"));
        // Host substitute: sample the field on the CPU, rounding through f32
        // exactly like the shader output.
        let reference = sdf_kernel::polygonize(
            &value_codec::from_value::<Field>(field).unwrap(),
            &value_codec::from_value::<Grid>(grid).unwrap(),
        )
        .unwrap();
        let mut values = Vec::new();
        let [nx, ny, nz] = [16usize, 16, 16];
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p: [f64; 3] = std::array::from_fn(|i| -12. + 24. * [x, y, z][i] as f64 / 16.);
                    let dist = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 10.;
                    values.push(dist as f32 as f64);
                }
            }
        }
        let mesh_value = finish(&json!({"id": id, "values": values})).unwrap();
        let mesh: polygon_kernel::BuiltMesh = value_codec::from_value(mesh_value).unwrap();
        assert_eq!(mesh.mesh.indices, reference.indices);
        // Second use of the same handle must fail.
        assert!(finish(&json!({"id": id, "values": []})).is_err());
    }
}
