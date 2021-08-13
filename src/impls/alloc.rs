use crate::{ArchivedInternedString, Intern, InternSerializeRegistry, InternedStringResolver};
use rkyv::{
    ser::Serializer,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Fallible,
};
#[cfg(not(feature = "std"))]
use ::alloc::string::String;

impl ArchiveWith<String> for Intern {
    type Archived = ArchivedInternedString;
    type Resolver = InternedStringResolver;

    unsafe fn resolve_with(
        field: &String,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedInternedString::resolve_from_str(field.as_str(), pos, resolver, out);
    }
}

impl<S: InternSerializeRegistry<String> + Serializer + ?Sized> SerializeWith<String, S> for Intern {
    fn serialize_with(field: &String, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedInternedString::serialize_from_str(field.as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedInternedString, String, D> for Intern {
    fn deserialize_with(field: &ArchivedInternedString, _: &mut D) -> Result<String, D::Error> {
        Ok(field.as_str().to_string())
    }
}

impl PartialEq<String> for ArchivedInternedString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl PartialEq<ArchivedInternedString> for String {
    #[inline]
    fn eq(&self, other: &ArchivedInternedString) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}
