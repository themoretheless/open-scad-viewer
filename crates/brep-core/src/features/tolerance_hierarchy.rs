//! Scene-scaled tolerance hierarchy (nurbs-priorities 185, 244).
//!
//! Levels, from tightest to loosest, all in millimetres:
//! * representation: `diagonal * REPRESENTATION_RELATIVE` (floating-point
//!   noise of coordinates at this scene scale);
//! * geometric (scene): `clamp(diagonal * relative, floor, ceiling)`, optionally
//!   widened by an authored document tolerance;
//! * body (model): `max(geometric, model.tolerance_mm)`;
//! * entity (vertex/edge/face): `max(body, local override)`.
//!
//! Every level inherits from its parent and only widens it: an authored model or
//! entity tolerance is never silently tightened. This is the one place that
//! computes effective tolerances for operation contracts.
use crate::{Model, TopoId};
use nurbs_core::{Error, Result};

/// Relative factor of the representation level (about 4.5 ulp of 1.0).
pub const REPRESENTATION_RELATIVE: f64 = 1e-15;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TolerancePolicy {
    /// Geometric tolerance as a fraction of the scene bounding-box diagonal.
    pub relative_to_diagonal: f64,
    pub floor_mm: f64,
    pub ceiling_mm: f64,
}

impl Default for TolerancePolicy {
    fn default() -> Self {
        // ~1e-9 * scale per the strategy note; the floor matches the legacy
        // primitive tolerance, the ceiling keeps healing from welding features.
        Self {
            relative_to_diagonal: 1e-9,
            floor_mm: 1e-7,
            ceiling_mm: 1e-2,
        }
    }
}

impl TolerancePolicy {
    fn validate(&self) -> Result<()> {
        let ok = self.relative_to_diagonal.is_finite()
            && self.relative_to_diagonal >= 0.
            && self.floor_mm.is_finite()
            && self.floor_mm > 0.
            && self.ceiling_mm.is_finite()
            && self.ceiling_mm >= self.floor_mm;
        if ok {
            Ok(())
        } else {
            Err(Error::new(
                "BREP_TOLERANCE_HIERARCHY_INVALID",
                "Tolerance policy needs finite relative >= 0 and 0 < floor <= ceiling",
            ))
        }
    }
}

/// Document/scene level of the hierarchy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneTolerance {
    pub diagonal_mm: f64,
    pub representation_mm: f64,
    pub geometric_mm: f64,
}

impl SceneTolerance {
    /// `document_mm` is an optional authored document tolerance; it widens the
    /// scaled geometric level but never tightens it.
    pub fn from_diagonal(
        diagonal_mm: f64,
        document_mm: Option<f64>,
        policy: TolerancePolicy,
    ) -> Result<Self> {
        policy.validate()?;
        if !diagonal_mm.is_finite() || diagonal_mm < 0. {
            return Err(Error::new(
                "BREP_TOLERANCE_HIERARCHY_INVALID",
                "Scene diagonal must be finite and non-negative",
            ));
        }
        if let Some(value) = document_mm
            && (!value.is_finite() || value <= 0.)
        {
            return Err(Error::new(
                "BREP_TOLERANCE_HIERARCHY_INVALID",
                "Document tolerance must be finite and positive",
            ));
        }
        let scaled = (diagonal_mm * policy.relative_to_diagonal)
            .clamp(policy.floor_mm, policy.ceiling_mm);
        Ok(Self {
            diagonal_mm,
            representation_mm: diagonal_mm * REPRESENTATION_RELATIVE,
            geometric_mm: scaled.max(document_mm.unwrap_or(0.)),
        })
    }

    pub fn from_models(
        models: &[&Model],
        document_mm: Option<f64>,
        policy: TolerancePolicy,
    ) -> Result<Self> {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for model in models {
            for vertex in &model.vertices {
                for i in 0..3 {
                    min[i] = min[i].min(vertex.point[i]);
                    max[i] = max[i].max(vertex.point[i]);
                }
            }
        }
        let diagonal = if min[0] <= max[0] {
            (0..3).map(|i| (max[i] - min[i]).powi(2)).sum::<f64>().sqrt()
        } else {
            0.
        };
        Self::from_diagonal(diagonal, document_mm, policy)
    }

