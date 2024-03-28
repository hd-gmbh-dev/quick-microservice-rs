pub mod cache;
pub mod cleanup;
pub mod config;
pub mod context;
pub mod groups;
pub mod marker;
pub mod model;
pub mod mutation;
pub mod query;
pub mod roles;
pub mod schema;
pub mod worker;

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
        impl $crate::context::RelatedStorage for $storage {}
    };
}

#[macro_export]
macro_rules! cache {
    ($storage:ty) => {
        impl $crate::context::InMemoryCache for $storage {
            fn cache_db(&self) -> &$crate::cache::CacheDB {
                &self.inner.cache_db
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
