use std::collections::HashMap;

use serde_json::Value;

use crate::validation::context::ValidationContext as Ctx;
use crate::validation::model::RealmConfigError;
use crate::validation::realm_errors;
use crate::{ClientRepresentation, RealmRepresentation};

pub async fn validate_realm(ctx: &Ctx<'_>) -> anyhow::Result<Option<Vec<RealmConfigError>>> {
    let mut errors = vec![];
    let realm = ctx.cfg().realm();
    log::info!("validating realm '{realm}'");
    check_realm_settings(ctx, realm, &mut errors).await?;
    check_client(ctx, realm, &mut errors).await?;
    Ok(Some(errors))
}

async fn check_realm_settings(
    ctx: &Ctx<'_>,
    realm: &str,
    errors: &mut Vec<RealmConfigError>,
) -> anyhow::Result<()> {
    let rep: RealmRepresentation = ctx.keycloak().realm_by_name(realm).await?;

    // default_locale must be `de`
    if let Some(locale) = &rep.default_locale {
        if locale != "de" {
            add_error(
                realm_errors::REALM_DEFAULT_LOCALE_INVALID_ID,
                realm_errors::REALM_DEFAULT_LOCALE_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_DEFAULT_LOCALE_MISSING_ID,
            realm_errors::REALM_DEFAULT_LOCALE_MISSING_KEY,
            errors,
        );
    }
    // internationalization_enabled must be `true`
    if !rep.internationalization_enabled.unwrap_or(false) {
        add_error(
            realm_errors::REALM_INTERNATIONALIZATION_ENABLED_ID,
            realm_errors::REALM_INTERNATIONALIZATION_ENABLED_KEY,
            errors,
        );
    }
    // login_theme must be `qm`
    if let Some(theme) = &rep.login_theme {
        if theme != ctx.keycloak().config().theme() {
            add_error(
                realm_errors::REALM_LOGIN_THEME_INVALID_ID,
                realm_errors::REALM_LOGIN_THEME_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_LOGIN_THEME_MISSING_ID,
            realm_errors::REALM_LOGIN_THEME_MISSING_KEY,
            errors,
        );
    }
    // password_policy must contain `length(8)`, `specialChars(1)`, `upperCase(1)`, `lowerCase(1)`, `digits(1)`
    if let Some(policy) = &rep.password_policy {
        if !policy.contains("length(8)") {
            add_error(
                realm_errors::REALM_PASSWORD_POLICY_LENGTH_ID,
                realm_errors::REALM_PASSWORD_POLICY_LENGTH_KEY,
                errors,
            );
        }
        if !policy.contains("specialChars(1)") {
            add_error(
                realm_errors::REALM_PASSWORD_POLICY_SYMBOL_ID,
                realm_errors::REALM_PASSWORD_POLICY_SYMBOL_KEY,
                errors,
            );
        }
        if !policy.contains("upperCase(1)") {
            add_error(
                realm_errors::REALM_PASSWORD_POLICY_UPPERCASE_ID,
                realm_errors::REALM_PASSWORD_POLICY_UPPERCASE_KEY,
                errors,
            );
        }
        if !policy.contains("lowerCase(1)") {
            add_error(
                realm_errors::REALM_PASSWORD_POLICY_LOWERCASE_ID,
                realm_errors::REALM_PASSWORD_POLICY_LOWERCASE_KEY,
                errors,
            );
        }
        if !policy.contains("digits(1)") {
            add_error(
                realm_errors::REALM_PASSWORD_POLICY_DIGIT_ID,
                realm_errors::REALM_PASSWORD_POLICY_DIGIT_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_PASSWORD_POLICY_MISSING_ID,
            realm_errors::REALM_PASSWORD_POLICY_MISSING_KEY,
            errors,
        );
    }
    // remember_me must be `true`
    if !rep.remember_me.unwrap_or(false) {
        add_error(
            realm_errors::REALM_REMEMBER_ME_ID,
            realm_errors::REALM_REMEMBER_ME_KEY,
            errors,
        );
    }
    // registration_allowed must be `false`
    if rep.registration_allowed.unwrap_or(false) {
        add_error(
            realm_errors::REALM_REGISTRATION_ALLOWED_ID,
            realm_errors::REALM_REGISTRATION_ALLOWED_KEY,
            errors,
        );
    }
    // reset_password_allowed must be `true`
    if !rep.reset_password_allowed.unwrap_or(false) {
        add_error(
            realm_errors::REALM_RESET_PASSWORD_ALLOWED_ID,
            realm_errors::REALM_RESET_PASSWORD_ALLOWED_KEY,
            errors,
        );
    }
    // supported_locales must contain `de`
    if let Some(locales) = &rep.supported_locales {
        if !locales.contains(&"de".to_string()) {
            add_error(
                realm_errors::REALM_SUPPORTED_LOCALES_INVALID_ID,
                realm_errors::REALM_SUPPORTED_LOCALES_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_SUPPORTED_LOCALES_MISSING_ID,
            realm_errors::REALM_SUPPORTED_LOCALES_MISSING_KEY,
            errors,
        );
    }
    // smtp_server must be configured
    if let Some(smtp_server) = &rep.smtp_server {
        check_realm_smtp_settings(ctx, smtp_server, errors);
    } else {
        add_error(
            realm_errors::REALM_SMTP_SERVER_MISSING_ID,
            realm_errors::REALM_SMTP_SERVER_MISSING_KEY,
            errors,
        );
    }

    Ok(())
}

async fn check_client(
    ctx: &Ctx<'_>,
    realm: &str,
    errors: &mut Vec<RealmConfigError>,
) -> anyhow::Result<()> {
    // clients must have `spa`
    let rep: Option<ClientRepresentation> = ctx
        .keycloak()
        .get_client(realm) // Hardcoded only gets `spa`
        .await?;

    if let Some(client) = rep {
        // attribute `oauth2.device.authorization.grant.enabled` must be `false`
        // Note that the field `ClientRepresentation::oauth2_device_authorization_grant_enabled` is not used
        if let Some(attributes) = &client.attributes {
            let oauth2_device_authorization_opt =
                attributes.get("oauth2.device.authorization.grant.enabled");
            let backchannel_logout_opt = attributes.get("backchannel.logout.url");
            if let Some(oauth2_device_authorization) = oauth2_device_authorization_opt {
                // Attribute values are always String
                if oauth2_device_authorization.as_str() != Some("false") {
                    add_error(
                        realm_errors::CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_INVALID_ID,
                        realm_errors::CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_INVALID_KEY,
                        errors,
                    );
                }
            } else {
                add_error(
                    realm_errors::CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_MISSING_ID,
                    realm_errors::CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_MISSING_KEY,
                    errors,
                );
            }
            if let Some(backchannel_logout) = backchannel_logout_opt {
                if backchannel_logout.as_str().unwrap_or("").is_empty() {
                    add_error(
                        realm_errors::CLIENTS_CLIENT_ATTRIBUTES_BACKCHANNEL_LOGOUT_DISABLED_ID,
                        realm_errors::CLIENTS_CLIENT_ATTRIBUTES_BACKCHANNEL_LOGOUT_DISABLED_KEY,
                        errors,
                    )
                }
            } else {
                add_error(
                    realm_errors::CLIENTS_CLIENT_ATTRIBUTES_BACKCHANNEL_LOGOUT_DISABLED_ID,
                    realm_errors::CLIENTS_CLIENT_ATTRIBUTES_BACKCHANNEL_LOGOUT_DISABLED_KEY,
                    errors,
                )
            }
        } else {
            add_error(
                realm_errors::CLIENTS_CLIENT_ATTRIBUTES_MISSING_ID,
                realm_errors::CLIENTS_CLIENT_ATTRIBUTES_MISSING_KEY,
                errors,
            );
        }
        // base_url must be the configured value
        if let Some(url) = &client.base_url {
            if url.trim_end_matches('/') != ctx.cfg().public_url().trim_end_matches('/') {
                log::info!(
                    "[{}]: Expected the 'base_url' value to be '{}' but was '{}'",
                    realm,
                    ctx.cfg().public_url().trim_end_matches('/'),
                    url.trim_end_matches('/')
                );
                add_error(
                    realm_errors::CLIENTS_CLIENT_BASE_URL_INVALID_ID,
                    realm_errors::CLIENTS_CLIENT_BASE_URL_INVALID_KEY,
                    errors,
                );
            }
        } else {
            add_error(
                realm_errors::CLIENTS_CLIENT_BASE_URL_MISSING_ID,
                realm_errors::CLIENTS_CLIENT_BASE_URL_MISSING_KEY,
                errors,
            );
        }
        // client_id must be `spa`
        if client.client_id.unwrap_or_default() != "spa" {
            add_error(
                realm_errors::CLIENTS_CLIENT_CLIENT_ID_ID,
                realm_errors::CLIENTS_CLIENT_CLIENT_ID_KEY,
                errors,
            );
        }
        // consent_required must be `false`
        if client.consent_required.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_CONSENT_REQUIRED_ID,
                realm_errors::CLIENTS_CLIENT_CONSENT_REQUIRED_KEY,
                errors,
            );
        }
        // direct_access_grants_enabled must be `false`
        /*if client.direct_access_grants_enabled.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_DIRECT_ACCESS_GRANT_ENABLED_ID,
                realm_errors::CLIENTS_CLIENT_DIRECT_ACCESS_GRANT_ENABLED_KEY,
                errors,
            );
        }*/
        // enabled mut be `true`
        if !client.enabled.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_ENABLED_ID,
                realm_errors::CLIENTS_CLIENT_ENABLED_KEY,
                errors,
            );
        }
        // implicit_flow_enabled must be `false`
        if client.implicit_flow_enabled.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_IMPLICIT_FLOW_ENABLED_ID,
                realm_errors::CLIENTS_CLIENT_IMPLICIT_FLOW_ENABLED_KEY,
                errors,
            );
        }
        // public_client must be `true`
        if !client.public_client.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_PUBLIC_CLIENT_ID,
                realm_errors::CLIENTS_CLIENT_PUBLIC_CLIENT_KEY,
                errors,
            );
        }
        // redirect_uris must contain a pattern matching the configured value
        if let Some(urls) = &client.redirect_uris {
            if !urls.iter().all(|url| {
                url == ctx.cfg().public_url() || url.replace("*", "") == ctx.cfg().public_url()
            }) {
                log::info!(
                    "[{}]: Expected the 'redirect_uris' values '{:?}' to contain a pattern that matches '{}'",
                    realm,
                    urls,
                    ctx.cfg().public_url()
                );
                add_error(
                    realm_errors::CLIENTS_CLIENT_REDIRECT_URIS_INVALID_ID,
                    realm_errors::CLIENTS_CLIENT_REDIRECT_URIS_INVALID_KEY,
                    errors,
                );
            }
        } else {
            add_error(
                realm_errors::CLIENTS_CLIENT_REDIRECT_URIS_MISSING_ID,
                realm_errors::CLIENTS_CLIENT_REDIRECT_URIS_MISSING_KEY,
                errors,
            );
        }
        // root_url must be the configured value
        if let Some(url) = &client.root_url {
            if url.trim_end_matches('/') != ctx.cfg().public_url().trim_end_matches('/') {
                log::info!(
                    "[{}]: Expected the 'root_url' value to be '{}' but was '{}'",
                    realm,
                    ctx.cfg().public_url().trim_end_matches('/'),
                    url.trim_end_matches('/')
                );
                add_error(
                    realm_errors::CLIENTS_CLIENT_ROOT_URL_INVALID_ID,
                    realm_errors::CLIENTS_CLIENT_ROOT_URL_INVALID_KEY,
                    errors,
                );
            }
        } else {
            add_error(
                realm_errors::CLIENTS_CLIENT_ROOT_URL_MISSING_ID,
                realm_errors::CLIENTS_CLIENT_ROOT_URL_MISSING_KEY,
                errors,
            );
        }
        // service_accounts_enabled must be `false`
        if client.service_accounts_enabled.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_SERVICE_ACCOUNTS_ENABLED_ID,
                realm_errors::CLIENTS_CLIENT_SERVICE_ACCOUNTS_ENABLED_KEY,
                errors,
            );
        }
        // standard_flow_enabled must be `true`
        if !client.standard_flow_enabled.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_STANDARD_FLOW_ENABLED_ID,
                realm_errors::CLIENTS_CLIENT_STANDARD_FLOW_ENABLED_KEY,
                errors,
            );
        }
        // frontchannel logout must be false
        if client.frontchannel_logout.unwrap_or(false) {
            add_error(
                realm_errors::CLIENTS_CLIENT_FRONTCHANNEL_LOGOUT_ENABLED_ID,
                realm_errors::CLIENTS_CLIENT_FRONTCHANNEL_LOGOUT_ENABLED_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::CLIENTS_CLIENT_MISSING_ID,
            realm_errors::CLIENTS_CLIENT_MISSING_KEY,
            errors,
        );
    }
    Ok(())
}

