use std::str::FromStr;

pub trait CustomerOwnerGroup<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn customer_owner_group() -> Option<&'static str> {
        None
    }
}

pub trait OrganizationOwnerGroup<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn organization_owner_group() -> Option<&'static str> {
        None
    }
}

pub trait InstitutionOwnerGroup<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn institution_owner_group() -> Option<&'static str> {
        None
    }
}

pub trait CustomerUnitOwnerGroup<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn customer_unit_owner_group() -> Option<&'static str> {
        None
    }
}

pub trait InstitutionUnitOwnerGroup<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn institution_unit_owner_group() -> Option<&'static str> {
        None
    }
}

pub trait RelatedGroups<R, P>:
    CustomerOwnerGroup<R, P>
    + OrganizationOwnerGroup<R, P>
    + InstitutionOwnerGroup<R, P>
    + CustomerUnitOwnerGroup<R, P>
    + InstitutionUnitOwnerGroup<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn built_in_groups() -> &'static [&'static str];
}

pub trait RelatedBuiltInGroup:
    AsRef<str> + async_graphql::InputType + FromStr + Eq + PartialEq + Send + Sync + 'static
{
}
