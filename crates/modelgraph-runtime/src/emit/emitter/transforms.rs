use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_affine(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let mut m = vec![];
        for (i, row) in array(&item["rows"]).iter().enumerate() {
            let mut values = vec![];
            for (j, v) in array(row).iter().enumerate() {
                values.push(self.value(
                    v,
                    scope,
                    &current,
                    &format!("rows/{i}/{j}"),
                    if j == 3 { LENGTH } else { SCALAR },
                )?);
            }
            m.push(values);
        }
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if !det.is_finite() || det == 0. {
            return Err(fail(
                "invalid_transform",
                "Affine matrix must be invertible.",
            ));
        }
        if expected == GeometryType::Profile && (m[2][0] != 0. || m[2][1] != 0. || m[2][3] != 0.) {
            return Err(fail(
                "nonplanar_profile",
                "Affine transform must preserve XY.",
            ));
        }
        m.push(vec![0., 0., 0., 1.]);
        self.lines
            .push(format!("multmatrix({}){{", json_string(&json!(m))));
        self.child(item, nodes, scope, &current, depth, expected)?;
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_mirror(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let normal = self.vector(&item["normal"], scope, &current, "normal", SCALAR)?;
        if normal.iter().all(|v| *v == 0.) {
            return Err(fail("invalid_normal", "Mirror normal must be nonzero."));
        }
        if expected == GeometryType::Profile && normal[2] != 0. {
            return Err(fail(
                "nonplanar_profile",
                "Profile mirror normal must be in XY.",
            ));
        }
        self.lines
            .push(format!("mirror({}){{", json_string(&json!(normal))));
        self.child(item, nodes, scope, &current, depth, expected)?;
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_projection(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            nodes,
            scope,
            current,
            depth,
            ..
        } = *cx;
        self.lines
            .push(format!("projection(cut={}){{", op == "section"));
        if op == "section" {
            let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
            self.lines
                .push(format!("translate([0,0,{}]){{", number(-height)));
        }
        self.child(item, nodes, scope, &current, depth, GeometryType::Solid)?;
        if op == "section" {
            self.lines.push("}".into());
        }
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_offset(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            ..
        } = *cx;
        let distance = self.value(&item["distance"], scope, &current, "distance", LENGTH)?;
        self.lines.push(format!(
            "offset({}={}){{",
            if item["mode"].as_str() == Some("delta") {
                "delta"
            } else {
                "r"
            },
            number(distance)
        ));
        self.child(item, nodes, scope, &current, depth, GeometryType::Profile)?;
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_pattern(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let count = self.value(&item["count"], scope, &current, "count", SCALAR)?;
        if count.fract() != 0. || !(1.0..=256.0).contains(&count) {
            return Err(fail(
                "invalid_count",
                "Pattern count must be an integer from 1 to 256.",
            ));
        }
        let step = if op == "linear_pattern" {
            self.vector(&item["step"], scope, &current, "step", LENGTH)?
        } else {
            vec![
                0.,
                0.,
                self.value(&item["angle_step"], scope, &current, "angle_step", ANGLE)?,
            ]
        };
        if expected == GeometryType::Profile && op == "linear_pattern" && step[2] != 0. {
            return Err(fail(
                "nonplanar_profile",
                "Profile pattern must remain in XY.",
            ));
        }
        self.lines.push("union(){".into());
        for i in 0..count as usize {
            let vector: Vec<f64> = step.iter().map(|v| v * i as f64).collect();
            if vector.iter().any(|v| v.abs() > 1e6) {
                return Err(fail(
                    "graph_limit",
                    "Pattern transform exceeds numeric limits.",
                ));
            }
            self.lines.push(format!(
                "{}({}){{",
                if op == "linear_pattern" {
                    "translate"
                } else {
                    "rotate"
                },
                json_string(&json!(vector))
            ));
            self.emit(
                str_at(item, "input"),
                nodes,
                scope,
                &format!("{current}[{i}]"),
                depth + 1,
                expected,
                None,
                false,
            )?;
            self.lines.push("}".into());
        }
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_transform(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let vector = self.geometry_vector(
            &item["vector"],
            scope,
            &current,
            "vector",
            if op == "rotate" {
                ANGLE
            } else if op == "translate" {
                LENGTH
            } else {
                SCALAR
            },
            3,
        )?;
        if expected == GeometryType::Profile
            && ((op == "translate" && vector[2] != 0.)
                || (op == "rotate" && (vector[0] != 0. || vector[1] != 0.))
                || (op == "scale" && vector[2] != 1.))
        {
            return Err(fail(
                "nonplanar_profile",
                "Profiles must remain in XY: translate Z=0, rotate X=Y=0, scale Z=1.",
            ));
        }
        if op == "scale" && vector.contains(&0.) {
            return Err(Error::new(
                "invalid_scale",
                format!("{current}/vector"),
                "Scale factors must be nonzero.",
            ));
        }
        self.lines
            .push(format!("{op}({}){{", json_string(&json!(vector))));
        self.child(item, nodes, scope, &current, depth, expected)?;
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_boolean(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        self.lines.push(format!("{op}(){{"));
        for child in children(item) {
            self.emit(
                child,
                nodes,
                scope,
                &current,
                depth + 1,
                expected,
                None,
                false,
            )?;
        }
        self.lines.push("}".into());
        Ok(())
    }
}
