pub trait CreateCustomerOwnerGroup<A, R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn create_customer_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait CreateOrganizationOwnerGroup<A, R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn create_organization_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait CreateInstitutionOwnerGroup<A, R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn create_institution_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait CreateOrganizationUnitOwnerGroup<A, R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn create_organization_unit_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait RelatedGroups<A, R, P>:
    CreateCustomerOwnerGroup<A, R, P>
    + CreateOrganizationOwnerGroup<A, R, P>
    + CreateInstitutionOwnerGroup<A, R, P>
    + CreateOrganizationUnitOwnerGroup<A, R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn built_in_groups() -> &'static [&'static str];
}

pub trait RelatedBuiltInGroup:
    AsRef<str> + async_graphql::InputType + Send + Sync + 'static
{
}
