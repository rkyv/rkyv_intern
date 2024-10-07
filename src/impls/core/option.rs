use crate::{Intern, InternSerializeRegistry};
use rkyv::{
    option::ArchivedOption,
    rancor::Fallible,
    with::{ArchiveWith, DeserializeWith, SerializeWith, With},
    Archive, Place, Serialize,
};

impl<T> ArchiveWith<Option<T>> for Intern
where
    Intern: ArchiveWith<T>,
{
    type Archived = ArchivedOption<<Intern as ArchiveWith<T>>::Archived>;
    type Resolver = Option<<Intern as ArchiveWith<T>>::Resolver>;

    fn resolve_with(field: &Option<T>, resolver: Self::Resolver, out: Place<Self::Archived>) {
        // SAFETY: With is repr(transparent) so Option<With<T, W>> has the same layout as Option<T>
        let field = unsafe { &*(field as *const Option<T> as *const Option<With<T, Intern>>) };
        field.resolve(resolver, unsafe { out.cast_unchecked() });
    }
}

impl<T, S> SerializeWith<Option<T>, S> for Intern
where
    S: Fallible + InternSerializeRegistry<T> + ?Sized,
    Intern: ArchiveWith<T>,
    Intern: SerializeWith<T, S>,
{
    fn serialize_with(field: &Option<T>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        unsafe {
            // SAFETY: With is repr(transparent) so Option<With<T, W>> has the same layout as
            // Option<T>
            let field = &*(field as *const Option<T> as *const Option<With<T, Intern>>);
            field.serialize(serializer)
        }
    }
}

impl<T, D> DeserializeWith<ArchivedOption<<Intern as ArchiveWith<T>>::Archived>, Option<T>, D>
    for Intern
where
    D: Fallible + ?Sized,
    Intern: ArchiveWith<T> + DeserializeWith<<Intern as ArchiveWith<T>>::Archived, T, D>,
{
    fn deserialize_with(
        field: &ArchivedOption<<Intern as ArchiveWith<T>>::Archived>,
        deserializer: &mut D,
    ) -> Result<Option<T>, D::Error> {
        field
            .as_ref()
            .map(|x| Intern::deserialize_with(x, deserializer))
            .transpose()
    }
}
