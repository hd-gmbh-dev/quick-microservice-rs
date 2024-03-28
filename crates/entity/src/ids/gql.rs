use std::sync::Arc;

use async_graphql::InputValueError;
use async_graphql::InputValueResult;
use async_graphql::OneofObject;
use async_graphql::Scalar;
use async_graphql::ScalarType;
use async_graphql::Value;

use crate::ids::CustomerId;
use crate::ids::CustomerResourceId;
use crate::ids::CustomerUnitId;
use crate::ids::CustomerUnitResourceId;
use crate::ids::InstitutionId;
use crate::ids::InstitutionResourceId;
use crate::ids::InstitutionUnitId;
use crate::ids::InstitutionUnitResourceId;
use crate::ids::OrganizationId;
use crate::ids::OrganizationResourceId;
use crate::ids::OrganizationUnitId;

macro_rules! impl_id_scalar {
    ($t:ty) => {
        #[Scalar]
        impl ScalarType for $t {
            fn parse(value: Value) -> InputValueResult<Self> {
                if let Value::String(value) = &value {
                    // Parse the integer value
                    Ok(<$t>::parse(value)
                        .map_err(|err| InputValueError::custom(err.to_string()))?)
                } else {
                    // If the type does not match
                    Err(InputValueError::expected_type(value))
                }
            }

            fn to_value(&self) -> Value {
                Value::String(self.to_string())
            }
        }
    };
}

impl_id_scalar!(CustomerId);
impl_id_scalar!(CustomerResourceId);
impl_id_scalar!(OrganizationId);
impl_id_scalar!(OrganizationResourceId);
impl_id_scalar!(InstitutionId);
impl_id_scalar!(InstitutionResourceId);
impl_id_scalar!(InstitutionUnitId);
impl_id_scalar!(InstitutionUnitResourceId);
impl_id_scalar!(CustomerUnitId);
impl_id_scalar!(CustomerUnitResourceId);
impl_id_scalar!(OrganizationUnitId);

#[derive(OneofObject)]
pub enum CustomerOrOrganization {
    Customer(CustomerId),
    Organization(OrganizationId),
}

pub type CustomerIds = Arc<[CustomerId]>;
pub type CustomerResourceIds = Arc<[CustomerResourceId]>;
pub type OrganizationIds = Arc<[OrganizationId]>;
pub type OrganizationResourceIds = Arc<[OrganizationResourceId]>;
pub type InstitutionIds = Arc<[InstitutionId]>;
pub type InstitutionResourceIds = Arc<[InstitutionResourceId]>;
pub type InstitutionUnitIds = Arc<[InstitutionUnitId]>;
pub type InstitutionUnitResourceIds = Arc<[InstitutionUnitResourceId]>;
pub type CustomerUnitIds = Arc<[CustomerUnitId]>;
pub type CustomerUnitResourceIds = Arc<[CustomerUnitResourceId]>;
pub type OrganizationUnitIds = Arc<[OrganizationUnitId]>;
