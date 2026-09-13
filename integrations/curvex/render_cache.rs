// Included by the isolated Curvex migration. Bound retained payload as well as
// entry count; a revision change still invalidates everything atomically.
const OSV_RENDER_CACHE_BYTES: usize = 64 * 1024 * 1024;
struct RenderCache {
    revision: Option<u64>,
    entries: HashMap<CacheKey, CacheEntry>,
    ages: HashMap<CacheKey, u64>,
    costs: HashMap<CacheKey, usize>,
    lru: VecDeque<(CacheKey, u64)>,
    clock: u64,
    bytes: usize,
}
impl RenderCache {
    fn new() -> Self {
        Self { revision: None, entries: HashMap::new(), ages: HashMap::new(),
            costs: HashMap::new(), lru: VecDeque::new(), clock: 0, bytes: 0 }
    }
    fn set_revision(&mut self, revision: u64) {
        if self.revision != Some(revision) {
            *self = Self::new();
            self.revision = Some(revision);
        }
    }
    fn touch(&mut self, key: &CacheKey) {
        // Compact stale touches in bounded batches, amortized constant work.
        // No linear search/removal on every cache hit.
        if self.lru.len() >= RENDER_CACHE_LIMIT * 2 || self.clock == u64::MAX {
            self.lru.retain(|(key, age)| self.ages.get(key) == Some(age));
            if self.clock == u64::MAX {
                for (i, (key, age)) in self.lru.iter_mut().enumerate() {
                    *age = i as u64;
                    self.ages.insert(key.clone(), *age);
                }
                self.clock = self.lru.len() as u64;
            }
        }
        self.clock += 1;
        self.ages.insert(key.clone(), self.clock);
        self.lru.push_back((key.clone(), self.clock));
    }
    fn get(&mut self, key: &CacheKey) -> Option<CacheEntry> {
        let entry = self.entries.get(key)?.clone();
        self.touch(key);
        Some(entry)
    }
    fn remove(&mut self, key: &CacheKey) {
        self.entries.remove(key);
        self.ages.remove(key);
        self.bytes -= self.costs.remove(key).unwrap_or(0);
    }
    fn insert(&mut self, key: CacheKey, entry: CacheEntry) {
        let cost = osv_cache_cost(&entry);
        self.remove(&key);
        if cost > OSV_RENDER_CACHE_BYTES { return; }
        while self.entries.len() >= RENDER_CACHE_LIMIT || self.bytes + cost > OSV_RENDER_CACHE_BYTES {
            let Some((oldest, age)) = self.lru.pop_front() else { break; };
            if self.ages.get(&oldest) == Some(&age) { self.remove(&oldest); }
        }
        self.bytes += cost;
        self.costs.insert(key.clone(), cost);
        self.entries.insert(key.clone(), entry);
        self.touch(&key);
    }
}
fn osv_cache_cost(entry: &CacheEntry) -> usize {
    let geometry = |g: &osv_geometry::render::tess::VertexBuffers<Pos2,u32>| {
        g.vertices.capacity() * std::mem::size_of::<Pos2>() + g.indices.capacity() * 4
    };
    std::mem::size_of::<CacheEntry>() + match entry {
        CacheEntry::Mesh(g) => geometry(g),
        CacheEntry::GradientFill(g) => geometry(&g.geometry) + std::mem::size_of::<MmRect>(),
        CacheEntry::GradientSamples { base, sampled, .. } => osv_cache_cost(base)
            + sampled.positions.capacity()*16 + sampled.values.capacity()*32 + sampled.indices.capacity()*4,
        // Ghosts contain heterogeneous shape data; use a conservative serialized
        // payload estimate. This is a cache weight, not a process RAM guarantee.
        CacheEntry::Ghost(Some(g)) => std::mem::size_of::<Shape>()
            + serde_json::to_vec(g.as_ref()).map_or(OSV_RENDER_CACHE_BYTES, |v|v.len().saturating_mul(2)),
        CacheEntry::Ghost(None) => 0,
    }
}
#[cfg(test)]
mod osv_large_render_cache_tests {
    use super::*;
    fn key(i:u64) -> CacheKey { CacheKey { id: ShapeId::new(), kind: KIND_RIBBON, discrim:i, zoom_bucket:0 } }
    #[test]
    fn large_working_set_hits_and_touch_log_stays_bounded() {
        let mut cache=RenderCache::new();cache.set_revision(1);
        let keys:Vec<_>=(0..5000).map(key).collect();
        for k in &keys { cache.insert(k.clone(),CacheEntry::Ghost(None)); }
        for _ in 0..10 {for k in &keys {assert!(cache.get(k).is_some());}}
        assert!(cache.lru.len()<=RENDER_CACHE_LIMIT*2);
        assert_eq!(cache.entries.len(),5000);
        cache.set_revision(2);assert!(cache.entries.is_empty());assert_eq!(cache.bytes,0);
    }
    #[test]
    fn eviction_respects_recent_hits_replacement_and_payload_budget() {
        let mut cache=RenderCache::new();let keys:Vec<_>=(0..RENDER_CACHE_LIMIT as u64).map(key).collect();
        for k in &keys {cache.insert(k.clone(),CacheEntry::Ghost(None));}
        cache.get(&keys[0]).unwrap();cache.insert(key(99999),CacheEntry::Ghost(None));
        assert!(cache.get(&keys[0]).is_some());assert!(cache.get(&keys[1]).is_none());
        let bytes=cache.bytes;cache.insert(keys[0].clone(),CacheEntry::Ghost(None));assert_eq!(cache.bytes,bytes);
        let large=Rc::new(osv_geometry::render::tess::VertexBuffers { vertices:Vec::<Pos2>::with_capacity(OSV_RENDER_CACHE_BYTES/8+1), indices:vec![] });
        cache.insert(keys[0].clone(),CacheEntry::Mesh(large));assert!(cache.get(&keys[0]).is_none());assert!(cache.bytes<=OSV_RENDER_CACHE_BYTES);
        cache.clock=u64::MAX;cache.get(&keys[2]).unwrap();assert!(cache.clock<u64::MAX);
    }
}
