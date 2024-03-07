pub trait CreateCustomerOwnerGroup<A, R, P> {
    fn create_customer_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait CreateOrganizationOwnerGroup<A, R, P> {
    fn create_organization_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait CreateInstitutionOwnerGroup<A, R, P> {
    fn create_institution_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait CreateOrganizationUnitOwnerGroup<A, R, P> {
    fn create_organization_unit_owner_group() -> qm_role::Group<A, R, P>;
}

pub trait RelatedGroups<A, R, P>:
    CreateCustomerOwnerGroup<A, R, P>
    + CreateOrganizationOwnerGroup<A, R, P>
    + CreateInstitutionOwnerGroup<A, R, P>
    + CreateOrganizationUnitOwnerGroup<A, R, P>
{
}
