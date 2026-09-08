use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
pub type Matrix = [f64; 16];
const IDENTITY: Matrix = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];
pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut out = [0.; 16];
    for r in 0..4 {
        for c in 0..4 {
            for k in 0..4 {
                out[r * 4 + c] += a[r * 4 + k] * b[k * 4 + c];
            }
        }
    }
    out
}
fn vector(v: &Value) -> [f64; 3] {
    [
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    ]
}
pub fn frame(origin: [f64; 3], rotation: [f64; 3]) -> Matrix {
    let [x, y, z] = rotation.map(|v| v * std::f64::consts::PI / 180.);
    let (cx, sx, cy, sy, cz, sz) = (x.cos(), x.sin(), y.cos(), y.sin(), z.cos(), z.sin());
    let mut m = multiply(
        multiply(
            [
                cz, -sz, 0., 0., sz, cz, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
            [
                cy, 0., sy, 0., 0., 1., 0., 0., -sy, 0., cy, 0., 0., 0., 0., 1.,
            ],
        ),
        [
            1., 0., 0., 0., 0., cx, -sx, 0., 0., sx, cx, 0., 0., 0., 0., 1.,
        ],
    );
    for i in 0..3 {
        m[i * 4 + 3] = origin[i];
    }
    m
}
fn frame_value(v: &Value) -> Matrix {
    frame(vector(&v["origin"]), vector(&v["rotation"]))
}
fn inverse(m: Matrix) -> Matrix {
    let mut out = IDENTITY;
    for r in 0..3 {
        for c in 0..3 {
            out[r * 4 + c] = m[c * 4 + r];
        }
        out[r * 4 + 3] = -(0..3).map(|k| out[r * 4 + k] * m[k * 4 + 3]).sum::<f64>();
    }
    out
}
pub fn matrix_json(m: Matrix) -> Value {
    json!([&m[0..4], &m[4..8], &m[8..12], &m[12..16]])
}
struct Solver<'a> {
    components: HashMap<&'a str, &'a Value>,
    solved: HashMap<&'a str, Matrix>,
    active: HashSet<&'a str>,
    path: &'a str,
}
impl<'a> Solver<'a> {
    fn err(&self, message: impl Into<String>) -> Error {
        Error::new("invalid_assembly", self.path, message)
    }
    fn anchor(&self, id: &str, name: &str) -> Result<Matrix> {
        let c = self
            .components
            .get(id)
            .ok_or_else(|| self.err(format!("Unknown component {id}.")))?;
        let a = c["anchors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"].as_str() == Some(name))
            .ok_or_else(|| self.err(format!("Unknown anchor {id}.{name}.")))?;
        Ok(frame_value(a))
    }
    fn solve(&mut self, id: &'a str) -> Result<Matrix> {
        if let Some(m) = self.solved.get(id) {
            return Ok(*m);
        }
        let c = *self
            .components
            .get(id)
            .ok_or_else(|| self.err(format!("Unknown component {id}.")))?;
        if !self.active.insert(id) {
            return Err(self.err(format!("Cyclic mate through {id}.")));
        }
        let matrix = if !c["placement"].is_null() {
            frame_value(&c["placement"])
        } else {
            let m = &c["mate"];
            let j = &m["joint"];
            let mut position = 0.;
            let kind = j["kind"].as_str().unwrap_or("");
            if !j.is_null() {
                position = j["position"].as_f64().unwrap();
                let min = j["min"].as_f64().unwrap();
                let max = j["max"].as_f64().unwrap();
                if min > max || position < min || position > max {
                    return Err(self.err(format!("Joint {id} is outside its limits.")));
                }
            }
            let offset = frame(
                [0., 0., if kind == "slider" { position } else { 0. }],
                [0., 0., if kind == "revolute" { position } else { 0. }],
            );
            let target = m["component"].as_str().unwrap();
            multiply(
                multiply(
                    multiply(
                        self.solve(target)?,
                        self.anchor(target, m["anchor"].as_str().unwrap())?,
                    ),
                    multiply(
                        frame([0., 0., m["gap"].as_f64().unwrap()], vector(&m["rotation"])),
                        offset,
                    ),
                ),
                inverse(self.anchor(id, m["own_anchor"].as_str().unwrap())?),
            )
        };
        if matrix.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
            return Err(self.err("Assembly transform exceeds numeric limits."));
        }
        self.active.remove(id);
        self.solved.insert(id, matrix);
        Ok(matrix)
    }
}
pub fn place(components: &[Value], path: &str) -> Result<Vec<Value>> {
    let mut s = Solver {
        components: HashMap::with_capacity(components.len()),
        solved: HashMap::with_capacity(components.len()),
        active: HashSet::new(),
        path,
    };
    for c in components {
        let id = c["id"].as_str().unwrap();
        if s.components.insert(id, c).is_some() {
            return Err(s.err("Component IDs must be unique."));
        }
    }
    for c in components {
        let id = c["id"].as_str().unwrap();
        let mut anchors = HashSet::new();
        for a in c["anchors"].as_array().unwrap() {
            if !anchors.insert(a["id"].as_str().unwrap()) {
                return Err(s.err(format!("Duplicate anchor in {id}.")));
            }
        }
        if c["placement"].is_null() == c["mate"].is_null() {
            return Err(s.err(format!(
                "Component {id} must have exactly one placement or mate."
            )));
        }
    }
    let mut out = Vec::with_capacity(components.len());
    for c in components {
        let id = c["id"].as_str().unwrap();
        let matrix = s.solve(id)?;
        let anchors: Vec<Value> = c["anchors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| json!({"id":a["id"],"matrix":multiply(matrix,frame_value(a))}))
            .collect();
        out.push(json!({"id":id,"input":c["input"],"joint":c["mate"]["joint"],"matrix":matrix,"anchors":anchors}));
    }
    Ok(out)
}
pub fn read_matrix(v: &Value) -> Matrix {
    let mut m = [0.; 16];
    for (i, x) in v.as_array().unwrap().iter().enumerate() {
        m[i] = x.as_f64().unwrap();
    }
    m
}
