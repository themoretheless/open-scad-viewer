use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_planetary_spinner(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            key,
            scope,
            current,
            depth,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        if depth != 1 {
            return Err(fail(
                "mechanism_scope",
                "Planetary spinner must be the root to preserve separate parts.",
            ));
        }
        let r = self.value(
            &item["inner_radius"],
            scope,
            &current,
            "inner_radius",
            LENGTH,
        )?;
        let outer = self.value(
            &item["outer_radius"],
            scope,
            &current,
            "outer_radius",
            LENGTH,
        )?;
        let bore = self.value(&item["bore"], scope, &current, "bore", LENGTH)?;
        let gap = self.value(&item["gap"], scope, &current, "gap", LENGTH)?;
        let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
        let helix = self.value(&item["helix_angle"], scope, &current, "helix_angle", ANGLE)?;
        let scale = r / 30.845;
        if r <= 0.
            || outer <= r
            || height <= 0.
            || gap < 0.
            || gap > 0.6 * scale
            || !(0.0..80.0).contains(&helix)
            || bore < 0.
            || bore >= 2. * (23.154388 * scale - gap / 2.) * (std::f64::consts::PI / 45.).cos()
            || outer * (std::f64::consts::PI / 96.).cos() <= r + gap / 2.
        {
            return Err(fail(
                "invalid_spinner",
                "Invalid spinner dimensions, gap, bore or rim thickness.",
            ));
        }
        self.lines.push(format!("inner_radius={};outer_radius={};center_hole_diameter={};gap={};spinner_height={};helix_angle={};",number(r),number(outer),number(bore),number(gap),number(height),number(helix)));
        self.lines
            .push(include_str!("../../planetary_spinner.scad").into());
        self.mechanical_reports.push(json!({"node_id":key,"kind":"planetary_spinner","sun_teeth":32,"planet_teeth":4,"ring_teeth":40,"planet_count":18,"gap_mm":gap,"profile":"fixed sampled prototype","printability":"unknown"}));
        Ok(())
    }
    pub(super) fn emit_mechanical(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            key,
            op,
            scope,
            current,
            depth,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        if op == "planetary_gears" && depth != 1 {
            return Err(fail(
                "mechanism_scope",
                "Planetary gears must be the document root; export or edit individual parts separately.",
            ));
        }
        let mut options = json!({});
        for (k, v) in item.as_object().unwrap() {
            if k == "id" || k == "op" {
                continue;
            }
            if v.is_boolean() {
                options[k] = v.clone();
                continue;
            }
            let dim = if [
                "module",
                "thickness",
                "bore",
                "backlash",
                "clearance",
                "rim_width",
                "diameter",
                "pitch",
                "length",
                "wall",
            ]
            .contains(&k.as_str())
            {
                LENGTH
            } else if ["pressure_angle", "carrier_angle", "helix_angle"].contains(&k.as_str()) {
                ANGLE
            } else {
                SCALAR
            };
            options[k] = json!(self.value(v, scope, &current, k, dim)?);
        }
        let generated = match op {
            "gear" => mechanical_core::gear(&options, &current)?,
            "thread" => mechanical_core::thread(&options, &current)?,
            _ => mechanical_core::planetary(&options, &current)?,
        };
        self.mechanical_characters += generated.source.chars().count();
        if self.mechanical_characters > 220000 {
            return Err(fail(
                "mechanical_limit",
                "Generated mechanical source exceeds 220000 characters; reduce resolution or instances.",
            ));
        }
        let mut report = generated.report;
        report["node_id"] = json!(key);
        report["instance_path"] = json!(current);
        self.mechanical_reports.push(report);
        self.mechanical_parts.extend(generated.parts);
        self.lines
            .extend(generated.source.split('\n').map(str::to_owned));
        Ok(())
    }
}
