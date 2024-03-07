pub mod cache;
pub mod context;
pub mod groups;
pub mod marker;
pub mod model;
pub mod roles;
pub mod schema;

#[macro_export]
macro_rules! storage {
    ($storage:ty) => {
        impl $crate::context::RelatedStorage for $storage {}
    };
}

#[macro_export]
macro_rules! cache {
    ($storage:ty) => {
        impl $crate::context::InMemoryCache for $storage {
            fn cache(&self) -> Option<&$crate::cache::Cache> {
                Some(&self.inner.cache)
            }
        }
    };
}
