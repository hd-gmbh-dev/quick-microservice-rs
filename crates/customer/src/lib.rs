pub mod cache;
pub mod cleanup;
pub mod context;
pub mod groups;
pub mod marker;
pub mod model;
pub mod roles;
pub mod schema;
pub mod worker;
// impl $crate::context::RelatedStorage for $storage {}

#[macro_export]
macro_rules! database {
    ($database:ty) => {
        impl $crate::cache::customer::CustomerCacheDB for $database {}
        impl $crate::cache::user::UserCacheDB for $database {}
        impl $crate::cache::CacheDB for $database {}
    };
}

#[macro_export]
macro_rules! storage {
    ($storage:ty) => {
        impl $crate::schema::customer::CustomerDB for $storage {}
        impl $crate::schema::organization::OrganizationDB for $storage {}
        impl $crate::schema::organization_unit::OrganizationUnitDB for $storage {}
        impl $crate::schema::institution::InstitutionDB for $storage {}
        impl $crate::schema::user::UserDB for $storage {}
        impl $crate::roles::RoleDB for $storage {}
        impl $crate::cache::customer::CustomerCacheDB for $storage {}
        impl $crate::cache::user::UserCacheDB for $storage {}
        impl $crate::cache::CacheDB for $storage {}
        impl $crate::context::RelatedStorage for $storage {}
    };
}

#[macro_export]
macro_rules! cache {
    ($storage:ty) => {
        impl $crate::context::InMemoryCache for $storage {
            fn cache(&self) -> &$crate::cache::Cache {
                &self.inner.cache
            }
        }
    };
}

#[macro_export]
macro_rules! mutation_event_producer {
    ($storage:ty) => {
        impl $crate::context::MutationEventProducer for $storage {
            fn mutation_event_producer(&self) -> Option<&$crate::context::Producer> {
                Some(&self.inner.mutation_event_producer)
            }
        }
    };
}

#[macro_export]
macro_rules! cleanup_task_producer {
    ($storage:ty) => {
        impl $crate::worker::CleanupTaskProducer for $storage {
            fn cleanup_task_producer(&self) -> &$crate::worker::Producer {
                self.inner.cleanup_task_producer.as_ref()
            }
        }
    };
}
