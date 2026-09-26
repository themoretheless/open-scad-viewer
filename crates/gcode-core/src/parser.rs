//! Shared strict-parser core for the preview and job dialects.
//!
//! Both `parse` and `parse_job` walk the same per-line skeleton: filament
//! metadata, layer markers, then absolute G0/G1 motion with a modal feedrate.
//! This module owns that shared motion state; dialect-specific setup,
//! shutdown, and extrusion rules stay in the callers.

use crate::{
    GcodeBounds, GcodeMove, GcodePreview, MAX_COORDINATE_MM, MAX_LAYERS, MAX_MOVES, MoveWords,
    Result, invalid, number, valid_coordinate, valid_dimension,
};

pub(crate) struct MotionParser {
    pub(crate) result: GcodePreview,
    pub(crate) diameter: Option<f64>,
    position: [f64; 3],
    known: [bool; 3],
    feedrate: Option<f64>,
    current_layer_z: Option<f64>,
    previous_layer_z: Option<f64>,
    annotated_z: Option<f64>,
    move_count: usize,
}

impl MotionParser {
    pub(crate) fn new() -> Self {
        Self {
            result: GcodePreview {
                layers: 0,
                extrusion_mm: 0.0,
                deposited_volume_mm3: 0.0,
                moves: Vec::new(),
                bounds: None,
                travel_distance_mm: 0.0,
                print_distance_mm: 0.0,
                estimated_time_s: 0.0,
            },
            diameter: None,
            position: [0.0; 3],
            known: [false; 3],
            feedrate: None,
            current_layer_z: None,
            previous_layer_z: None,
            annotated_z: None,
            move_count: 0,
        }
    }

