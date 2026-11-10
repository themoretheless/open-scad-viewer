//! Global admission for the Cartesian-product extrusion family only.
//! A bounded Jordan region (CCW outer, disjoint contained CW holes) times a
//! positive interval has an embedded, outward-oriented closed boundary. Plane
//! caps are injective charts; regular Jordan arcs times height are injective
//! ruled patches, meeting only along the authored boundary. This is not a
//! general NURBS shell or source/result material-inclusion certificate.
use super::*;
#[derive(Clone, Debug)]
pub struct Solid {
    shell: ShellGeometry,
    nesting: nesting::Nesting,
    model: Topology,
    jordan_limits: jordan::Limits,
    nesting_limits: nesting::Limits,
}
impl ShellGeometry {
    pub fn qualify_solid(
        &self,
        jordan_limits: jordan::Limits,
        nesting_limits: nesting::Limits,
    ) -> Result<Solid> {
        let jordan = self.qualify_profile_jordan(jordan_limits)?;
        let nesting = nesting::qualify(&jordan, nesting_limits)?;
        let mut model = self.model.clone();
        if model.shells.len() != 1 || !model.shells[0].closed || !model.bodies.is_empty() {
            return Err(refused("Unexpected exact extrusion shell roles"));
        }
        model.bodies.push(brep_topology::Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        validate(&model)?;
        Ok(Solid {
            shell: self.clone(),
            nesting,
            model,
            jordan_limits,
            nesting_limits,
        })
    }
}
impl Solid {
    pub fn model(&self) -> &Topology {
        &self.model
    }
    pub fn shell_geometry(&self) -> &ShellGeometry {
        &self.shell
    }
    pub fn nesting(&self) -> &nesting::Nesting {
        &self.nesting
    }
    pub fn reverify(&self) -> Result<()> {
        self.nesting.reverify()?;
        let expected = self
            .shell
            .qualify_solid(self.jordan_limits, self.nesting_limits)?;
        let a = self.nesting.jordan().sources();
        let b = expected.nesting.jordan().sources();
        if a.len() != b.len()
            || a.iter().zip(b).any(|(a, b)| a.pieces() != b.pieces())
            || self.model != expected.model
            || self.nesting.orientations() != expected.nesting.orientations()
            || self.nesting.containment() != expected.nesting.containment()
            || self.nesting.ray_cells() != expected.nesting.ray_cells()
            || self.nesting.jordan().pairs() != expected.nesting.jordan().pairs()
        {
            return Err(refused(
                "Exact solid source, role or nesting replay changed",
            ));
        }
        Ok(())
    }
}