    /// Body level for one model in this scene.
    pub fn body(&self, model: &Model) -> Result<BodyTolerance> {
        if !model.tolerance_mm.is_finite() || model.tolerance_mm <= 0. {
            return Err(Error::new(
                "BREP_TOLERANCE_HIERARCHY_INVALID",
                "Model tolerance must be finite and positive",
            ));
        }
        Ok(BodyTolerance {
            scene: *self,
            body_mm: self.geometric_mm.max(model.tolerance_mm),
        })
    }
}

/// Body level; resolves entity tolerances by inheritance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyTolerance {
    pub scene: SceneTolerance,
    pub body_mm: f64,
}

impl BodyTolerance {
    /// Single-model scene with the default policy.
    pub fn of(model: &Model) -> Result<Self> {
        SceneTolerance::from_models(&[model], None, TolerancePolicy::default())?.body(model)
    }

    fn entity(&self, model: &Model, id: Option<&TopoId>) -> f64 {
        id.and_then(|id| model.1.local_tolerances_mm.get(id))
            .filter(|value| value.is_finite())
            .map_or(self.body_mm, |local| self.body_mm.max(*local))
    }
    pub fn vertex_mm(&self, model: &Model, vertex: usize) -> f64 {
        self.entity(model, model.1.vertices.get(vertex))
    }
    pub fn edge_mm(&self, model: &Model, edge: usize) -> f64 {
        let own = self.entity(model, model.1.edges.get(edge));
        // An edge cannot be tighter than its bounding vertices.
        model.edges.get(edge).map_or(own, |e| {
            e.vertices
                .iter()
                .fold(own, |acc, &v| acc.max(self.vertex_mm(model, v)))
        })
    }
    pub fn face_mm(&self, model: &Model, face: usize) -> f64 {
        self.entity(model, model.1.faces.get(face))
    }
    /// Loosest entity tolerance in the model (the contract-level bound).
    pub fn max_entity_mm(&self, model: &Model) -> f64 {
        (0..model.edges.len())
            .map(|edge| self.edge_mm(model, edge))
            .chain((0..model.faces.len()).map(|face| self.face_mm(model, face)))
            .fold(self.body_mm, f64::max)
    }
}

/// Document length unit. Internal kernel geometry is always millimetres;
/// the unit is document metadata applied at import/export and display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthUnit {
    Millimetre,
    Centimetre,
    Metre,
    Inch,
}

