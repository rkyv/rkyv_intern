use crate::{Intern, InternSerializeRegistry};
use rkyv::{
    ser::Serializer,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    option::ArchivedOption,
    Fallible,
    out_field
};
use core::{hint::unreachable_unchecked, ptr};

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedOptionTag {
    None,
    Some,
}

#[repr(C)]
struct ArchivedOptionVariantNone(ArchivedOptionTag);

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

impl<T> ArchiveWith<Option<T>> for Intern
    where Intern: ArchiveWith<T>
{
    type Archived = ArchivedOption<<Intern as ArchiveWith<T>>::Archived>;
    type Resolver = Option<<Intern as ArchiveWith<T>>::Resolver>;

    unsafe fn resolve_with(field: &Option<T>, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        match resolver {
            None => {
                let out = out.cast::<ArchivedOptionVariantNone>();
                ptr::addr_of_mut!((*out).0).write(ArchivedOptionTag::None);
            },
            Some(resolver) => {
                let out = out.cast::<ArchivedOptionVariantSome<<Intern as ArchiveWith<T>>::Archived>>();
                ptr::addr_of_mut!((*out).0).write(ArchivedOptionTag::Some);

                let value = if let Some(value) = field.as_ref() {
                    value
                } else {
                    unreachable_unchecked();
                };

                let (fp, fo) = out_field!(out.1);
                <Intern as ArchiveWith<T>>::resolve_with(value, pos + fp, resolver, fo);
            }
        }
    }
}

impl<T, S: InternSerializeRegistry<T> + Serializer + ?Sized> SerializeWith<Option<T>, S> for Intern
    where Intern: ArchiveWith<T>,
          Intern: SerializeWith<T, S>
{
    fn serialize_with(field: &Option<T>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        match field {
            None => Ok(None),
            Some(field) => Ok(Some(<Intern as SerializeWith<T, S>>::serialize_with(field, serializer)?)),
        }
    }
}

impl<T, D: Fallible + ?Sized> DeserializeWith<ArchivedOption<<Intern as ArchiveWith<T>>::Archived>, Option<T>, D> for Intern
    where Intern: ArchiveWith<T>,
          Intern: DeserializeWith<<Intern as ArchiveWith<T>>::Archived, T, D>
{
    fn deserialize_with(field: &ArchivedOption<<Intern as ArchiveWith<T>>::Archived>, deserializer: &mut D) -> Result<Option<T>, D::Error> {
        match field {
            ArchivedOption::None => Ok(None),
            ArchivedOption::Some(field) => Ok(Some(<Intern as DeserializeWith<<Intern as ArchiveWith<T>>::Archived, T, D>>::deserialize_with(field, deserializer)?)),
        }
    }
}