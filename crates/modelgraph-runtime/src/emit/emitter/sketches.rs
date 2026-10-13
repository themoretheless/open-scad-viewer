use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_loft(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let count = array(&item["profile"]).len();
        self.profile_work += count * count + count * array(&item["sections"]).len();
        if self.profile_work > 262144 {
            return Err(fail("profile_limit", "Loft work budget exceeded."));
        }
        let mut profile = vec![];
        for (i, p) in array(&item["profile"]).iter().enumerate() {
            let v = self.vector(p, scope, &current, &format!("profile/{i}"), LENGTH)?;
            profile.push([v[0], v[1]]);
        }
        let mut sections = vec![];
        for (i, s) in array(&item["sections"]).iter().enumerate() {
            let scale = self.vector(
                &s["scale"],
                scope,
                &current,
                &format!("sections/{i}/scale"),
                SCALAR,
            )?;
            let offset = self.vector(
                &s["offset"],
                scope,
                &current,
                &format!("sections/{i}/offset"),
                LENGTH,
            )?;
            sections.push(profiles::Section {
                z: self.value(&s["z"], scope, &current, &format!("sections/{i}/z"), LENGTH)?,
                scale: [scale[0], scale[1]],
                offset: [offset[0], offset[1]],
            });
        }
        let mesh = profiles::loft(profile, &sections, &current)?;
        self.lines.push(format!(
            "polyhedron(points={},faces={});",
            json_string(&mesh["points"]),
            json_string(&mesh["faces"])
        ));
        Ok(())
    }
    pub(super) fn emit_sketch(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        if self.sketch_solutions.len() >= 16 {
            return Err(fail(
                "sketch_limit",
                "Maximum 16 sketch solves per compilation.",
            ));
        }
        let mut points = vec![];
        for (i, p) in array(&item["points"]).iter().enumerate() {
            points.push(json!({"id":p["id"],"position":self.vector(&p["position"],scope,&current,&format!("points/{i}/position"),LENGTH)?}));
        }
        let mut constraints = vec![];
        for (i, c) in array(&item["constraints"]).iter().enumerate() {
            let mut constraint = c.clone();
            match str_at(c, "kind") {
                "fix" => {
                    constraint["at"] = json!(self.vector(
                        &c["at"],
                        scope,
                        &current,
                        &format!("constraints/{i}/at"),
                        LENGTH
                    )?)
                }
                "distance" => {
                    constraint["value"] = json!(self.value(
                        &c["value"],
                        scope,
                        &current,
                        &format!("constraints/{i}/value"),
                        LENGTH
                    )?)
                }
                _ => {}
            }
            constraints.push(constraint);
        }
        let boundary = array(&item["boundary"]);
        let mut seen = HashSet::new();
        for id in boundary {
            if !seen.insert(id.as_str().unwrap()) || !points.iter().any(|p| p["id"] == *id) {
                return Err(fail(
                    "invalid_boundary",
                    "Boundary IDs must exist and be unique; closing is implicit.",
                ));
            }
        }
        let mut report = sketch::solve(&points, &constraints, &current)?;
        report["instance_path"] = json!(current);
        self.sketch_solutions.push(report.clone());
        let status = str_at(&report, "status");
        if status != "solved"
            && !(status == "underconstrained" && boolean(item, "allow_underconstrained"))
        {
            return Err(fail(
                &format!("sketch_{status}"),
                "Sketch could not produce a fully constrained profile. Inspect the solver report.",
            )
            .with_details(report));
        }
        let polygon: Vec<[f64; 2]> = boundary
            .iter()
            .map(|id| {
                let p = array(&report["points"])
                    .iter()
                    .find(|p| p["id"] == *id)
                    .unwrap();
                [
                    p["position"][0].as_f64().unwrap(),
                    p["position"][1].as_f64().unwrap(),
                ]
            })
            .collect();
        profiles::validate(&polygon, &current)?;
        self.lines
            .push(format!("polygon(points={});", json_string(&json!(polygon))));
        Ok(())
    }
}
