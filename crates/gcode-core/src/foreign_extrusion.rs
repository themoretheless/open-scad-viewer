//! Positive material advance for the tolerant reader; not net physical deposition.
use crate::{MAX_COORDINATE_MM, Result, invalid};

const MAX_TOOLS: usize = 256;

#[derive(Clone, Copy)]
struct Tool {
    id: u32,
    area_mm2: f64,
    flow: f64,
}

pub(super) struct Extrusion {
    // One logical E coordinate, independent of flow and active tool parameters.
    position: f64,
    volumetric: bool,
    active: usize,
    tools: Vec<Tool>,
    initial_area_mm2: f64,
    filament_per_e: f64,
    volume_per_e: f64,
    pub total_mm: f64,
    pub volume_mm3: f64,
}

fn area(diameter_mm: f64) -> Result<f64> {
    let area = std::f64::consts::PI * (diameter_mm / 2.0).powi(2);
    if !(diameter_mm.is_finite()
        && diameter_mm > 0.0
        && diameter_mm <= MAX_COORDINATE_MM
        && area > 0.0
        && (1.0 / area).is_finite())
    {
        return Err(invalid(
            "GCODE_INVALID_SETTINGS",
            "Filament diameter must be positive, at most 1000000 mm, with a representable area and reciprocal",
        ));
    }
    Ok(area)
}

fn numeric(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid(
            "GCODE_NUMERIC",
            "Extrusion exceeds numeric precision",
        ))
    }
}

impl Extrusion {
    pub fn new(diameter_mm: f64) -> Result<Self> {
        let area_mm2 = area(diameter_mm)?;
        Ok(Self {
            position: 0.0,
            volumetric: false,
            active: 0,
            tools: vec![Tool {
                id: 0,
                area_mm2,
                flow: 1.0,
            }],
            initial_area_mm2: area_mm2,
            filament_per_e: 1.0,
            volume_per_e: area_mm2,
            total_mm: 0.0,
            volume_mm3: 0.0,
        })
    }

    fn tool(&mut self, target: Option<f64>) -> Result<usize> {
        let Some(target) = target else {
            return Ok(self.active);
        };
        if !(target.is_finite()
            && target >= 0.0
            && target <= u32::MAX as f64
            && target.fract() == 0.0)
        {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "Extruder identifier must be an unsigned 32-bit integer",
            ));
        }
        let id = target as u32;
        if let Some(index) = self.tools.iter().position(|tool| tool.id == id) {
            return Ok(index);
        }
        if self.tools.len() == MAX_TOOLS {
            return Err(invalid(
                "GCODE_TOOL_LIMIT",
                "G-code exceeded 256 distinct extruder identifiers",
            ));
        }
        self.tools.push(Tool {
            id,
            area_mm2: self.initial_area_mm2,
            flow: 1.0,
        });
        Ok(self.tools.len() - 1)
    }

    fn refresh(&mut self) -> Result<()> {
        let tool = self.tools[self.active];
        self.filament_per_e = if self.volumetric {
            tool.flow / tool.area_mm2
        } else {
            tool.flow
        };
        self.volume_per_e = if self.volumetric {
            tool.flow
        } else {
            tool.flow * tool.area_mm2
        };
        numeric(self.filament_per_e)?;
        numeric(self.volume_per_e)?;
        if tool.flow > 0.0 && (self.filament_per_e == 0.0 || self.volume_per_e == 0.0) {
            return Err(invalid(
                "GCODE_NUMERIC",
                "Extrusion scale underflows numeric precision",
            ));
        }
        Ok(())
    }

    pub fn select_tool(&mut self, target: u32) -> Result<()> {
        self.active = self.tool(Some(target as f64))?;
        self.refresh()
    }

    pub fn configure_volume(
        &mut self,
        diameter: Option<f64>,
        enabled: Option<f64>,
        target: Option<f64>,
        linear_scale: f64,
    ) -> Result<()> {
        let tool = self.tool(target)?;
        if enabled.is_some_and(|s| s != 0.0 && s != 1.0) {
            return Err(invalid("GCODE_INVALID_SETTINGS", "M200 S must be 0 or 1"));
        }
        if let Some(d) = diameter
            && d != 0.0
        {
            self.tools[tool].area_mm2 = area(d * linear_scale)?;
            self.volumetric = true;
        }
        if let Some(s) = enabled {
            self.volumetric = s == 1.0;
        }
        // D0 disables even with S1, but keeps the last nonzero diameter.
        if diameter == Some(0.0) {
            self.volumetric = false;
        }
        self.refresh()
    }

    pub fn configure_flow(&mut self, percent: Option<f64>, target: Option<f64>) -> Result<()> {
        let tool = self.tool(target)?;
        if let Some(percent) = percent {
            if !percent.is_finite() || percent < 0.0 {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    "Flow percentage must be finite and nonnegative",
                ));
            }
            let flow = percent / 100.0;
            if percent > 0.0 && flow == 0.0 {
                return Err(invalid(
                    "GCODE_NUMERIC",
                    "Flow percentage underflows numeric precision",
                ));
            }
            self.tools[tool].flow = flow;
        }
        self.refresh()
    }

    fn input_scale(&self, linear_scale: f64) -> f64 {
        if self.volumetric {
            linear_scale.powi(3)
        } else {
            linear_scale
        }
    }

    pub fn reset(&mut self, value: f64, linear_scale: f64) -> Result<()> {
        self.position = numeric(value * self.input_scale(linear_scale))?;
        Ok(())
    }

    /// Retractions move logical E but do not subtract from positive-advance totals.
    pub fn advance(
        &mut self,
        value: Option<f64>,
        relative: bool,
        linear_scale: f64,
    ) -> Result<bool> {
        let Some(value) = value else { return Ok(false) };
        let scaled = numeric(value * self.input_scale(linear_scale))?;
        let next = numeric(if relative {
            self.position + scaled
        } else {
            scaled
        })?;
        let delta = numeric(if relative {
            scaled
        } else {
            next - self.position
        })?;
        self.position = next;
        if delta <= 0.0 || self.filament_per_e == 0.0 {
            return Ok(false);
        }
        let length = numeric(delta * self.filament_per_e)?;
        let volume = numeric(delta * self.volume_per_e)?;
        if length == 0.0 || volume == 0.0 {
            return Err(invalid(
                "GCODE_NUMERIC",
                "Positive extrusion underflows numeric precision",
            ));
        }
        self.total_mm = numeric(self.total_mm + length)?;
        self.volume_mm3 = numeric(self.volume_mm3 + volume)?;
        Ok(true)
    }
}
