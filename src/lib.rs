#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc as alloc_;

#[cfg(feature = "alloc")]
mod alloc;
mod impls;
mod string;

use core::{alloc::Layout, borrow::Borrow, hash::Hash, ptr::NonNull};
use rkyv::{
    rancor::{Fallible, Strategy},
    ser::{sharing::SharingState, Allocator, Positional, Sharing, Writer},
    SerializeUnsized,
};

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use string::*;

/// A wrapper that pools strings to reduce memory usage.
///
/// This causes any interned archived strings to become immutable.
///
/// # Example
///
/// ```
/// use rkyv::Archive;
/// use rkyv_intern::Intern;
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(Intern)]
///     name: String,
/// }
/// ```
#[derive(Debug)]
pub struct Intern;

pub trait InternSerializeRegistry<T, E = <Self as Fallible>::Error> {
    fn get_interned<U: Hash + Eq + ?Sized>(&self, value: &U) -> Option<usize>
    where
        T: Borrow<U>;

    fn add_interned(&mut self, value: T, pos: usize) -> Result<(), E>;

    fn serialize_interned<U>(&mut self, value: &U) -> Result<usize, E>
    where
        Self: Fallible<Error = E>,
        T: Borrow<U> + for<'a> From<&'a U>,
        U: Hash + Eq + ?Sized + SerializeUnsized<Self>,
    {
        if let Some(pos) = self.get_interned(value) {
            Ok(pos)
        } else {
            let owned = T::from(value);
            let pos = value.serialize_unsized(self)?;
            self.add_interned(owned, pos)?;
            Ok(pos)
        }
    }
}

/// A basic adapter that can add interning capabilities to a compound serializer.
///
/// While this struct is useful for ergonomics, it's best to define a custom serializer when
/// combining capabilities across many crates.
#[derive(Debug, Default)]
pub struct InternSerializerAdapter<S, T> {
    serializer: S,
    intern_registry: T,
}

impl<S, T> InternSerializerAdapter<S, T> {
    /// Constructs a new intern serializer adapter from the given serializer and intern registry.
    pub fn new(serializer: S, intern_registry: T) -> Self {
        Self {
            serializer,
            intern_registry,
        }
    }

    /// Consumes the adapter and returns the components.
    pub fn into_components(self) -> (S, T) {
        (self.serializer, self.intern_registry)
    }

    /// Consumes the adapter and returns the underlying serializer.
    pub fn into_serializer(self) -> S {
        self.serializer
    }
}

unsafe impl<S: Allocator<E>, T, E> Allocator<E> for InternSerializerAdapter<S, T> {
    #[inline]
    unsafe fn push_alloc(&mut self, layout: Layout) -> Result<NonNull<[u8]>, E> {
        self.serializer.push_alloc(layout)
    }

    #[inline]
    unsafe fn pop_alloc(&mut self, ptr: NonNull<u8>, layout: Layout) -> Result<(), E> {
        self.serializer.pop_alloc(ptr, layout)
    }
}

impl<S: Positional, T> Positional for InternSerializerAdapter<S, T> {
    #[inline]
    fn pos(&self) -> usize {
        self.serializer.pos()
    }
}

impl<S: Writer<E>, T, E> Writer<E> for InternSerializerAdapter<S, T> {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.serializer.write(bytes)
    }
}

impl<S: Sharing<E>, T, E> Sharing<E> for InternSerializerAdapter<S, T> {
    #[inline]
    fn start_sharing(&mut self, address: usize) -> SharingState {
        self.serializer.start_sharing(address)
    }

    #[inline]
    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), E> {
        self.serializer.finish_sharing(address, pos)
    }
}

impl<S, T: InternSerializeRegistry<U, E>, U, E> InternSerializeRegistry<U, E>
    for InternSerializerAdapter<S, T>
{
    #[inline]
    fn get_interned<Q: Hash + Eq + ?Sized>(&self, value: &Q) -> Option<usize>
    where
        U: Borrow<Q>,
    {
        self.intern_registry.get_interned(value)
    }

    #[inline]
    fn add_interned(&mut self, value: U, pos: usize) -> Result<(), E> {
        self.intern_registry.add_interned(value, pos)
    }
}

impl<T: InternSerializeRegistry<U, E>, U, E> InternSerializeRegistry<U, E> for Strategy<T, E> {
    #[inline]
    fn get_interned<Q: Hash + Eq + ?Sized>(&self, value: &Q) -> Option<usize>
    where
        U: Borrow<Q>,
    {
        T::get_interned(self, value)
    }

    #[inline]
    fn add_interned(&mut self, value: U, pos: usize) -> Result<(), E> {
        T::add_interned(self, value, pos)
    }
}

#[cfg(all(feature = "alloc", feature = "bytecheck"))]
#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use rkyv::{
        rancor::{Panic, ResultExt},
        ser::{allocator::SubAllocator, Serializer},
        util::AlignedVec,
        vec::ArchivedVec,
    };

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    use alloc_::{
        boxed::Box,
        string::{String, ToString},
        vec::Vec,
    };

    #[test]
    fn intern_strings() {
        use crate::{Intern, InternSerializeMap, InternSerializerAdapter};
        use rkyv::{Archive, Deserialize, Serialize};

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(compare(PartialEq), derive(Debug))]
        struct Log {
            #[rkyv(with = Intern)]
            user: Option<String>,
            code: u16,
        }

        const USERS: [&str; 4] = [
            "Alice, the leader and brains behind the team",
            "Bob, bodybuilder and the muslce of the operation",
            "Carol, safe-cracker and swindler extraordinaire",
            "Dave, Jumanji master of the nineteenth dimension",
        ];

        let mut value = Vec::new();
        for i in 0..1000 {
            value.push(Log {
                user: Some(USERS[i % USERS.len()].to_string()),
                code: (i % u16::MAX as usize) as u16,
            });
        }

        let mut alloc: Box<[MaybeUninit<u8>]> = Box::from([MaybeUninit::uninit(); 16_000]);

        let mut serializer = InternSerializerAdapter::new(
            Serializer::new(AlignedVec::<8>::new(), SubAllocator::new(&mut alloc), ()),
            InternSerializeMap::default(),
        );

        rkyv::api::serialize_using::<_, Panic>(&value, &mut serializer).always_ok();

        let result = serializer.into_serializer().into_writer();
        assert!(result.len() < 20_000);

        let archived = rkyv::access::<ArchivedVec<ArchivedLog>, Panic>(&result).always_ok();
        assert_eq!(archived, &value);

        let deserialized = rkyv::deserialize::<Vec<Log>, Panic>(archived).always_ok();
        assert_eq!(deserialized, value);
    }
}
