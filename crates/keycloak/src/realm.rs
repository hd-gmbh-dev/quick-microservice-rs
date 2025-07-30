use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::identity;

use crate::{
    schema::UserInput,
    validation::{
        context::{Config, ValidationContext},
        updater::{get_smtp_server_defaults, update_for_errors},
        validator::validate_realm,
    },
    CredentialRepresentation, GroupRepresentation, Keycloak, KeycloakError, RealmRepresentation,
    RoleRepresentation, UserRepresentation,
};

use qm_role::Group;

lazy_static::lazy_static! {
    static ref REALM_TEMPLATE: RealmRepresentation = serde_json::from_str(include_str!("../templates/realm.json")).unwrap();
    static ref APP_URL: String = std::env::var("SERVER_APP_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
}

pub fn app_url() -> &'static str {
    APP_URL.as_str()
}

pub async fn create(keycloak: &Keycloak) -> anyhow::Result<()> {
    create_custom(keycloak, identity).await
}

pub async fn create_custom<T>(keycloak: &Keycloak, realm_repr_transform: T) -> anyhow::Result<()>
where
    T: Fn(RealmRepresentation) -> RealmRepresentation,
{
    let realm = keycloak.config().realm();
    let client_id = keycloak.config().client_id();
    let url = APP_URL.as_str();
    let mut realm_representation = REALM_TEMPLATE.clone();
    realm_representation.realm = Some(realm.to_string());
    if let Some(client) = realm_representation.clients.as_mut().and_then(|c| {
        c.iter_mut()
            .find(|c| c.client_id.as_deref() == Some(client_id))
    }) {
        client.redirect_uris = Some(vec![format!(
            "{}*",
            if url.chars().filter(|c| c == &':').count() > 1 {
                url.rsplit_once(':').map(|(l, _)| l).unwrap_or(url)
            } else {
                &url
            }
        )]);
        client.base_url = Some(format!("{}/", &url));
        client.root_url = Some(format!("{}/", &url));
        client.direct_access_grants_enabled = Some(true);
    }
    let ctx = ValidationContext {
        config: &Config {
            realm,
            client_id,
            keycloak: keycloak.config(),
            public_url: url,
        },
        keycloak,
    };
    realm_representation.smtp_server = get_smtp_server_defaults(&ctx);
    tracing::info!("create keycloak realm '{realm}'");
    keycloak
        .create_realm(realm_repr_transform(realm_representation))
        .await?;
    Ok(())
}

pub async fn configure_realm<R, P>(
    keycloak: &Keycloak,
    groups: Vec<Group<R, P>>,
) -> anyhow::Result<()>
where
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
{
    let realm = keycloak.config().realm();
    let client_id = keycloak.config().client_id();
    let url = APP_URL.as_str();
    let keycloak_config = keycloak.config();
    let ctx = ValidationContext {
        config: &Config {
            realm,
            client_id,
            keycloak: keycloak_config,
            public_url: url,
        },
        keycloak,
    };
    let max_tries = 5;
    let mut current_try = 1;
    while let Some(errors) = validate_realm(&ctx).await? {
        if errors.is_empty() {
            break;
        } else {
            for error in errors.iter() {
                tracing::error!("{}", error.id);
            }
        }
        tracing::info!(
            "{current_try}. try time to update realm {} for errors {}",
            realm,
            errors.len()
        );
        update_for_errors(&ctx, errors.into_iter().map(From::from).collect()).await?;
        current_try += 1;
        if current_try > max_tries {
            break;
        }
    }
    ensure_groups_with_roles(realm, keycloak, groups, true).await?;
    Ok(())
}

fn set_attributes(attributes: HashMap<&str, Option<String>>, u: &mut UserRepresentation) {
    if u.attributes.is_none() {
        u.attributes = Some(HashMap::new());
    }

    if let Some(a) = u.attributes.as_mut() {
        // Loop all attributes possible
        for (key, value) in attributes.into_iter() {
            if let Some(v) = value {
                a.insert(
                    key.to_string(),
                    v.split(',').map(|v| v.trim().to_string()).collect(),
                );
            } else {
                a.remove(key);
            }
        }
    }
}

