use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_rectangle_box(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            scope,
            current,
            ..
        } = *cx;
        let size = self.geometry_vector(
            &item["size"],
            scope,
            &current,
            "size",
            LENGTH,
            if op == "box" { 3 } else { 2 },
        )?;
        if size.iter().any(|v| *v <= 0.) {
            return Err(Error::new(
                "invalid_dimension",
                if op == "box" {
                    format!("{current}/size")
                } else {
                    current.clone()
                },
                if op == "box" {
                    "Box dimensions must be positive."
                } else {
                    "Rectangle dimensions must be positive."
                },
            ));
        }
        self.lines.push(format!(
            "{}({},center={});",
            if op == "box" { "cube" } else { "square" },
            json_string(&json!(size)),
            boolean(item, "center")
        ));
        Ok(())
    }
    pub(super) fn emit_round(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            op,
            scope,
            current,
            ..
        } = *cx;
        let radius = self.value(&item["radius"], scope, &current, "radius", LENGTH)?;
        if radius <= 0. {
            return Err(Error::new(
                "invalid_dimension",
                if op == "circle" {
                    current.clone()
                } else {
                    format!("{current}/radius")
                },
                if op == "circle" {
                    "Circle radius must be positive."
                } else {
                    "Radius must be positive."
                },
            ));
        }
        if op == "cylinder" {
            let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
            if height <= 0. {
                return Err(Error::new(
                    "invalid_dimension",
                    format!("{current}/height"),
                    "Height must be positive.",
                ));
            }
            self.lines.push(format!(
                "cylinder(r={},h={},center={});",
                number(radius),
                number(height),
                boolean(item, "center")
            ));
        } else {
            self.lines.push(format!("{op}(r={});", number(radius)));
        }
        Ok(())
    }
    pub(super) fn emit_polygon(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let mut points = vec![];
        for (i, p) in array(&item["points"]).iter().enumerate() {
            let v = self.vector(p, scope, &current, &format!("points/{i}"), LENGTH)?;
            points.push([v[0], v[1]]);
        }
        self.profile_work += points.len() * points.len();
        if self.profile_work > 262144 {
            return Err(fail(
                "profile_limit",
                "Polygon validation work budget exceeded.",
            ));
        }
        profiles::validate(&points, &current)?;
        self.lines
            .push(format!("polygon(points={});", json_string(&json!(points))));
        Ok(())
    }
}
