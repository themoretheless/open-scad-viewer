//! Exact material selection after complete strict root-boundary relations.
use super::*;
use crate::closed_solid::BooleanMaterialOp;
mod policy;
use policy::Selection;
enum Payload {
    Empty,
    Source(usize),
    Forest(QualifiedRootShellForest),
}
pub struct QualifiedRootBoolean {
    relation: Arc<QualifiedRootRelation>,
    operation: BooleanMaterialOp,
    swapped: bool,
    selection: Selection,
    payload: Payload,
    lineage: [Option<crate::mixed_root_model::MixedTopologyMap>; 2],
    volume: [f64; 2],
    absolute_error: f64,
}
impl QualifiedRootBoolean {
    pub fn qualify(
        relation: Arc<QualifiedRootRelation>,
        operation: BooleanMaterialOp,
        absolute_error: f64,
    ) -> Result<Self> {
        Self::build(relation, operation, false, absolute_error)
    }
    pub fn qualify_swapped(
        relation: Arc<QualifiedRootRelation>,
        operation: BooleanMaterialOp,
        absolute_error: f64,
    ) -> Result<Self> {
        Self::build(relation, operation, true, absolute_error)
    }
    fn build(
        relation: Arc<QualifiedRootRelation>,
        operation: BooleanMaterialOp,
        swapped: bool,
        absolute_error: f64,
    ) -> Result<Self> {
        if !absolute_error.is_finite() || absolute_error <= 0. {
            return Err(unproved("Invalid root Boolean result accuracy"));
        }
        let mut source = relation.sources();
        if swapped {
            source.swap(0, 1);
        }
        let kind = if swapped {
            policy::reverse(relation.kind())
        } else {
            relation.kind()
        };
        let selection = policy::select(kind, operation);
        let (payload, lineage, volume) = match selection {
            Selection::Empty => (Payload::Empty, [None, None], [0.; 2]),
            Selection::First | Selection::Second => {
                let i = usize::from(selection == Selection::Second);
                let v = source[i].volume();
                if v[0] <= 0. || I::point(v[1]).sub(I::point(v[0]))?.hi > absolute_error {
                    return Err(unproved(
                        "Selected root Boolean material volume accuracy unproved",
                    ));
                }
                let mut maps = [None, None];
                maps[i] = Some(source[i].model().identity_map()?);
                (Payload::Source(i), maps, v)
            }
            Selection::Both => {
                let forest = QualifiedRootShellForest::qualify(
                    vec![source[0].clone(), source[1].clone()],
                    vec![relation.clone()],
                    absolute_error,
                )?;
                let v = forest.volume();
                let maps = [
                    Some(forest.lineage()[0].clone()),
                    Some(forest.lineage()[1].clone()),
                ];
                (Payload::Forest(forest), maps, v)
            }
        };
        Ok(Self {
            relation,
            operation,
            swapped,
            selection,
            payload,
            lineage,
            volume,
            absolute_error,
        })
    }
    pub fn operands(&self) -> [&Arc<QualifiedPairedRootSolid>; 2] {
        let mut source = self.relation.sources();
        if self.swapped {
            source.swap(0, 1);
        }
        source
    }
    pub fn operation(&self) -> BooleanMaterialOp {
        self.operation
    }
    pub fn is_empty(&self) -> bool {
        matches!(self.payload, Payload::Empty)
    }
    pub fn material_admitted(&self) -> bool {
        true
    }
    pub fn volume(&self) -> [f64; 2] {
        self.volume
    }
    pub fn lineage(&self) -> [Option<&crate::mixed_root_model::MixedTopologyMap>; 2] {
        [self.lineage[0].as_ref(), self.lineage[1].as_ref()]
    }
    pub fn model(&self) -> Option<&MixedBrep> {
        match &self.payload {
            Payload::Empty => None,
            Payload::Source(i) => Some(self.operands()[*i].model()),
            Payload::Forest(f) => Some(f.model()),
        }
    }
    /// Empty results classify every finite point outside without ray work.
    /// Other results query only retained boundaries; discarded source boundaries
    /// do not introduce unresolved boundary hits into the result.
    pub fn classify_point(
        &self,
        point: [f64; 3],
        directions: &[[f64; 3]],
        limits: RootSolidPointLimits,
    ) -> Result<Option<bool>> {
        if !point.iter().all(|x| x.is_finite()) {
            return Err(unproved("Invalid root Boolean point"));
        }
        match &self.payload {
            Payload::Empty => Ok(Some(false)),
            Payload::Source(i) => Ok(self.operands()[*i]
                .classify_point(point, directions, limits)?
                .parity),
            Payload::Forest(f) => f.classify_point(point, directions, limits),
        }
    }
    pub fn reverify(&self) -> Result<()> {
        for (map, source) in self.lineage().into_iter().zip(self.operands()) {
            if let Some(map) = map {
                if !map.source_matches(source.model())
                    || !self.model().is_some_and(|m| map.target_matches(m))
                {
                    return Err(unproved("Root Boolean lineage belongs to another snapshot"));
                }
            }
        }
        self.relation.reverify()?;
        let fresh = Self::build(
            self.relation.clone(),
            self.operation,
            self.swapped,
            self.absolute_error,
        )?;
        if fresh.selection != self.selection
            || fresh.volume != self.volume
            || !fresh
                .lineage
                .iter()
                .zip(&self.lineage)
                .all(|(a, b)| match (a, b) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.same_mapping(b),
                    _ => false,
                })
        {
            return Err(unproved(
                "Root Boolean material selection, volume or lineage changed",
            ));
        }
        Ok(())
    }
}
