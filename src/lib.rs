#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
mod alloc;
mod impls;
mod string;

use core::{alloc::Layout, borrow::Borrow, fmt, hash::Hash, ptr::NonNull};
use rkyv::{
    ser::{ScratchSpace, Serializer, SharedSerializeRegistry},
    Fallible,
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

pub trait InternSerializeRegistry<T>: Fallible {
    fn get_interned<U: Hash + Eq + ?Sized>(&self, value: &U) -> Option<usize>
    where
        T: Borrow<U>;

    fn add_interned(&mut self, value: T, pos: usize) -> Result<(), Self::Error>;

    fn serialize_interned<U>(&mut self, value: &U) -> Result<usize, Self::Error>
    where
        Self: Serializer,
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

/// An error occurred while serializing interned types.
#[derive(Debug)]
pub enum InternSerializerAdapterError<S, T> {
    SerializerError(S),
    InternError(T),
}

impl<S: fmt::Display, T: fmt::Display> fmt::Display for InternSerializerAdapterError<S, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternSerializerAdapterError::SerializerError(e) => e.fmt(f),
            InternSerializerAdapterError::InternError(e) => e.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl<S: 'static + Error, T: 'static + Error> Error for InternSerializerAdapterError<S, T> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                InternSerializerAdapterError::SerializerError(e) => Some(e as &dyn Error),
                InternSerializerAdapterError::InternError(e) => Some(e as &dyn Error),
            }
        }
    }
};

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

impl<S: Fallible, T: Fallible> Fallible for InternSerializerAdapter<S, T> {
    type Error = InternSerializerAdapterError<S::Error, T::Error>;
}

impl<S: ScratchSpace, T: Fallible> ScratchSpace for InternSerializerAdapter<S, T> {
    #[inline]
    unsafe fn push_scratch(&mut self, layout: Layout) -> Result<NonNull<[u8]>, Self::Error> {
        self.serializer.push_scratch(layout)
            .map_err(InternSerializerAdapterError::SerializerError)
    }

    #[inline]
    unsafe fn pop_scratch(&mut self, ptr: NonNull<u8>, layout: Layout) -> Result<(), Self::Error> {
        self.serializer.pop_scratch(ptr, layout)
            .map_err(InternSerializerAdapterError::SerializerError)
    }
}

impl<S: Serializer, T: Fallible> Serializer for InternSerializerAdapter<S, T> {
    #[inline]
    fn pos(&self) -> usize {
        self.serializer.pos()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.serializer.write(bytes)
            .map_err(InternSerializerAdapterError::SerializerError)
    }

    #[inline]
    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        self.serializer.pad(padding)
            .map_err(InternSerializerAdapterError::SerializerError)
    }

    #[inline]
    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        self.serializer.align(align)
            .map_err(InternSerializerAdapterError::SerializerError)
    }

    #[inline]
    fn align_for<U>(&mut self) -> Result<usize, Self::Error> {
        self.serializer.align_for::<U>()
            .map_err(InternSerializerAdapterError::SerializerError)
    }

    #[inline]
    unsafe fn resolve_aligned<U: rkyv::Archive + ?Sized>(&mut self, value: &U, resolver: U::Resolver) -> Result<usize, Self::Error> {
        self.serializer.resolve_aligned(value, resolver)
            .map_err(InternSerializerAdapterError::SerializerError)
    }

    #[inline]
    unsafe fn resolve_unsized_aligned<U: rkyv::ArchiveUnsized + ?Sized>(&mut self, value: &U, to: usize, metadata_resolver: U::MetadataResolver) -> Result<usize, Self::Error> {
        self.serializer.resolve_unsized_aligned(value, to, metadata_resolver)
            .map_err(InternSerializerAdapterError::SerializerError)
    }
}

impl<S: SharedSerializeRegistry, T: Fallible> SharedSerializeRegistry for InternSerializerAdapter<S, T> {
    #[inline]
    fn get_shared_ptr(&self, value: *const u8) -> Option<usize> {
        self.serializer.get_shared_ptr(value)
    }

    #[inline]
    fn add_shared_ptr(&mut self, value: *const u8, pos: usize) -> Result<(), Self::Error> {
        self.serializer.add_shared_ptr(value, pos)
            .map_err(InternSerializerAdapterError::SerializerError)
    }
}

impl<S: Fallible, T: InternSerializeRegistry<U>, U> InternSerializeRegistry<U> for InternSerializerAdapter<S, T> {
    #[inline]
    fn get_interned<Q: Hash + Eq + ?Sized>(&self, value: &Q) -> Option<usize>
    where
        U: Borrow<Q>,
    {
        self.intern_registry.get_interned(value)
    }

    #[inline]
    fn add_interned(&mut self, value: U, pos: usize) -> Result<(), Self::Error> {
        self.intern_registry.add_interned(value, pos)
            .map_err(InternSerializerAdapterError::InternError)
    }
}

#[cfg(test)]
mod tests {
    use rkyv::archived_root;


    #[test]
    fn intern_strings() {
        use crate::{Intern, InternSerializeMap, InternSerializerAdapter};
        use rkyv::{
            ser::{serializers::AllocSerializer, Serializer},
            Archive,
            Deserialize,
            Infallible,
            Serialize,
        };

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Log {
            #[with(Intern)]
            user: String,
            code: u16,
        }

        const USERS: [&'static str; 4] = [
            "Alice, the leader and brains behind the team",
            "Bob, bodybuilder and the muslce of the operation",
            "Carol, safe-cracker and swindler extraordinaire",
            "Dave, Jumanji master of the nineteenth dimension",
        ];
        let mut logs = Vec::new();
        for i in 0..64 {
            logs.push(Log {
                user: USERS[i % 4].to_string(),
                code: i as u16,
            });
        }

        type MySerializer = InternSerializerAdapter<
            AllocSerializer<4096>,
            InternSerializeMap<String>,
        >;

        let mut value = Vec::new();
        for i in 0..1000 {
            value.push(Log {
                user: USERS[i % USERS.len()].to_string(),
                code: (i % u16::MAX as usize) as u16,
            });
        }

        let mut serializer = MySerializer::default();
        serializer.serialize_value(&value)
            .expect("failed to serialize interned strings");

        let result = serializer.into_serializer().into_serializer().into_inner();
        assert!(result.len() < 20_000);

        let archived = unsafe { archived_root::<Vec<Log>>(result.as_ref()) };
        assert_eq!(archived, &value);

        let deserialized: Vec<Log> = archived.deserialize(&mut Infallible)
            .expect("failed to deserialized interned strings");
        assert_eq!(deserialized, value);
    }
}
