use qm_mongodb::bson::oid::ObjectId;

pub fn select_ids<'a, T, U>(ids: &'a [T]) -> Vec<&'a ObjectId>
where
    T: AsRef<U>,
    U: std::ops::Deref<Target = ObjectId> + 'a,
{
    ids.iter().map(|v| v.as_ref().deref()).collect()
}
