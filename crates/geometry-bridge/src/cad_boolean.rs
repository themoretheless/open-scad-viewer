//! Atomic ordered Boolean reduction for homogeneous mesh or B-rep body records.
use super::{Result, Value, brep, encode, field, input};
pub fn boolean(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let operation: String = field(&v, "operation")?;
    if bodies.len() < 2 {
        return Err(input("Select at least two bodies; the first is the base."));
    }
    if !["union", "difference", "intersection", "xor"].contains(&operation.as_str()) {
        return Err(input("Invalid Boolean operation."));
    }
    let retained = bodies.iter().filter(|b| b.get("brep").is_some()).count();
    if retained == 0 {
        return mesh_boolean(bodies, &operation);
    }
    if retained != bodies.len() {
        return Err(input("Boolean operands must all retain B-rep geometry."));
    }
    // Admit every input, including operands that would follow an empty result.
    let models = bodies
        .iter()
        .map(|b| {
            let m: brep_core::Model = field(b, "brep")?;
            m.validate()?;
            Ok(m)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut result = models[0].clone();
    for model in &models[1..] {
        result = brep_core::boolean(&result, model, &operation)?;
    }
    if result.bodies.is_empty() {
        return encode(Vec::<Value>::new());
    }
    let mesh = brep::nurbs(&result, 4)?.built.mesh;
    let mut body = bodies[0]
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    body.insert("brep".into(), encode(result)?);
    body.insert("mesh".into(), encode(mesh)?);
    encode(vec![Value::Object(body)])
}

fn mesh_boolean(bodies: Vec<Value>, operation: &str) -> Result<Value> {
    use polygon_core::{
        Mesh,
        solid::boolean::{Operation, Options, boolean, difference_many, union_many},
    };
    let op = match operation {
        "union" => Operation::Union,
        "difference" => Operation::Difference,
        "intersection" => Operation::Intersection,
        _ => {
            return Err(input(
                "Mesh Boolean supports union, difference and intersection.",
            ));
        }
    };
    // Every operand is admitted before any fold, including operands that would
    // follow an empty intermediate result.
    let meshes = bodies
        .iter()
        .map(|b| {
            let m: Mesh = field(b, "mesh")?;
            m.validate()?;
            Ok(m)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut pairwise = |a: &Mesh, b: &Mesh| Ok(boolean(a, b, op, &Options::default())?.mesh);
    let mesh = match op {
        Operation::Union => union_many(&meshes, &mut pairwise)?,
        Operation::Difference => {
            difference_many(&meshes[0], &meshes[1..], &mut pairwise, super::mesh::DIFFERENCE_BATCH)?
        }
        Operation::Intersection => {
            let mut m = meshes[0].clone();
            for next in &meshes[1..] {
                m = pairwise(&m, next)?;
            }
            m
        }
    };
    if mesh.indices.is_empty() {
        return encode(Vec::<Value>::new());
    }
    let mut body = bodies[0]
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    body.insert("mesh".into(), encode(mesh)?);
    encode(vec![Value::Object(body)])
}
