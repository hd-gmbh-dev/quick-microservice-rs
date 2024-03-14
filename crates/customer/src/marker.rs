pub type Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> =
    std::marker::PhantomData<
        Option<(Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup)>,
    >;

pub type ArpMarker<AccessLevel, Resource, Permission> =
    std::marker::PhantomData<Option<(AccessLevel, Resource, Permission)>>;

pub type RpMarker<Resource, Permission> = std::marker::PhantomData<Option<(Resource, Permission)>>;