    /// Handles a `;FILAMENT_DIAMETER_MM:` line; `None` for other lines.
    pub(crate) fn admit_diameter(&mut self, line: &str) -> Option<Result<()>> {
        let text = line.strip_prefix(";FILAMENT_DIAMETER_MM:")?;
        Some((|| {
            if self.diameter.is_some() || self.result.layers != 0 {
                return Err(invalid(
                    "GCODE_METADATA",
                    "Filament diameter must occur once before layers",
                ));
            }
            let value = number(text)?;
            if !valid_dimension(value) {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    "Invalid filament diameter metadata",
                ));
            }
            self.diameter = Some(value);
            Ok(())
        })())
    }

    /// Handles a `;Z:` annotation line; `None` for other lines.
    pub(crate) fn admit_z_annotation(&mut self, line: &str) -> Option<Result<()>> {
        let text = line.strip_prefix(";Z:")?;
        Some((|| {
            let value = number(text)?;
            if self.result.layers == 0
                || self.current_layer_z.is_some()
                || self.annotated_z.is_some()
                || !valid_coordinate(value)
            {
                return Err(invalid(
                    "GCODE_LAYER",
                    "Invalid or misplaced layer Z metadata",
                ));
            }
            self.annotated_z = Some(value);
            Ok(())
        })())
    }

    /// `;LAYER:` marker text (without the prefix). `ready` reports the
    /// caller's dialect-specific setup state; messages keep each dialect's
    /// wording.
    pub(crate) fn admit_layer(
        &mut self,
        text: &str,
        ready: bool,
        prologue_error: &'static str,
        limit_error: &'static str,
    ) -> Result<()> {
        if !ready || self.diameter.is_none() {
            return Err(invalid("GCODE_PROLOGUE", prologue_error));
        }
        if self.result.layers > 0 && self.current_layer_z.is_none() {
            return Err(invalid(
                "GCODE_LAYER",
                "Each layer must establish its Z coordinate",
            ));
        }
        let index = text
            .parse::<usize>()
            .map_err(|_| invalid("GCODE_LAYER", "Invalid layer index"))?;
        if index != self.result.layers {
            return Err(invalid(
                "GCODE_LAYER",
                "Layer indices must start at zero and be consecutive",
            ));
        }
        if self.result.layers == MAX_LAYERS {
            return Err(invalid("GCODE_LAYER_LIMIT", limit_error));
        }
        self.result.layers += 1;
        self.previous_layer_z = self.current_layer_z;
        self.current_layer_z = None;
        self.annotated_z = None;
        Ok(())
    }

    pub(crate) fn count_move(&mut self, limit_error: &'static str) -> Result<()> {
        self.move_count += 1;
        if self.move_count > MAX_MOVES {
            return Err(invalid("GCODE_MOVE_LIMIT", limit_error));
        }
        Ok(())
    }

    pub(crate) fn position(&self) -> [f64; 3] {
        self.position
    }

    pub(crate) fn all_known(&self) -> bool {
        self.known.iter().all(|v| *v)
    }

    pub(crate) fn current_layer_z(&self) -> Option<f64> {
        self.current_layer_z
    }

    /// Validates and applies X/Y/Z words to the tracked position.
    pub(crate) fn apply_position_words(&mut self, words: &MoveWords) -> Result<()> {
        for (index, value) in [words.x, words.y, words.z].into_iter().enumerate() {
            if let Some(value) = value {
                if !valid_coordinate(value) {
                    return Err(invalid(
                        "GCODE_INVALID_COORDINATE",
                        "Move coordinates must be within +/-1000000 mm",
                    ));
                }
                self.position[index] = value;
                self.known[index] = true;
            }
        }
        Ok(())
    }

    /// Enforces the layer-Z contract for a move, after applying its words.
    pub(crate) fn enforce_layer_z(&mut self, words: &MoveWords) -> Result<()> {
        if let Some(layer_z) = self.current_layer_z {
            if words.z.is_some() && self.position[2] != layer_z {
                return Err(invalid(
                    "GCODE_LAYER",
                    "Z changes require a new layer marker",
                ));
            }
            return Ok(());
        }
        let z = words.z.ok_or_else(|| {
            invalid("GCODE_LAYER", "The first move in each layer must set Z")
        })?;
        if self.previous_layer_z.is_some_and(|previous| z <= previous)
            || self.annotated_z.is_some_and(|annotation| z != annotation)
        {
            return Err(invalid(
                "GCODE_LAYER",
                "Layer Z must increase and match its annotation",
            ));
        }
        self.current_layer_z = Some(z);
        Ok(())
    }

    /// Validates and applies the F word; returns the established feedrate in mm/s.
    pub(crate) fn admit_feedrate(&mut self, words: &MoveWords) -> Result<f64> {
        if let Some(value) = words.f {
            if value < 0.001 || value > MAX_COORDINATE_MM * 60.0 {
                return Err(invalid(
                    "GCODE_INVALID_FEEDRATE",
                    "Feedrate F must be 0.001..60000000 mm/min",
                ));
            }
            self.feedrate = Some(value / 60.0);
        }
        self.feedrate.ok_or_else(|| {
            invalid(
                "GCODE_INVALID_FEEDRATE",
                "A move requires an established positive feedrate",
            )
        })
    }

    /// 3D distance from a previous position (zero until all axes are known).
    pub(crate) fn distance_from(&self, old_position: [f64; 3], was_known: bool) -> f64 {
        if was_known {
            (self.position[0] - old_position[0])
                .hypot(self.position[1] - old_position[1])
                .hypot(self.position[2] - old_position[2])
        } else {
            0.0
        }
    }

    /// Accumulates distance/time totals, bounds, and the move record.
    pub(crate) fn record_motion(
        &mut self,
        old_position: [f64; 3],
        was_known: bool,
        feedrate_mm_s: f64,
        e: f64,
        extruded: bool,
    ) {
        let distance = self.distance_from(old_position, was_known);
        if extruded {
            self.result.print_distance_mm += distance;
        } else {
            self.result.travel_distance_mm += distance;
        }
        self.result.estimated_time_s += distance / feedrate_mm_s;
        if let Some(bounds) = &mut self.result.bounds {
            for (axis, value) in self.position.iter().enumerate() {
                bounds.min[axis] = bounds.min[axis].min(*value);
                bounds.max[axis] = bounds.max[axis].max(*value);
            }
        } else {
            self.result.bounds = Some(GcodeBounds {
                min: self.position,
                max: self.position,
            });
        }
        self.result.moves.push(GcodeMove {
            x: self.position[0],
            y: self.position[1],
            z: self.position[2],
            e,
            feedrate_mm_s,
            layer_index: self.result.layers - 1,
            extruded,
        });
    }

    /// Final layer/totals validation and deposited-volume computation.
    pub(crate) fn finish(
        mut self,
        extrusion_mm: f64,
        totals_error: &'static str,
    ) -> Result<GcodePreview> {
        if self.result.layers > 0 && self.current_layer_z.is_none() {
            return Err(invalid("GCODE_LAYER", "Final layer has no Z coordinate"));
        }
        let filament_area = std::f64::consts::PI * (self.diameter.unwrap_or(0.0) / 2.0).powi(2);
        self.result.extrusion_mm = extrusion_mm;
        self.result.deposited_volume_mm3 = extrusion_mm * filament_area;
        if ![
            self.result.extrusion_mm,
            self.result.deposited_volume_mm3,
            self.result.travel_distance_mm,
            self.result.print_distance_mm,
            self.result.estimated_time_s,
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Err(invalid("GCODE_NUMERIC", totals_error));
        }
        Ok(self.result)
    }
}
