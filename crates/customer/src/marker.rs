pub type Marker<Auth, Store, Resource, Permission, BuiltInGroup> =
    std::marker::PhantomData<Option<(Auth, Store, Resource, Permission, BuiltInGroup)>>;

pub type StoreMarker<Auth, Store> = std::marker::PhantomData<Option<(Auth, Store)>>;

pub type RpMarker<Resource, Permission> = std::marker::PhantomData<Option<(Resource, Permission)>>;
