use crate::{Intern, InternSerializeRegistry};
use rkyv::{
    option::ArchivedOption,
    ser::Serializer,
    with::{ArchiveWith, DeserializeWith, SerializeWith, With},
    Archive, Deserialize, Fallible, Serialize,
};

impl<T> ArchiveWith<Option<T>> for Intern
where
    Intern: ArchiveWith<T>,
{
    type Archived = ArchivedOption<<Intern as ArchiveWith<T>>::Archived>;
    type Resolver = Option<<Intern as ArchiveWith<T>>::Resolver>;

    unsafe fn resolve_with(
        field: &Option<T>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        // SAFETY: With is repr(transparent) so Option<With<T, W>> has the same layout as Option<T>
        let field = &*(field as *const Option<T> as *const Option<With<T, Intern>>);
        field.resolve(pos, resolver, out.cast());
    }
}

impl<T, S> SerializeWith<Option<T>, S> for Intern
where
    S: InternSerializeRegistry<T> + Serializer + ?Sized,
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
    Intern: ArchiveWith<T>,
    Intern: DeserializeWith<<Intern as ArchiveWith<T>>::Archived, T, D>,
{
    fn deserialize_with(
        field: &ArchivedOption<<Intern as ArchiveWith<T>>::Archived>,
        deserializer: &mut D,
    ) -> Result<Option<T>, D::Error> {
        Ok(
            Deserialize::<Option<With<T, Intern>>, D>::deserialize(field, deserializer)?
                .map(|x| x.into_inner()),
        )
    }
}
