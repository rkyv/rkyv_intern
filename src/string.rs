#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc_::string::String;
use core::{
    borrow::Borrow,
    cmp, fmt, hash,
    ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};
use rkyv::{
    munge::munge,
    string::{
        repr::{ArchivedStringRepr, INLINE_CAPACITY},
        ArchivedString,
    },
    Place, Portable,
};

/// An interned archived string.
///
/// Because the memory for this string may be shared with other structures, it cannot be accessed
/// mutably.
#[repr(transparent)]
#[cfg_attr(
    feature = "bytecheck",
    derive(rkyv::bytecheck::CheckBytes),
    bytecheck(verify, crate = rkyv::bytecheck)
)]
#[derive(Portable)]
pub struct ArchivedInternedString(ArchivedStringRepr);

impl ArchivedInternedString {
    /// Extracts a string slice containing the entire `ArchivedInternedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Resolves an interned archived string from a given `str`.
    #[inline]
    pub fn resolve_from_str(value: &str, resolver: InternedStringResolver, out: Place<Self>) {
        munge!(let Self(repr) = out);
        if value.len() <= INLINE_CAPACITY {
            unsafe {
                ArchivedStringRepr::emplace_inline(value, repr.ptr());
            }
        } else {
            unsafe {
                ArchivedStringRepr::emplace_out_of_line(value, resolver.pos, repr);
            }
        }
    }

    /// Serializes an interned archived string from a given `str`.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn serialize_from_str<S>(
        value: &str,
        serializer: &mut S,
    ) -> Result<InternedStringResolver, S::Error>
    where
        S: crate::InternSerializeRegistry<String> + rkyv::rancor::Fallible + ?Sized,
        str: rkyv::SerializeUnsized<S>,
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
        Some(self.cmp(other))
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

#[cfg(feature = "bytecheck")]
const _: () = {
    use core::any::TypeId;

    use rkyv::{
        bytecheck::{CheckBytes, Verify},
        rancor::{Fallible, Source},
        validation::{shared::ValidationState, ArchiveContext, ArchiveContextExt, SharedContext},
    };

    unsafe impl<C> Verify<C> for ArchivedInternedString
    where
        C: Fallible + ArchiveContext + SharedContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            if self.0.is_inline() {
                unsafe {
                    str::check_bytes(self.0.as_str_ptr(), context)?;
                }
            } else {
                let base = (&self.0 as *const ArchivedStringRepr).cast::<u8>();
                let offset = unsafe { self.0.out_of_line_offset() };
                let address = base.wrapping_offset(offset) as usize;
                let type_id = TypeId::of::<Self>();

                match context.start_shared(address, type_id)? {
                    ValidationState::Started => {
                        let metadata = self.0.len();
                        let ptr = rkyv::ptr_meta::from_raw_parts(address as *const _, metadata);
                        context.in_subtree(ptr, |context| {
                            // SAFETY: `in_subtree` has guaranteed that `ptr` is
                            // properly aligned and points to enough bytes to represent
                            // the pointed-to `str`.
                            unsafe { str::check_bytes(ptr, context) }
                        })?;

                        context.finish_shared(address, type_id)?;
                    }
                    ValidationState::Pending | ValidationState::Finished => (),
                }
            }

            Ok(())
        }
    }
};