fn add_error<S>(error_id: S, error_key: S, errors: &mut Vec<RealmConfigError>)
where
    S: Into<String>,
{
    errors.push(RealmConfigError::new(error_id.into(), error_key.into()));
}

fn check_realm_smtp_settings(
    ctx: &Ctx<'_>,
    smtp_server: &HashMap<String, Value>,
    errors: &mut Vec<RealmConfigError>,
) {
    if let Some(configured_reply_to_display_name) =
        ctx.cfg().keycloak().smtp_reply_to_display_name()
    {
        // reply_to_display_name must be the configured value
        if let Some(reply_to_display_name_value) = smtp_server.get("replyToDisplayName") {
            let reply_to_display_name = get_string_from_value(reply_to_display_name_value);
            if configured_reply_to_display_name != reply_to_display_name {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_REPLY_TO_DISPLAY_NAME' '{}' does not match with the value from keycloak '{}'",
                    configured_reply_to_display_name,
                    reply_to_display_name
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISMATCHED_KEY,
                    errors,
                );
            }
        } else {
            add_error(
                realm_errors::REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISSING_ID,
                realm_errors::REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISSING_KEY,
                errors,
            );
        }
    }

    if let Some(starttls_value) = smtp_server.get("starttls") {
        // starttls must be the configured value or `false` if not configured
        let starttls = get_bool_from_string_value(starttls_value);
        if let Some(configured_starttls) = ctx.cfg().keycloak().smtp_starttls() {
            if configured_starttls != &starttls {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_STARTTLS' '{}' does not match with the value from keycloak '{}'",
                    configured_starttls,
                    starttls
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_STARTTLS_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_STARTTLS_MISMATCHED_KEY,
                    errors,
                );
            }
        } else if starttls {
            add_error(
                realm_errors::REALM_SMTP_SERVER_STARTTLS_INVALID_ID,
                realm_errors::REALM_SMTP_SERVER_STARTTLS_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_SMTP_SERVER_STARTTLS_MISSING_ID,
            realm_errors::REALM_SMTP_SERVER_STARTTLS_MISSING_KEY,
            errors,
        );
    }

    if let Some(port_value) = smtp_server.get("port") {
        // port must be the configured value or `1025` if not configured
        let port = get_u16_from_value(port_value);
        if let Some(configured_port) = ctx.cfg().keycloak().smtp_port() {
            if configured_port != &port {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_PORT' '{}' does not match with the value from keycloak '{}'",
                    configured_port,
                    port
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_PORT_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_PORT_MISMATCHED_KEY,
                    errors,
                );
            }
        } else if port != 1025 {
            add_error(
                realm_errors::REALM_SMTP_SERVER_PORT_INVALID_ID,
                realm_errors::REALM_SMTP_SERVER_PORT_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_SMTP_SERVER_PORT_MISSING_ID,
            realm_errors::REALM_SMTP_SERVER_PORT_MISSING_KEY,
            errors,
        );
    }

    if let Some(host_value) = smtp_server.get("host") {
        // port must be the configured value or `smtp` if not configured
        let host = get_string_from_value(host_value);
        if let Some(configured_host) = ctx.cfg().keycloak().smtp_host() {
            if configured_host != host {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_HOST' '{}' does not match with the value from keycloak '{}'",
                    configured_host,
                    host
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_HOST_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_HOST_MISMATCHED_KEY,
                    errors,
                );
            }
        } else if host != "smtp" {
            add_error(
                realm_errors::REALM_SMTP_SERVER_HOST_INVALID_ID,
                realm_errors::REALM_SMTP_SERVER_HOST_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_SMTP_SERVER_HOST_MISSING_ID,
            realm_errors::REALM_SMTP_SERVER_HOST_MISSING_KEY,
            errors,
        );
    }

    if let Some(configured_reply_to) = ctx.cfg().keycloak().smtp_reply_to() {
        // reply_to must be the configured value
        if let Some(reply_to_value) = smtp_server.get("replyTo") {
            let reply_to = get_string_from_value(reply_to_value);
            if configured_reply_to != reply_to {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_REPLY_TO' '{}' does not match with the value in keycloak '{}'",
                    configured_reply_to,
                    reply_to
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_REPLY_TO_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_REPLY_TO_MISMATCHED_KEY,
                    errors,
                );
            }
        } else {
            add_error(
                realm_errors::REALM_SMTP_SERVER_REPLY_TO_MISSING_ID,
                realm_errors::REALM_SMTP_SERVER_REPLY_TO_MISSING_KEY,
                errors,
            );
        }
    }

    if let Some(from_value) = smtp_server.get("from") {
        // from must be the configured value or `noreply@qm.local` if not configured
        let from = get_string_from_value(from_value);
        if let Some(configured_from) = ctx.cfg().keycloak().smtp_from() {
            if configured_from != from {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_FROM' '{}' does not match with the value from keycloak '{}'",
                    configured_from,
                    from
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_FROM_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_FROM_MISMATCHED_KEY,
                    errors,
                );
            }
        } else if from != "noreply@qm.local" {
            add_error(
                realm_errors::REALM_SMTP_SERVER_FROM_INVALID_ID,
                realm_errors::REALM_SMTP_SERVER_FROM_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_SMTP_SERVER_FROM_MISSING_ID,
            realm_errors::REALM_SMTP_SERVER_FROM_MISSING_KEY,
            errors,
        );
    }

    if let Some(configured_from_display_name) = ctx.cfg().keycloak().smtp_from_display_name() {
        // reply_to_display_name must be the configured value
        if let Some(from_display_name_value) = smtp_server.get("fromDisplayName") {
            let from_display_name = get_string_from_value(from_display_name_value);
            if configured_from_display_name != from_display_name {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_FROM_DISPLAY_NAME' '{}' does not match with the value in keycloak '{}'",
                    configured_from_display_name,
                    from_display_name
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISMATCHED_KEY,
                    errors,
                );
            }
        } else {
            add_error(
                realm_errors::REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISSING_ID,
                realm_errors::REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISSING_KEY,
                errors,
            );
        }
    }

    if let Some(ssl_value) = smtp_server.get("ssl") {
        // ssl must be the configured value or `false` if not configured
        let ssl = get_bool_from_string_value(ssl_value);
        if let Some(configured_ssl) = ctx.cfg().keycloak().smtp_ssl() {
            if configured_ssl != &ssl {
                log::info!(
                    "The configured 'KEYCLOAK_SMTP_SSL' '{}' does not match with the value from keycloak '{}'",
                    configured_ssl,
                    ssl
                );
                add_error(
                    realm_errors::REALM_SMTP_SERVER_SSL_MISMATCHED_ID,
                    realm_errors::REALM_SMTP_SERVER_SSL_MISMATCHED_KEY,
                    errors,
                );
            }
        } else if ssl {
            add_error(
                realm_errors::REALM_SMTP_SERVER_SSL_INVALID_ID,
                realm_errors::REALM_SMTP_SERVER_SSL_INVALID_KEY,
                errors,
            );
        }
    } else {
        add_error(
            realm_errors::REALM_SMTP_SERVER_SSL_MISSING_ID,
            realm_errors::REALM_SMTP_SERVER_SSL_MISSING_KEY,
            errors,
        );
    }
}

/// Gets a String from a [Value].
///
/// Will return an empty string if no value is present.
fn get_string_from_value(value: &Value) -> String {
    serde_json::from_value::<String>(value.clone()).unwrap_or("".to_string())
}

/// Gets a bool from a [Value].
///
/// Will return `true` if the value contains the string "true".
/// Returns `false` otherwise.
fn get_bool_from_string_value(value: &Value) -> bool {
    matches!(get_string_from_value(value).as_str(), "true")
}

/// Gets a u16 from a [Value].
///
/// Will return `0` if no value is present.
fn get_u16_from_value(value: &Value) -> u16 {
    serde_json::from_value::<String>(value.clone())
        .unwrap_or("0".to_string())
        .parse::<u16>()
        .unwrap_or(0)
}
