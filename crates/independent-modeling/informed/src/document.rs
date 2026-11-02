use crate::{Error, Operation, Result, operation::execute};
use modeling_clean::{
    boolean,
    curve::Curve,
    math::{Bounds, Point},
    mesh::Mesh,
    surface::Surface,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

static SERIAL: AtomicU64 = AtomicU64::new(1);
pub(crate) fn next_key() -> Result<u64> {
    SERIAL
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::Limit("identity counter exhausted"))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId {
    pub(crate) document: u64,
    pub(crate) serial: u64,
}
impl EntityId {
    pub fn serial(self) -> u64 {
        self.serial
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Curve(Curve),
    Surface(Surface),
    Mesh(Mesh),
    BoxCells(Vec<Bounds>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryKind {
    Curve,
    Surface,
    FacetedMesh,
    AxisAlignedBoxCells,
}
impl Geometry {
    pub fn kind(&self) -> GeometryKind {
        match self {
            Self::Curve(_) => GeometryKind::Curve,
            Self::Surface(_) => GeometryKind::Surface,
            Self::Mesh(_) => GeometryKind::FacetedMesh,
            Self::BoxCells(_) => GeometryKind::AxisAlignedBoxCells,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Entity {
    pub(crate) id: EntityId,
    pub(crate) content: u64,
    pub(crate) geometry: Arc<Geometry>,
}
impl Entity {
    pub fn id(&self) -> EntityId {
        self.id
    }
    pub fn content_id(&self) -> u64 {
        self.content
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub(crate) entities: BTreeMap<EntityId, Entity>,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    document: u64,
    revision: u64,
    state: State,
}
impl Snapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.state.entities.get(&id)
    }
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.state.entities.values()
    }
}
#[derive(Clone, Debug)]
struct Change {
    before: State,
    after: State,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub revision: u64,
    /// Unique surviving operation outputs, in first-output order.
    pub entities: Vec<EntityId>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Display {
    Polyline(Vec<Point>),
    Triangles(Mesh),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resolution {
    pub curve_segments: usize,
    pub surface_segments: [usize; 2],
}
impl Default for Resolution {
    fn default() -> Self {
        Self {
            curve_segments: 64,
            surface_segments: [16, 32],
        }
    }
}

pub struct Document {
    id: u64,
    revision: u64,
    state: State,
    past: Vec<Change>,
    future: Vec<Change>,
    history_limit: usize,
    cache: BTreeMap<(EntityId, u64, Resolution), Arc<Display>>,
}
impl Document {
    pub fn new() -> Result<Self> {
        Ok(Self {
            id: next_key()?,
            revision: 0,
            state: State::default(),
            past: vec![],
            future: vec![],
            history_limit: 64,
            cache: BTreeMap::new(),
        })
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.state.entities.get(&id)
    }
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.state.entities.values()
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            document: self.id,
            revision: self.revision,
            state: self.state.clone(),
        }
    }
    pub fn set_history_limit(&mut self, limit: usize) -> Result<()> {
        if limit > 1024 {
            return Err(Error::Limit("history limit must be <=1024"));
        }
        self.history_limit = limit;
        if self.past.len() > limit {
            self.past.drain(..self.past.len() - limit);
        }
        self.future.clear();
        Ok(())
    }
    pub fn transaction(&mut self) -> Transaction<'_> {
        let staged = self.state.clone();
        Transaction {
            document: self,
            staged,
            ids: vec![],
            failed: false,
            operations: 0,
        }
    }
    pub fn apply(&mut self, operation: Operation) -> Result<Receipt> {
        let mut transaction = self.transaction();
        transaction.apply(operation)?;
        transaction.commit()
    }
    fn install(&mut self, state: State, mut ids: Vec<EntityId>) -> Result<Receipt> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::Limit("revision counter exhausted"))?;
        let mut seen = BTreeSet::new();
        ids.retain(|id| state.entities.contains_key(id) && seen.insert(*id));
        if self.history_limit > 0 {
            self.past.push(Change {
                before: self.state.clone(),
                after: state.clone(),
            });
            if self.past.len() > self.history_limit {
                self.past.remove(0);
            }
        }
        self.state = state;
        self.revision = revision;
        self.future.clear();
        self.cache.clear();
        Ok(Receipt {
            revision,
            entities: ids,
        })
    }
    pub fn undo(&mut self) -> Result<bool> {
        if self.past.is_empty() {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::Limit("revision counter exhausted"))?;
        let change = self.past.pop().unwrap();
        self.state = change.before.clone();
        self.future.push(change);
        self.revision = revision;
        self.cache.clear();
        Ok(true)
    }
    pub fn redo(&mut self) -> Result<bool> {
        if self.future.is_empty() {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::Limit("revision counter exhausted"))?;
        let change = self.future.pop().unwrap();
        self.state = change.after.clone();
        self.past.push(change);
        self.revision = revision;
        self.cache.clear();
        Ok(true)
    }
    /// Owned immutable job input; compute on a caller-chosen thread. Geometry
    /// work never mutates the live document. Publication rechecks the revision.
    pub fn prepare(&self, operation: Operation) -> Job {
        Job {
            snapshot: self.snapshot(),
            operation,
            cancellation: Cancellation::default(),
        }
    }
    pub fn publish(&mut self, computed: Computed) -> std::result::Result<Receipt, PublishError> {
        let reason = if computed.document != self.id {
            Some(Error::WrongDocument)
        } else if computed.cancellation.is_cancelled() {
            Some(Error::Cancelled)
        } else if computed.base_revision != self.revision {
            Some(Error::StaleRevision {
                expected: computed.base_revision,
                actual: self.revision,
            })
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(PublishError { reason, computed });
        }
        if self.revision == u64::MAX {
            return Err(PublishError {
                reason: Error::Limit("revision counter exhausted"),
                computed,
            });
        }
        // install cannot fail after the explicit counter check above.
        Ok(self
            .install(computed.state, computed.ids)
            .expect("checked revision"))
    }
    /// Separate derived display cache; source geometry and history remain intact.
    pub fn display(&mut self, id: EntityId, resolution: Resolution) -> Result<Arc<Display>> {
        let entity = self
            .state
            .entities
            .get(&id)
            .ok_or(Error::MissingEntity(id))?;
        let key = (id, entity.content, resolution);
        if let Some(value) = self.cache.get(&key) {
            return Ok(Arc::clone(value));
        }
        let display = match entity.geometry() {
            Geometry::Curve(c) => Display::Polyline(c.sample(resolution.curve_segments)?),
            Geometry::Surface(s) => Display::Triangles(s.tessellate(resolution.surface_segments)?),
            Geometry::Mesh(m) => Display::Triangles(m.clone()),
            Geometry::BoxCells(c) => Display::Triangles(boolean::mesh_cells(c)?),
        };
        if self.cache.len() >= 64 {
            self.cache.clear();
        }
        let value = Arc::new(display);
        self.cache.insert(key, Arc::clone(&value));
        Ok(value)
    }
}

/// Drop rolls back. Any failed operation poisons the whole transaction, so a
/// partially successful batch cannot accidentally be committed.
pub struct Transaction<'a> {
    document: &'a mut Document,
    staged: State,
    ids: Vec<EntityId>,
    failed: bool,
    operations: usize,
}
impl Transaction<'_> {
    pub fn apply(&mut self, operation: Operation) -> Result<Vec<EntityId>> {
        if self.failed {
            return Err(Error::FailedTransaction);
        }
        match execute(&mut self.staged, self.document.id, operation) {
            Ok(ids) => {
                self.ids.extend_from_slice(&ids);
                self.operations += 1;
                Ok(ids)
            }
            Err(e) => {
                self.failed = true;
                Err(e)
            }
        }
    }
    pub fn preview(&self) -> Snapshot {
        Snapshot {
            document: self.document.id,
            revision: self.document.revision,
            state: self.staged.clone(),
        }
    }
    pub fn commit(self) -> Result<Receipt> {
        if self.failed {
            return Err(Error::FailedTransaction);
        }
        if self.operations == 0 {
            return Ok(Receipt {
                revision: self.document.revision,
                entities: vec![],
            });
        }
        self.document.install(self.staged, self.ids)
    }
}
#[derive(Clone, Default, Debug)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
pub struct Job {
    snapshot: Snapshot,
    operation: Operation,
    cancellation: Cancellation,
}
impl Job {
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
    /// Cancellation is checked before/after a whole geometry operation, not
    /// inside clean-kernel loops. It prevents publication, not preemptive work.
    pub fn run(self) -> Result<Computed> {
        if self.cancellation.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let mut state = self.snapshot.state;
        let ids = execute(&mut state, self.snapshot.document, self.operation)?;
        if self.cancellation.is_cancelled() {
            return Err(Error::Cancelled);
        }
        Ok(Computed {
            document: self.snapshot.document,
            base_revision: self.snapshot.revision,
            state,
            ids,
            cancellation: self.cancellation,
        })
    }
}
#[derive(Debug)]
pub struct Computed {
    document: u64,
    base_revision: u64,
    state: State,
    ids: Vec<EntityId>,
    cancellation: Cancellation,
}
impl Computed {
    pub fn base_revision(&self) -> u64 {
        self.base_revision
    }
    pub fn preview(&self) -> Snapshot {
        Snapshot {
            document: self.document,
            revision: self.base_revision,
            state: self.state.clone(),
        }
    }
    pub fn entities(&self) -> &[EntityId] {
        &self.ids
    }
}
/// Failed publication retains the complete computed result for inspection.
#[derive(Debug)]
pub struct PublishError {
    pub reason: Error,
    pub computed: Computed,
}
impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}
impl std::error::Error for PublishError {}
