pub type Marker<Auth, Store, Access, Resource, Permission> =
    std::marker::PhantomData<Option<(Auth, Store, Access, Resource, Permission)>>;

pub type ArpMarker<Access, Resource, Permission> =
    std::marker::PhantomData<Option<(Access, Resource, Permission)>>;

pub type RpMarker<Resource, Permission> = std::marker::PhantomData<Option<(Resource, Permission)>>;