pub async fn create_keycloak_user(
    realm: &str,
    keycloak: &Keycloak,
    user: UserInput,
) -> anyhow::Result<(UserRepresentation, bool)> {
    let mut keycloak_user: UserRepresentation = UserRepresentation {
        access: None,
        attributes: None,
        client_consents: None,
        client_roles: None,
        created_timestamp: None,
        credentials: None,
        disableable_credential_types: None,
        email: Some(user.email),
        email_verified: None,
        enabled: user.enabled,
        federated_identities: None,
        federation_link: None,
        first_name: Some(user.firstname),
        groups: None,
        id: None,
        last_name: Some(user.lastname),
        not_before: None,
        origin: None,
        realm_roles: None,
        // Some(vec!["UPDATE_PASSWORD".to_string()]),
        required_actions: user
            .required_actions
            .as_ref()
            .map(|actions| actions.iter().map(|action| action.to_string()).collect()),
        self_: None,
        service_account_client_id: None,
        username: Some(user.username.clone()),
        totp: None,
        user_profile_metadata: None,
        ..Default::default()
    };

    set_attributes(
        HashMap::from([
            ("phone", user.phone),
            ("salutation", user.salutation),
            ("fax", user.fax),
            ("room-number", user.room_number),
            ("job-title", user.job_title),
        ]),
        &mut keycloak_user,
    );

    // Set the credential
    keycloak_user.credentials = Some(vec![CredentialRepresentation {
        created_date: None,
        credential_data: None,
        id: None,
        priority: None,
        secret_data: None,
        temporary: user
            .required_actions
            .as_ref()
            .map(|actions| actions.contains(&crate::schema::RequiredUserAction::UpdatePassword)),
        type_: Some("password".to_string()),
        user_label: None,
        value: Some(user.password),
        ..Default::default()
    }]);

    let result = keycloak.create_user(realm, keycloak_user).await;
    let exists = match result {
        Ok(_) => Ok(false),
        Err(err) => match err {
            KeycloakError::HttpFailure { status: 409, .. } => anyhow::Ok(true),
            _ => {
                tracing::error!("{err:#?}");
                Err(err)?
            }
        },
    }?;

    let k_user = keycloak.user_by_username(realm, user.username).await?;
    Ok((k_user.unwrap(), exists))
}

pub async fn get_keycloak_user(
    realm: &str,
    keycloak: &Keycloak,
    user_id: &str,
) -> anyhow::Result<UserRepresentation, anyhow::Error> {
    keycloak
        .user_by_id(realm, user_id)
        .await?
        .ok_or(anyhow::format_err!("unable to get user from keycloak"))
}

pub async fn ensure_roles(
    realm: &str,
    keycloak: &Keycloak,
    role_set: BTreeSet<String>,
) -> anyhow::Result<Vec<RoleRepresentation>> {
    let mut roles = vec![];
    for role in role_set {
        match keycloak.realm_role_by_name(realm, &role).await {
            Ok(existing_role) => {
                roles.push(existing_role);
            }
            Err(KeycloakError::HttpFailure { status: 404, .. }) => {
                match keycloak
                    .create_role(
                        realm,
                        RoleRepresentation {
                            name: Some(role.clone()),
                            ..RoleRepresentation::default()
                        },
                    )
                    .await
                {
                    Ok(_) => {
                        roles.push(keycloak.realm_role_by_name(realm, &role).await?);
                    }
                    Err(err) => {
                        tracing::error!("{err:#?}");
                        return Err(err.into());
                    }
                }
            }
            Err(err) => {
                tracing::error!("{err:#?}");
                return Err(err.into());
            }
        }
    }
    Ok(roles)
}

pub async fn ensure_groups<R, P>(
    realm: &str,
    keycloak: &Keycloak,
    group_map: &BTreeMap<String, Group<R, P>>,
    built_in: bool,
) -> anyhow::Result<BTreeMap<String, GroupRepresentation>>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    let mut groups: BTreeMap<String, GroupRepresentation> = BTreeMap::new();
    for (_, group) in group_map.iter() {
        let s = group
            .path
            .split('/')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();
        let l = s.len();
        let mut path = "".to_string();
        for i in 0..l {
            let part = s.get(i).unwrap();
            if i > 0 {
                let parent_group = groups.get(&path).unwrap();
                path += &format!("/{part}");
                if !groups.contains_key(&path) {
                    let allowed_types = group
                        .allowed_types()
                        .iter()
                        .map(|v| v.as_ref())
                        .collect::<Vec<&str>>()
                        .join(",");

                    if let Ok(existing) = keycloak.group_by_path(realm, &path).await {
                        groups.insert(path.clone(), existing);
                        continue;
                    }

                    let result: Result<(), KeycloakError> = keycloak
                        .create_sub_group_with_id(
                            realm,
                            parent_group.id.as_deref().unwrap(),
                            GroupRepresentation {
                                name: Some(part.to_string()),
                                attributes: Some(if built_in {
                                    HashMap::from_iter([
                                        ("built_in".to_string(), vec!["1".to_string()]),
                                        ("display_name".to_string(), vec![group.name.to_string()]),
                                        ("allowed_types".to_string(), vec![allowed_types]),
                                    ])
                                } else {
                                    HashMap::from_iter([
                                        ("display_name".to_string(), vec![group.name.to_string()]),
                                        ("allowed_types".to_string(), vec![allowed_types]),
                                    ])
                                }),
                                ..Default::default()
                            },
                        )
                        .await;
                    match result {
                        Ok(_) => {
                            groups
                                .insert(path.clone(), keycloak.group_by_path(realm, &path).await?);
                        }
                        Err(err) => match err {
                            KeycloakError::HttpFailure { status: 409, .. } => {
                                groups.insert(
                                    path.clone(),
                                    keycloak.group_by_path(realm, &path).await?,
                                );
                            }
                            _ => {
                                tracing::error!("{err:#?}");
                                Err(err)?
                            }
                        },
                    }
                }
            } else {
                let parent_path = format!("/{}", part);
                if !groups.contains_key(&parent_path) {
                    if let Ok(existing) = keycloak.group_by_path(realm, &parent_path).await {
                        groups.insert(parent_path.clone(), existing);
                        path = parent_path;
                        continue;
                    }

                    let result = keycloak
                        .create_group(
                            realm,
                            GroupRepresentation {
                                name: Some(part.to_string()),
                                ..Default::default()
                            },
                        )
                        .await;
                    match result {
                        Ok(_) => {
                            groups.insert(
                                parent_path.clone(),
                                keycloak.group_by_path(realm, &parent_path).await?,
                            );
                        }
                        Err(err) => match err {
                            KeycloakError::HttpFailure { status: 409, .. } => {
                                groups.insert(
                                    parent_path.clone(),
                                    keycloak.group_by_path(realm, &parent_path).await?,
                                );
                            }
                            _ => {
                                tracing::error!("{err:#?}");
                                Err(err)?
                            }
                        },
                    }
                }
                path = parent_path;
            }
        }
    }
    Ok(groups)
}

