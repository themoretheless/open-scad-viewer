use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_advanced_extrude(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
        let twist = self.value(&item["twist"], scope, &current, "twist", ANGLE)?;
        let scale = self.vector(&item["top_scale"], scope, &current, "top_scale", SCALAR)?;
        if height <= 0. || scale.iter().any(|v| *v <= 0.) {
            return Err(fail(
                "invalid_dimension",
                "Height and top scales must be positive.",
            ));
        }
        if twist.abs() > 3600. {
            return Err(fail(
                "invalid_angle",
                "Twist must be within +/-3600 degrees.",
            ));
        }
        self.lines.push(format!(
            "linear_extrude(height={},twist={},scale={},slices={},center={}){{",
            number(height),
            number(twist),
            json_string(&json!(scale)),
            number(item["slices"].as_f64().unwrap()),
            boolean(item, "center")
        ));
        self.child(item, nodes, scope, &current, depth, GeometryType::Profile)?;
        self.lines.push("}".into());
        Ok(())
    }
    pub(super) fn emit_cone(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let bottom = self.value(
            &item["radius_bottom"],
            scope,
            &current,
            "radius_bottom",
            LENGTH,
        )?;
        let top = self.value(&item["radius_top"], scope, &current, "radius_top", LENGTH)?;
        let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
        if bottom < 0. || top < 0. || bottom + top <= 0. || height <= 0. {
            return Err(fail(
                "invalid_dimension",
                "Cone needs nonnegative radii, at least one positive radius, and positive height.",
            ));
        }
        self.lines.push(format!(
            "cylinder(r1={},r2={},h={},center={});",
            number(bottom),
            number(top),
            number(height),
            boolean(item, "center")
        ));
        Ok(())
    }
    pub(super) fn emit_torus(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let major = self.value(
            &item["major_radius"],
            scope,
            &current,
            "major_radius",
            LENGTH,
        )?;
        let minor = self.value(
            &item["minor_radius"],
            scope,
            &current,
            "minor_radius",
            LENGTH,
        )?;
        if minor <= 0. || major <= minor {
            return Err(fail(
                "invalid_dimension",
                "Torus requires major_radius > minor_radius > 0.",
            ));
        }
        self.lines.push(format!(
            "rotate_extrude(angle=360){{translate([{},0,0]){{circle(r={});}}}}",
            number(major),
            number(minor)
        ));
        Ok(())
    }
    pub(super) fn emit_extrude_revolve(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            nodes,
            scope,
            current,
            depth,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        if op == "extrude" {
            let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
            if height <= 0. {
                return Err(fail(
                    "invalid_dimension",
                    "Extrusion height must be positive.",
                ));
            }
            self.lines.push(format!(
                "linear_extrude(height={},center={}){{",
                number(height),
                boolean(item, "center")
            ));
        } else {
            let angle = self.value(&item["angle"], scope, &current, "angle", ANGLE)?;
            if angle <= 0. || angle > 360. {
                return Err(fail(
                    "invalid_angle",
                    "Revolution angle must be greater than zero and at most 360 degrees.",
                ));
            }
            self.lines
                .push(format!("rotate_extrude(angle={}){{", number(angle)));
        }
        self.child(item, nodes, scope, &current, depth, GeometryType::Profile)?;
        self.lines.push("}".into());
        Ok(())
    }
}
