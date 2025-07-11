use std::sync::Arc;

use async_graphql::Description;
use async_graphql::InputValueError;
use async_graphql::InputValueResult;
use async_graphql::OneofObject;
use async_graphql::Scalar;
use async_graphql::ScalarType;
use async_graphql::Value;

use crate::ids::CustomerId;
use crate::ids::CustomerResourceId;
use crate::ids::InfraContext;
use crate::ids::InstitutionId;
use crate::ids::InstitutionResourceId;
use crate::ids::OrganizationId;
use crate::ids::OrganizationResourceId;

#[cfg_attr(
    feature = "serde-str",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
pub struct InfraContextId(pub InfraContext);

impl Description for InfraContextId {
    fn description() -> &'static str {
        "InfraContextId"
    }
}

#[Scalar(use_type_description)]
impl ScalarType for InfraContextId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(InfraContextId(
                InfraContext::parse(value)
                    .map_err(|err| InputValueError::custom(err.to_string()))?,
            ))
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

#[macro_export]
macro_rules! impl_id_scalar {
    ($t:ty) => {
        #[Scalar(use_type_description)]
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

#[derive(OneofObject)]
pub enum CustomerOrOrganization {
    Customer(CustomerId),
    Organization(OrganizationId),
}

#[derive(OneofObject, serde::Serialize, serde::Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum OrganizationOrInstitution {
    Organization(OrganizationId),
    Institution(InstitutionId),
}

pub type CustomerIds = Arc<[CustomerId]>;
pub type CustomerResourceIds = Arc<[CustomerResourceId]>;
pub type OrganizationIds = Arc<[OrganizationId]>;
pub type OrganizationResourceIds = Arc<[OrganizationResourceId]>;
pub type InstitutionIds = Arc<[InstitutionId]>;
pub type InstitutionResourceIds = Arc<[InstitutionResourceId]>;

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde-str")]
    #[test]
    fn test_infra_context_id_serde() {
        use super::{InfraContext, InfraContextId};
        let infra_context = serde_json::from_str::<InfraContextId>("\"V09\"")
            .expect("Failed to parse InfraContextId");
        assert_eq!(infra_context.0, InfraContext::Customer(9.into()));
        assert_eq!(
            serde_json::to_string(&infra_context).expect("Failed to serialize InfraContextId"),
            "\"V09\""
        );
    }
}