impl LengthUnit {
    /// Exact factors (inch = 25.4 mm by definition).
    pub fn mm_per_unit(self) -> f64 {
        match self {
            Self::Millimetre => 1.,
            Self::Centimetre => 10.,
            Self::Metre => 1000.,
            Self::Inch => 25.4,
        }
    }
    pub fn to_mm(self, value: f64) -> f64 {
        value * self.mm_per_unit()
    }
    pub fn from_mm(self, value_mm: f64) -> f64 {
        value_mm / self.mm_per_unit()
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Millimetre => "mm",
            Self::Centimetre => "cm",
            Self::Metre => "m",
            Self::Inch => "in",
        }
    }
    pub fn parse(code: &str) -> Result<Self> {
        Ok(match code {
            "mm" => Self::Millimetre,
            "cm" => Self::Centimetre,
            "m" => Self::Metre,
            "in" => Self::Inch,
            _ => {
                return Err(Error::new(
                    "BREP_LENGTH_UNIT_INVALID",
                    "Length unit must be mm, cm, m or in",
                ));
            }
        })
    }
    /// Exact-as-possible factor converting coordinates from `self` to `target`.
    pub fn factor_to(self, target: Self) -> f64 {
        if self == target {
            1.
        } else {
            self.mm_per_unit() / target.mm_per_unit()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometric_level_scales_with_diagonal_and_clamps() {
        let p = TolerancePolicy::default();
        let small = SceneTolerance::from_diagonal(1., None, p).unwrap();
        assert_eq!(small.geometric_mm, p.floor_mm);
        let mid = SceneTolerance::from_diagonal(1e6, None, p).unwrap();
        assert!((mid.geometric_mm - 1e-3).abs() < 1e-15);
        let huge = SceneTolerance::from_diagonal(1e12, None, p).unwrap();
        assert_eq!(huge.geometric_mm, p.ceiling_mm);
        assert!(mid.representation_mm < mid.geometric_mm);
        // Twice the scene, twice the tolerance in the scaled band.
        let twice = SceneTolerance::from_diagonal(2e6, None, p).unwrap();
        assert!((twice.geometric_mm / mid.geometric_mm - 2.).abs() < 1e-12);
    }

    #[test]
    fn document_tolerance_widens_but_never_tightens() {
        let p = TolerancePolicy::default();
        let wide = SceneTolerance::from_diagonal(1., Some(1e-4), p).unwrap();
        assert_eq!(wide.geometric_mm, 1e-4);
        let tight = SceneTolerance::from_diagonal(1e6, Some(1e-9), p).unwrap();
        assert!((tight.geometric_mm - 1e-3).abs() < 1e-15);
        assert!(SceneTolerance::from_diagonal(1., Some(0.), p).is_err());
        assert!(SceneTolerance::from_diagonal(f64::NAN, None, p).is_err());
        let bad = TolerancePolicy {
            floor_mm: 1.,
            ceiling_mm: 0.5,
            ..p
        };
        assert_eq!(
            SceneTolerance::from_diagonal(1., None, bad).unwrap_err().code,
            "BREP_TOLERANCE_HIERARCHY_INVALID"
        );
    }

    #[test]
    fn body_and_entities_inherit_and_only_widen() {
        let mut model = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let body = BodyTolerance::of(&model).unwrap();
        assert_eq!(body.body_mm, model.tolerance_mm.max(body.scene.geometric_mm));
        assert_eq!(body.vertex_mm(&model, 0), body.body_mm);
        assert_eq!(body.face_mm(&model, 0), body.body_mm);
        // Local override on a vertex widens that vertex and incident edges.
        let v = model.edges[0].vertices[0];
        let id = model.1.vertices[v];
        model.1.local_tolerances_mm.insert(id, 1e-4);
        let body = BodyTolerance::of(&model).unwrap();
        assert_eq!(body.vertex_mm(&model, v), 1e-4);
        assert_eq!(body.edge_mm(&model, 0), 1e-4);
        assert_eq!(body.max_entity_mm(&model), 1e-4);
        // A tighter local override inherits the body value.
        let face = model.1.faces[0];
        model.1.local_tolerances_mm.insert(face, 1e-12);
        assert_eq!(body.face_mm(&model, 0), body.body_mm);
        // Model tolerance wider than the scene level survives.
        model.tolerance_mm = 5e-3;
        assert_eq!(BodyTolerance::of(&model).unwrap().body_mm, 5e-3);
    }

    #[test]
    fn scene_spans_all_models() {
        let a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let b = crate::cuboid([1000.; 3], [1001.; 3]).unwrap();
        let scene =
            SceneTolerance::from_models(&[&a, &b], None, TolerancePolicy::default()).unwrap();
        assert!(scene.diagonal_mm > 1.7e3);
        assert!(scene.body(&a).unwrap().body_mm > 1.7e-6);
        let alone = BodyTolerance::of(&a).unwrap();
        assert_eq!(alone.body_mm, 1e-7);
    }

    #[test]
    fn length_units_are_exact() {
        assert_eq!(LengthUnit::Inch.to_mm(1.), 25.4);
        assert_eq!(LengthUnit::Metre.to_mm(1.5), 1500.);
        assert_eq!(LengthUnit::Centimetre.from_mm(25.), 2.5);
        assert_eq!(LengthUnit::Inch.factor_to(LengthUnit::Millimetre), 25.4);
        assert_eq!(LengthUnit::Metre.factor_to(LengthUnit::Centimetre), 100.);
        for unit in [
            LengthUnit::Millimetre,
            LengthUnit::Centimetre,
            LengthUnit::Metre,
            LengthUnit::Inch,
        ] {
            assert_eq!(LengthUnit::parse(unit.code()).unwrap(), unit);
        }
        assert_eq!(
            LengthUnit::parse("ft").unwrap_err().code,
            "BREP_LENGTH_UNIT_INVALID"
        );
    }
}
