//! Renderer-owned bounded retention. Draws hold Arc leases so eviction cannot
//! invalidate an already prepared command. In-flight leases are additional to
//! the retention budget and are released by the frame owner after submission.
use crate::{ResidentMesh, Vertex};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Immutable geometry/material snapshot. A replacement gets a new identity,
/// so edits and undo never accidentally reuse stale GPU bytes.
pub struct MeshData {
    id: u64,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    bytes: u64,
}
impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Result<Arc<Self>, &'static str> {
        if vertices.is_empty()
            || indices.is_empty()
            || !indices.len().is_multiple_of(3)
            || indices.len() > u32::MAX as usize
        {
            return Err("invalid triangle mesh");
        }
        if vertices
            .iter()
            .any(|v| v.position.iter().any(|p| !p.is_finite()))
            || indices.iter().any(|&i| i as usize >= vertices.len())
        {
            return Err("invalid vertex or index");
        }
        let bytes = (vertices.len() as u64)
            .checked_mul(12)
            .and_then(|n| n.checked_add((indices.len() as u64).checked_mul(4)?))
            .ok_or("mesh size overflow")?;
        let id = NEXT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "mesh identity exhausted")?;
        Ok(Arc::new(Self {
            id,
            vertices,
            indices,
            bytes,
        }))
    }
    pub fn byte_size(&self) -> u64 {
        self.bytes
    }
}
#[derive(Clone, Hash, PartialEq, Eq)]
enum Key {
    Mesh(u64),
    Batch(Box<[u64]>),
}
struct Entry {
    mesh: Arc<ResidentMesh>,
    age: u64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub uploads: u64,
    pub uploaded_bytes: u64,
    pub evictions: u64,
}

pub struct GeometryCache {
    entries: HashMap<Key, Entry>,
    ages: BTreeMap<u64, Key>,
    clock: u64,
    bytes: u64,
    max_bytes: u64,
    max_entries: usize,
    stats: CacheStats,
}
impl GeometryCache {
    pub fn new(max_bytes: u64, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            ages: BTreeMap::new(),
            clock: 0,
            bytes: 0,
            max_bytes,
            max_entries,
            stats: CacheStats::default(),
        }
    }
    pub fn retained_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn stats(&self) -> CacheStats {
        self.stats
    }
    /// Called on renderer/device replacement or when its document is closed.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.ages.clear();
        self.bytes = 0;
        self.clock = 0;
    }
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        data: &MeshData,
    ) -> Result<Arc<ResidentMesh>, &'static str> {
        self.prepare_with(Key::Mesh(data.id), data.bytes, || {
            ResidentMesh::upload(device, &data.vertices, &data.indices, data.bytes)
        })
    }
    /// Merge only a contiguous draw sequence with identical camera and clipping.
    /// The full identity list is the key, so hashing never substitutes for equality.
    pub fn prepare_batch(
        &mut self,
        device: &wgpu::Device,
        data: &[Arc<MeshData>],
    ) -> Result<Arc<ResidentMesh>, &'static str> {
        if data.len() == 1 {
            return self.prepare(device, &data[0]);
        }
        if data.is_empty() {
            return Err("empty draw batch");
        }
        let bytes = data
            .iter()
            .try_fold(0u64, |n, d| n.checked_add(d.bytes))
            .ok_or("batch size overflow")?;
        let key = Key::Batch(data.iter().map(|d| d.id).collect());
        self.prepare_with(key, bytes, || {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for mesh in data {
                let base = u32::try_from(vertices.len()).map_err(|_| "batch vertex overflow")?;
                vertices.extend_from_slice(&mesh.vertices);
                for &index in &mesh.indices {
                    indices.push(base.checked_add(index).ok_or("batch index overflow")?);
                }
            }
            ResidentMesh::upload(device, &vertices, &indices, bytes)
        })
    }
    fn prepare_with(
        &mut self,
        key: Key,
        bytes: u64,
        upload: impl FnOnce() -> Result<ResidentMesh, &'static str>,
    ) -> Result<Arc<ResidentMesh>, &'static str> {
        if bytes > self.max_bytes || self.max_entries == 0 {
            return Err("mesh outside retained GPU budget");
        }
        if self.clock == u64::MAX {
            let ordered: Vec<_> = self.ages.values().cloned().collect();
            self.ages.clear();
            for (age, id) in ordered.into_iter().enumerate() {
                self.entries.get_mut(&id).unwrap().age = age as u64;
                self.ages.insert(age as u64, id);
            }
            self.clock = self.entries.len() as u64;
        }
        let age = self.clock;
        self.clock += 1;
        if let Some(entry) = self.entries.get_mut(&key) {
            self.ages.remove(&entry.age);
            entry.age = age;
            self.ages.insert(age, key.clone());
            self.stats.hits += 1;
            return Ok(entry.mesh.clone());
        }
        // Admission and data validation precede eviction; failed admission leaves
        // all reusable entries intact. Device allocation errors belong to the host.
        let mesh = Arc::new(upload()?);
        while self.entries.len() >= self.max_entries || self.bytes > self.max_bytes - bytes {
            let (_, id) = self.ages.pop_first().expect("cache accounting invariant");
            self.bytes -= self.entries.remove(&id).unwrap().mesh.allocated_bytes;
            self.stats.evictions += 1;
        }
        self.stats.uploads += 1;
        self.stats.uploaded_bytes += bytes;
        self.bytes += bytes;
        self.ages.insert(age, key.clone());
        self.entries.insert(
            key,
            Entry {
                mesh: mesh.clone(),
                age,
            },
        );
        Ok(mesh)
    }
}
