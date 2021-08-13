use crate::InternSerializeRegistry;
use core::{
    borrow::Borrow,
    cmp, fmt, hash,
    ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    pin::Pin,
};
use rkyv::{
    ser::Serializer,
    string::{
        repr::{ArchivedStringRepr, INLINE_CAPACITY},
        ArchivedString,
    },
    SerializeUnsized,
};

/// An interned archived string.
///
/// Because the memory for this string may be shared with other structures, it cannot be accessed
/// mutably.
#[repr(transparent)]
pub struct ArchivedInternedString(ArchivedStringRepr);

impl ArchivedInternedString {
    /// Extracts a string slice containing the entire `ArchivedInternedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Extracts a pinned mutable string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn pin_mut_str(self: Pin<&mut Self>) -> Pin<&mut str> {
        unsafe { self.map_unchecked_mut(|s| s.0.as_mut_str()) }
    }

    /// Resolves an interned archived string from a given `str`.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` msut be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_str(
        value: &str,
        pos: usize,
        resolver: InternedStringResolver,
        out: *mut Self,
    ) {
        if value.len() <= INLINE_CAPACITY {
            ArchivedStringRepr::emplace_inline(value, out.cast());
        } else {
            ArchivedStringRepr::emplace_out_of_line(value, pos, resolver.pos, out.cast());
        }
    }

    /// Serializes an interned archived string from a given `str`.
    #[inline]
    pub fn serialize_from_str<S: InternSerializeRegistry<String> + Serializer + ?Sized>(
        value: &str,
        serializer: &mut S,
    ) -> Result<InternedStringResolver, S::Error>
    where
        str: SerializeUnsized<S>,
    {
        if value.len() <= INLINE_CAPACITY {
            Ok(InternedStringResolver { pos: 0 })
        } else {
            Ok(InternedStringResolver {
                pos: serializer.serialize_interned(value)?,
            })
        }
    }
}

impl AsRef<str> for ArchivedInternedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ArchivedInternedString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for ArchivedInternedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Deref for ArchivedInternedString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for ArchivedInternedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl Eq for ArchivedInternedString {}

impl hash::Hash for ArchivedInternedString {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

macro_rules! impl_index {
    ($index:ty) => {
        impl Index<$index> for ArchivedInternedString {
            type Output = str;

            #[inline]
            fn index(&self, index: $index) -> &Self::Output {
                self.as_str().index(index)
            }
        }
    };
}

impl_index!(Range<usize>);
impl_index!(RangeFrom<usize>);
impl_index!(RangeFull);
impl_index!(RangeInclusive<usize>);
impl_index!(RangeTo<usize>);
impl_index!(RangeToInclusive<usize>);

impl Ord for ArchivedInternedString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialEq for ArchivedInternedString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialOrd for ArchivedInternedString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl PartialEq<&str> for ArchivedInternedString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl PartialEq<str> for ArchivedInternedString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<ArchivedInternedString> for &str {
    #[inline]
    fn eq(&self, other: &ArchivedInternedString) -> bool {
        PartialEq::eq(other.as_str(), *self)
    }
}

impl PartialEq<ArchivedString> for ArchivedInternedString {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl PartialEq<ArchivedInternedString> for ArchivedString {
    #[inline]
    fn eq(&self, other: &ArchivedInternedString) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

pub struct InternedStringResolver {
    pos: usize,
}

#[cfg(feature = "validation")]
const _: () = {
    use crate::validation::{
        owned::{CheckOwnedPointerError, OwnedPointerError},
        ArchiveContext, SharedContext,
    };
    use bytecheck::{CheckBytes, Error};
    use core::any::TypeId;

    impl<C: ArchiveContext + SharedContext + ?Sized> CheckBytes<C> for ArchivedInternedString
    where
        C::Error: Error,
    {
        type Error = CheckOwnedPointerError<str, C>;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            // The repr is always valid
            let repr = &*value.cast::<ArchivedStringRepr>();

            if repr.is_inline() {
                // Inline interned strings are always owned
                str::check_bytes(repr.as_str_ptr(), context)
                    .map_err(OwnedPointerError::ValueCheckBytesError)?;
            } else {
                // Out-of-line interned strings are shared
                let base = value.cast();
                let offset = repr.out_of_line_offset();
                let metadata = repr.len();

                let ptr = context
                    .check_ptr::<str>(base, offset, metadata)
                    .map_err(OwnedPointerError::ContextError)?;

                let type_id = TypeId::of::<Self>();
                if context
                    .register_shared_ptr(ptr.cast(), type_id)
                    .map_err(OwnedPointerError::ContextError)?
                {
                    context
                        .bounds_check_subtree_ptr(ptr)
                        .map_err(OwnedPointerError::ContextError)?;

                    let range = context
                        .push_prefix_subtree(ptr)
                        .map_err(OwnedPointerError::ContextError)?;
                    str::check_bytes(ptr, context)
                        .map_err(OwnedPointerError::ValueCheckBytesError)?;
                    context
                        .pop_prefix_range(range)
                        .map_err(OwnedPointerError::ContextError)?;
                }
            }

            Ok(&*value)
        }
    }
};
