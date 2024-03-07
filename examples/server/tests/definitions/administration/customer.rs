use crate::utils::str::str_cap;
use crate::world::World;
use cucumber::{then, when};
use serde_json::json;

fn create_user_payload(name: &str) -> serde_json::Value {
    let username = format!("{name}.owner");
    let mut s = username.split('.');
    let firstname = str_cap(s.next().unwrap().trim());
    // let lastname = str_cap(s.next().unwrap().trim());
    let email = format!("{}@qm-test.live", username);
    let password = format!("{firstname}123!");
    // let enabled = false;
    json!({
        "username": &username,
        "password": &password,
        "email": &email,
    })
}

pub async fn create_customer(w: &mut World, name: &str) {
    let response = w
        .graphql(
            "createCustomer",
            include_str!("../../graphql/mutation/createCustomer/createCustomer.gql"),
        )
        .var(
            "input",
            json!({
                "initialUser": &create_user_payload(name),
                "name": name,
            }),
        )
        .execute()
        .await;
    w.save_last_response(response);
}

#[when(expr = "creating Customer with the name {string}")]
pub async fn create_customer_with_the_name(w: &mut World, name: String) -> anyhow::Result<()> {
    create_customer(w, &name).await;
    Ok(())
}

#[then(expr = "created Customer has name {string}")]
pub async fn created_customer_has_name(w: &mut World, name: String) -> anyhow::Result<()> {
    assert_eq!(w.last_response_data_field("name").as_str(), &name);
    Ok(())
}

#[then(expr = "creating Customer with the name {string} again")]
pub async fn create_customer_with_the_name_again(
    w: &mut World,
    name: String,
) -> anyhow::Result<()> {
    create_customer(w, &name).await;
    Ok(())
}
