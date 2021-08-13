use crate::InternSerializeRegistry;
use core::{borrow::Borrow, fmt, hash::Hash};
#[cfg(not(feature = "std"))]
use hashbrown::{hash_map::Entry, HashMap};
use rkyv::Fallible;
#[cfg(feature = "std")]
use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug)]
pub enum InternSerializeMapError {
    DuplicateKeyAdded,
}

impl fmt::Display for InternSerializeMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "duplicate key added to intern serialize registry")
    }
}

impl std::error::Error for InternSerializeMapError {}

#[derive(Default)]
pub struct InternSerializeMap<T> {
    value_to_pos: HashMap<T, usize>,
}

impl<T> Fallible for InternSerializeMap<T> {
    type Error = InternSerializeMapError;
}

impl<T: Hash + Eq> InternSerializeRegistry<T> for InternSerializeMap<T> {
    fn get_interned<U: Hash + Eq + ?Sized>(&self, value: &U) -> Option<usize>
    where
        T: Borrow<U>,
    {
        self.value_to_pos.get(value).cloned()
    }

    fn add_interned(&mut self, value: T, pos: usize) -> Result<(), Self::Error> {
        if let Entry::Vacant(entry) = self.value_to_pos.entry(value) {
            entry.insert(pos);
            Ok(())
        } else {
            Err(InternSerializeMapError::DuplicateKeyAdded)
        }
    }
}