pub async fn ensure_group_role_mappings<R, P>(
    realm: &str,
    keycloak: &Keycloak,
    groups: &BTreeMap<String, GroupRepresentation>,
    group_map: &BTreeMap<String, Group<R, P>>,
    existing_roles: &[RoleRepresentation],
) -> anyhow::Result<()>
where
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
{
    for group in group_map.values() {
        if let Some(group_rep) = groups.get(&group.path) {
            let roles = group.resources();
            keycloak
                .create_realm_role_mappings_by_group_id(
                    realm,
                    group_rep.id.as_deref().unwrap(),
                    existing_roles
                        .iter()
                        .filter(|role_rep| roles.iter().any(|r| Some(r) == role_rep.name.as_ref()))
                        .cloned()
                        .collect(),
                )
                .await
                .map_err(|e| {
                    tracing::error!("{e:#?}");
                    e
                })?;
        }
    }
    Ok(())
}

pub async fn ensure_groups_with_roles<R, P>(
    realm: &str,
    keycloak: &Keycloak,
    groups: Vec<Group<R, P>>,
    built_in: bool,
) -> anyhow::Result<BTreeMap<String, GroupRepresentation>>
where
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
{
    let mut group_map = BTreeMap::new();
    let mut role_set = BTreeSet::new();
    for group in groups {
        for role in group.resources() {
            role_set.insert(role);
        }
        group_map.insert(group.path.clone(), group);
    }
    let roles = ensure_roles(realm, keycloak, role_set).await?;
    let groups = ensure_groups(realm, keycloak, &group_map, built_in).await?;
    ensure_group_role_mappings(realm, keycloak, &groups, &group_map, &roles).await?;
    Ok(groups)
}

pub async fn create_user_with_groups(
    realm: &str,
    keycloak: &Keycloak,
    user: UserInput,
    user_groups: Vec<String>,
    group_map: Option<BTreeMap<String, GroupRepresentation>>,
) -> anyhow::Result<UserRepresentation> {
    let (user, _) = create_keycloak_user(realm, keycloak, user).await?;
    if let Some(groups) = group_map {
        for user_group in user_groups.iter() {
            if let Some(group) = groups.get(user_group) {
                keycloak
                    .add_user_to_group(
                        realm,
                        user.id.as_deref().unwrap(),
                        group.id.as_deref().unwrap(),
                    )
                    .await?;
            }
        }
    } else {
        for user_group in user_groups {
            let group = keycloak.group_by_path(realm, &user_group).await?;
            keycloak
                .add_user_to_group(
                    realm,
                    user.id.as_deref().unwrap(),
                    group.id.as_deref().unwrap(),
                )
                .await?;
        }
    }
    Ok(user)
}

pub async fn ensure_admin_user<R, P>(
    realm: &str,
    keycloak: &Keycloak,
    username: &str,
    password: &str,
    email: &str,
    admin_group: Group<R, P>,
    role_set: BTreeSet<String>,
) -> anyhow::Result<UserRepresentation>
where
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
{
    tracing::info!("ensure admin user");
    let admin_user = keycloak
        .user_by_username(realm, username.to_string())
        .await
        .map_err(|e| {
            tracing::error!("{e:#?}");
            e
        })?;
    if let Some(user) = admin_user {
        Ok(user)
    } else {
        ensure_roles(realm, keycloak, role_set).await?;
        let user_groups = vec![admin_group.path.clone()];
        let groups = vec![admin_group];
        let group_map = ensure_groups_with_roles(realm, keycloak, groups, true).await?;
        let firstname = realm.to_string();
        let lastname = "Admin".to_string();
        let username = username.to_string();
        let password = password.to_string();
        let email = email.to_string();
        create_user_with_groups(
            realm,
            keycloak,
            UserInput {
                username: username.clone(),
                firstname,
                lastname,
                password,
                email,
                phone: None,
                salutation: None,
                fax: None,
                room_number: None,
                job_title: None,
                required_actions: None,
                enabled: Some(true),
            },
            user_groups,
            Some(group_map),
        )
        .await
    }
}

pub(crate) fn realm_template() -> &'static RealmRepresentation {
    &REALM_TEMPLATE
}
