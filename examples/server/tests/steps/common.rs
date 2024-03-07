use async_graphql::ErrorExtensionValues;
use cucumber::{given, then};

use crate::world::World;

#[given(expr = "Admin user")]
pub async fn admin_user(w: &mut World) -> anyhow::Result<()> {
    let username = w
        .store()
        .keycloak()
        .config()
        .realm_admin_username()
        .to_string();
    let password = w
        .store()
        .keycloak()
        .config()
        .realm_admin_password()
        .to_string();
    w.switch_user(username, password).await?;
    Ok(())
}

#[given(expr = "Without user")]
pub async fn without_user(w: &mut World) -> anyhow::Result<()> {
    w.clear_user().await?;
    Ok(())
}

#[then(expr = "the response should contain the error extension code {int}")]
pub async fn the_response_should_contain_the_error_extension_code(
    w: &mut World,
    code: i32,
) -> anyhow::Result<()> {
    let extensions = w.last_response_error_extensions()?;
    let mut expected = ErrorExtensionValues::default();
    expected.set("code", code);
    assert_eq!(extensions.get("code"), expected.get("code"));
    Ok(())
}

#[then(expr = "the response should contain the error extension type {string}")]
pub async fn the_response_should_contain_the_error_extension_type(
    w: &mut World,
    ty: String,
) -> anyhow::Result<()> {
    let extensions = w.last_response_error_extensions()?;
    let mut expected = ErrorExtensionValues::default();
    expected.set("type", &ty);
    assert_eq!(extensions.get("type"), expected.get("type"));
    Ok(())
}

#[then(expr = "the response should contain the error extension field {string}")]
pub async fn the_response_should_contain_the_error_extension_field(
    w: &mut World,
    field: String,
) -> anyhow::Result<()> {
    let extensions = w.last_response_error_extensions()?;
    let mut expected = ErrorExtensionValues::default();
    expected.set("field", &field);
    assert_eq!(extensions.get("field"), expected.get("field"));
    Ok(())
}
