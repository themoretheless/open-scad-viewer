use super::*;

impl Compiler {
    pub(crate) fn geometry(&mut self, v: V) -> R<String> {
        match v {
            V::Array(a) => {
                let inputs = a
                    .into_iter()
                    .map(|v| self.geometry(v))
                    .collect::<R<Vec<_>>>()?;
                let g = self.add(json!({"op":"group","inputs":inputs}))?;
                let id = self.geometry(g)?;
                self.collections.insert(id.clone());
                Ok(id)
            }
            V::Json(j) if j.get("geometry").is_some() => Ok(s(&j, "geometry").into()),
            V::Json(j) if empty_value_sequence(&j["sequence"]) => {
                let value = self.geometry(V::Array(Vec::new()))?;
                let protected =
                    self.protect(V::Json(json!({"geometry":value})), &[j["sequence"].clone()])?;
                self.geometry(protected)
            }
            V::Json(j) if empty_match_sequence(&j["sequence"]) => {
                let sequence = &j["sequence"];
                let empty = self.geometry(V::Array(Vec::new()))?;
                let mut arms = Vec::new();
                for original in arr(sequence, "arms") {
                    let mut arm = original.clone();
                    let input = if s(&arm["body"], "op") != "list" {
                        let protected = self
                            .protect(V::Json(json!({"geometry":&empty})), &[arm["body"].clone()])?;
                        self.geometry(protected)?
                    } else {
                        empty.clone()
                    };
                    arm.as_object_mut().unwrap().remove("body");
                    arm["input"] = json!(input);
                    arms.push(arm);
                }
                let value =
                    self.add(json!({"op":"match","value":&sequence["input"],"arms":arms}))?;
                let id = self.geometry(value)?;
                self.collections.insert(id.clone());
                Ok(id)
            }
            _ => Err(crate::error("Expected geometry")),
        }
    }
}
