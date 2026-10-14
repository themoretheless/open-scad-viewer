//! Small insertion-ordered lexical maps. Replacements preserve declaration order.
use std::{borrow::Borrow, ops::Index};
#[derive(Clone, Debug)]
pub struct OrderedMap<K, V>(Vec<(K, V)>);
impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<K: Eq, V> OrderedMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some((_, v)) = self.0.iter_mut().find(|(k, _)| *k == key) {
            Some(std::mem::replace(v, value))
        } else {
            self.0.push((key, value));
            None
        }
    }
    pub fn get<Q: ?Sized + Eq>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.0
            .iter()
            .find(|(k, _)| k.borrow() == key)
            .map(|(_, v)| v)
    }
    pub fn contains_key<Q: ?Sized + Eq>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get(key).is_some()
    }
    pub fn shift_remove<Q: ?Sized + Eq>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let i = self.0.iter().position(|(k, _)| k.borrow() == key)?;
        Some(self.0.remove(i).1)
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().map(|(k, _)| k)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter().map(|(_, v)| v)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }
}
impl<K: Eq, V> FromIterator<(K, V)> for OrderedMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(i: T) -> Self {
        let mut m = Self::new();
        m.extend(i);
        m
    }
}
impl<K: Eq, V> Extend<(K, V)> for OrderedMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, i: T) {
        for (k, v) in i {
            self.insert(k, v);
        }
    }
}
impl<K, V> IntoIterator for OrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a, K, V> IntoIterator for &'a OrderedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, V)>, fn(&'a (K, V)) -> (&'a K, &'a V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(k, v)| (k, v))
    }
}
impl<K: Eq + Borrow<Q>, Q: ?Sized + Eq, V> Index<&Q> for OrderedMap<K, V> {
    type Output = V;
    fn index(&self, k: &Q) -> &V {
        self.get(k).expect("Missing ordered map key")
    }
}
