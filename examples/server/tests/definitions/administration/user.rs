#![allow(clippy::too_many_arguments)]

use crate::utils::{str_cap, World};
use async_graphql::{Request, Response, Variables};

use crate::steps::common::{JRef, J};
use cucumber::{then, when};
use serde_json::json;

fn create_user_payload(username: &str) -> serde_json::Value {
    let mut s = username.split('.');
    let firstname = str_cap(s.next().unwrap().trim());
    let lastname = str_cap(s.next().unwrap().trim());
    let email = format!("{}@qm-test.live", username);
    let password = "Owner123!";
    let enabled = false;
    json!({
        "firstname": &firstname,
        "lastname": &lastname,
        "email": &email,
        "username": &username,
        "password": &password,
        "enabled": &enabled,
    })
}

async fn create_customer_user(w: &mut World, username: String) -> Response {
    let mut user = create_user_payload(username.as_str());
    let additional = json!({
        "context": &w.active_customer_id,
        "access": "CUSTOMER",
        "group": {
            "builtIn": "CUSTOMER_OWNER"
        },
    });
    user.as_object_mut()
        .unwrap()
        .extend(additional.as_object().unwrap().to_owned());
    let variables = json!({ "user": user });
    let request = Request::new(include_str!(
        "../../graphql/mutation/createUser/createUser.gql"
    ))
    .variables(Variables::from_json(variables));
    w.schema().execute(request).await
}

#[when(expr = "creating user at customer with username {string} as owner")]
pub async fn creating_user_at_customer(w: &mut World, username: String) -> anyhow::Result<()> {
    let result = create_customer_user(w, username).await;
    if result.is_err() {
        anyhow::bail!("{result:#?}");
    }
    let mut data = result.data.into_json()?;
    let data = data.as_object_mut().unwrap().remove("createUser");
    w.variables.push(data.unwrap());
    Ok(())
}

#[when(
    expr = "should not be able to create user at customer with username {string} as owner again"
)]
pub async fn should_not_be_able_to_create_user_at_customer_again(
    w: &mut World,
    username: String,
) -> anyhow::Result<()> {
    let result = create_customer_user(w, username).await;
    assert!(result.is_err());
    let err = result.errors.first().unwrap();
    let expected = serde_json::to_string(&serde_json::json!({
        "code": 409
    }))?;
    assert_eq!(serde_json::to_string(&err.extensions)?, expected);
    Ok(())
}

#[then(expr = "user is created for customer {string} with username {string} as owner")]
pub async fn user_is_created_for_customer(
    w: &mut World,
    customer: String,
    username: String,
) -> anyhow::Result<()> {
    let user = J(create_user_payload(&username));
    let data: serde_json::Value = w.variables.pop().unwrap();
    assert_eq!(
        JRef(data.pointer("/details").unwrap()).s("username"),
        user.s("username"),
    );
    assert_eq!(
        JRef(data.pointer("/details/customer").unwrap()).s("name"),
        customer.as_str(),
    );
    Ok(())
}

async fn create_organization_user(w: &mut World, username: String) -> Response {
    let mut user = create_user_payload(username.as_str());
    let additional = json!({
        "context": &format!("{}{}", w.active_customer_id, w.active_organization_id),
        "access": "ORGANIZATION",
        "group": {
            "builtIn": "ORGANIZATION_OWNER"
        },
    });
    user.as_object_mut()
        .unwrap()
        .extend(additional.as_object().unwrap().to_owned());
    let variables = json!({ "user": user });
    let request = Request::new(include_str!(
        "../../graphql/mutation/createUser/createUser.gql"
    ))
    .variables(Variables::from_json(variables));
    w.schema().execute(request).await
}

#[when(expr = "creating user at organization with username {string} as owner")]
pub async fn creating_user_at_organization(w: &mut World, username: String) -> anyhow::Result<()> {
    let result = create_organization_user(w, username).await;
    if result.is_err() {
        anyhow::bail!("{result:#?}");
    }
    let mut data = result.data.into_json()?;
    let data = data.as_object_mut().unwrap().remove("createUser");
    w.variables.push(data.unwrap());
    Ok(())
}

#[when(
    expr = "should not be able to create user at organization with username {string} as owner again"
)]
pub async fn should_not_be_able_to_create_user_at_organization_again(
    w: &mut World,
    username: String,
) -> anyhow::Result<()> {
    let result = create_organization_user(w, username).await;
    assert!(result.is_err());
    let err = result.errors.first().unwrap();
    let expected = serde_json::to_string(&serde_json::json!({
        "code": 409
    }))?;
    assert_eq!(serde_json::to_string(&err.extensions)?, expected);
    Ok(())
}

#[then(expr = "user is created for organization {string} with username {string} as owner")]
pub async fn user_is_created_for_organization(
    w: &mut World,
    organization: String,
    username: String,
) -> anyhow::Result<()> {
    let user = J(create_user_payload(&username));
    let data: serde_json::Value = w.variables.pop().unwrap();
    assert_eq!(
        JRef(data.pointer("/details").unwrap()).s("username"),
        user.s("username"),
    );
    assert_eq!(
        JRef(data.pointer("/details/organization").unwrap()).s("name"),
        organization.as_str(),
    );
    Ok(())
}

async fn create_institution_user(w: &mut World, username: String) -> Response {
    let mut user = create_user_payload(username.as_str());
    let additional = json!({
        "context": &format!("{}{}{}", w.active_customer_id, w.active_organization_id, w.active_institution_id),
        "access": "INSTITUTION",
        "group": {
            "builtIn": "INSTITUTION_OWNER"
        },
    });
    user.as_object_mut()
        .unwrap()
        .extend(additional.as_object().unwrap().to_owned());
    let variables = json!({ "user": user });
    let request = Request::new(include_str!(
        "../../graphql/mutation/createUser/createUser.gql"
    ))
    .variables(Variables::from_json(variables));
    w.schema().execute(request).await
}

#[when(expr = "creating user at institution with username {string} as owner")]
pub async fn creating_user_at_institution(w: &mut World, username: String) -> anyhow::Result<()> {
    let result = create_institution_user(w, username).await;
    if result.is_err() {
        anyhow::bail!("{result:#?}");
    }
    let mut data = result.data.into_json()?;
    let data = data.as_object_mut().unwrap().remove("createUser");
    w.variables.push(data.unwrap());
    Ok(())
}

#[when(
    expr = "should not be able to create user at institution with username {string} as owner again"
)]
pub async fn should_not_be_able_to_create_user_at_institution_again(
    w: &mut World,
    username: String,
) -> anyhow::Result<()> {
    let result = create_institution_user(w, username).await;
    assert!(result.is_err());
    let err = result.errors.first().unwrap();
    let expected = serde_json::to_string(&serde_json::json!({
        "code": 409
    }))?;
    assert_eq!(serde_json::to_string(&err.extensions)?, expected);
    Ok(())
}

#[then(expr = "user is created for institution {string} with username {string} as owner")]
pub async fn user_is_created_for_institution(
    w: &mut World,
    institution: String,
    username: String,
) -> anyhow::Result<()> {
    let user = J(create_user_payload(&username));
    let data: serde_json::Value = w.variables.pop().unwrap();
    assert_eq!(
        JRef(data.pointer("/details").unwrap()).s("username"),
        user.s("username"),
    );
    assert_eq!(
        JRef(data.pointer("/details/institution").unwrap()).s("name"),
        institution.as_str(),
    );
    Ok(())
}
