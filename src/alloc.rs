use crate::InternSerializeRegistry;
use core::{borrow::Borrow, error::Error, fmt, hash::Hash};
#[cfg(not(feature = "std"))]
use hashbrown::{hash_map::Entry, HashMap};
use rkyv::rancor::{fail, Source};
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

impl Error for InternSerializeMapError {}

#[derive(Default)]
pub struct InternSerializeMap<T> {
    value_to_pos: HashMap<T, usize>,
}

impl<T: Hash + Eq, E: Source> InternSerializeRegistry<T, E> for InternSerializeMap<T> {
    fn get_interned<U: Hash + Eq + ?Sized>(&self, value: &U) -> Option<usize>
    where
        T: Borrow<U>,
    {
        self.value_to_pos.get(value).copied()
    }

    fn add_interned(&mut self, value: T, pos: usize) -> Result<(), E> {
        if let Entry::Vacant(entry) = self.value_to_pos.entry(value) {
            entry.insert(pos);
            Ok(())
        } else {
            fail!(InternSerializeMapError::DuplicateKeyAdded)
        }
    }
}
