use crate::{ArchivedInternedString, Intern, InternSerializeRegistry, InternedStringResolver};
#[cfg(not(feature = "std"))]
use alloc_::string::String;
use rkyv::{
    rancor::Fallible,
    ser::Writer,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Place,
};

impl ArchiveWith<String> for Intern {
    type Archived = ArchivedInternedString;
    type Resolver = InternedStringResolver;

    fn resolve_with(field: &String, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedInternedString::resolve_from_str(field.as_str(), resolver, out);
    }
}

impl<S> SerializeWith<String, S> for Intern
where
    S: Fallible + InternSerializeRegistry<String> + Writer + ?Sized,
{
    fn serialize_with(field: &String, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedInternedString::serialize_from_str(field.as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedInternedString, String, D> for Intern {
    fn deserialize_with(field: &ArchivedInternedString, _: &mut D) -> Result<String, D::Error> {
        Ok(String::from(field.as_str()))
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
