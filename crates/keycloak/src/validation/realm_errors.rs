/// Error IDs and keys for realm validation.
pub const REALM_PREFIX: &str = "realm-";
/// Prefix for client errors.
pub const CLIENTS_CLIENT_PREFIX: &str = "clients-client-";
/// Authentication flow 2FA email prefix.
pub const REALM_AUTHENTICATION_FLOW_2FAEMAIL_PREFIX: &str = "authentication_flow_2faemail-";
/// Browser flow prefix.
pub const REALM_BROWSER_FLOW_PREFIX: &str = "browser_flow";
/// Default locale invalid ID.
pub const REALM_DEFAULT_LOCALE_INVALID_ID: &str = "realm-default_locale-invalid";
/// Default locale missing ID.
pub const REALM_DEFAULT_LOCALE_MISSING_ID: &str = "realm-default_locale-missing";
/// Internationalization enabled ID.
pub const REALM_INTERNATIONALIZATION_ENABLED_ID: &str = "realm-internationalization_enabled";
/// Login theme invalid ID.
pub const REALM_LOGIN_THEME_INVALID_ID: &str = "realm-login_theme-invalid";
/// Login theme missing ID.
pub const REALM_LOGIN_THEME_MISSING_ID: &str = "realm-login_theme-missing";
/// Browser flow invalid ID.
pub const REALM_BROWSER_FLOW_INVALID_ID: &str = "browser_flow-invalid";
/// Browser flow missing ID.
pub const REALM_BROWSER_FLOW_MISSING_ID: &str = "browser_flow-missing";
/// Authentication flow 2FA email missing ID.
pub const REALM_AUTHENTICATION_FLOW_2FAEMAIL_MISSING_ID: &str =
    "authentication_flow_2faemail-missing";
/// Email theme invalid ID.
pub const REALM_EMAIL_THEME_INVALID_ID: &str = "realm-email_theme-invalid";
/// Email theme missing ID.
pub const REALM_EMAIL_THEME_MISSING_ID: &str = "realm-email_theme-missing";
/// Password policy length ID.
pub const REALM_PASSWORD_POLICY_LENGTH_ID: &str = "realm-password_policy-length";
/// Password policy symbol ID.
pub const REALM_PASSWORD_POLICY_SYMBOL_ID: &str = "realm-password_policy-symbol";
/// Password policy uppercase ID.
pub const REALM_PASSWORD_POLICY_UPPERCASE_ID: &str = "realm-password_policy-uppercase";
/// Password policy lowercase ID.
pub const REALM_PASSWORD_POLICY_LOWERCASE_ID: &str = "realm-password_policy-lowercase";
/// Password policy digit ID.
pub const REALM_PASSWORD_POLICY_DIGIT_ID: &str = "realm-password_policy-digit";
/// Password policy missing ID.
pub const REALM_PASSWORD_POLICY_MISSING_ID: &str = "realm-password_policy-missing";
/// Remember me ID.
pub const REALM_REMEMBER_ME_ID: &str = "realm-remember_me";
/// Registration allowed ID.
pub const REALM_REGISTRATION_ALLOWED_ID: &str = "realm-registration_allowed";
/// Reset password allowed ID.
pub const REALM_RESET_PASSWORD_ALLOWED_ID: &str = "realm-reset_password_allowed";
/// Supported locales invalid ID.
pub const REALM_SUPPORTED_LOCALES_INVALID_ID: &str = "realm-supported_locales-invalid";
/// Supported locales missing ID.
pub const REALM_SUPPORTED_LOCALES_MISSING_ID: &str = "realm-supported_locales-missing";
/// SMTP server missing ID.
pub const REALM_SMTP_SERVER_MISSING_ID: &str = "realm-smtp_server-missing";
/// SMTP server reply to display name missing ID.
pub const REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISSING_ID: &str =
    "realm-smtp_server-reply_to_display_name-missing";
/// SMTP server reply to display name mismatched ID.
pub const REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISMATCHED_ID: &str =
    "realm-smtp_server-reply_to_display_name-mismatched";
/// SMTP server starttls missing ID.
pub const REALM_SMTP_SERVER_STARTTLS_MISSING_ID: &str = "realm-smtp_server-starttls-missing";
/// SMTP server starttls mismatched ID.
pub const REALM_SMTP_SERVER_STARTTLS_MISMATCHED_ID: &str = "realm-smtp_server-starttls-mismatched";
/// SMTP server starttls invalid ID.
pub const REALM_SMTP_SERVER_STARTTLS_INVALID_ID: &str = "realm-smtp_server-starttls-invalid";
/// SMTP server port missing ID.
pub const REALM_SMTP_SERVER_PORT_MISSING_ID: &str = "realm-smtp_server-port-missing";
/// SMTP server port mismatched ID.
pub const REALM_SMTP_SERVER_PORT_MISMATCHED_ID: &str = "realm-smtp_server-port-mismatched";
/// SMTP server port invalid ID.
pub const REALM_SMTP_SERVER_PORT_INVALID_ID: &str = "realm-smtp_server-port-invalid";
/// SMTP server host missing ID.
pub const REALM_SMTP_SERVER_HOST_MISSING_ID: &str = "realm-smtp_server-host-missing";
/// SMTP server host mismatched ID.
pub const REALM_SMTP_SERVER_HOST_MISMATCHED_ID: &str = "realm-smtp_server-host-mismatched";
/// SMTP server host invalid ID.
pub const REALM_SMTP_SERVER_HOST_INVALID_ID: &str = "realm-smtp_server-host-invalid";
/// SMTP server reply to missing ID.
pub const REALM_SMTP_SERVER_REPLY_TO_MISSING_ID: &str = "realm-smtp_server-reply_to-missing";
/// SMTP server reply to mismatched ID.
pub const REALM_SMTP_SERVER_REPLY_TO_MISMATCHED_ID: &str = "realm-smtp_server-reply_to-mismatched";
/// SMTP server from missing ID.
pub const REALM_SMTP_SERVER_FROM_MISSING_ID: &str = "realm-smtp_server-from-missing";
/// SMTP server from mismatched ID.
pub const REALM_SMTP_SERVER_FROM_MISMATCHED_ID: &str = "realm-smtp_server-from-mismatched";
/// SMTP server from invalid ID.
pub const REALM_SMTP_SERVER_FROM_INVALID_ID: &str = "realm-smtp_server-from-invalid";
/// SMTP server from display name missing ID.
pub const REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISSING_ID: &str =
    "realm-smtp_server-from_display_name-missing";
/// SMTP server from display name mismatched ID.
pub const REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISMATCHED_ID: &str =
    "realm-smtp_server-from_display_name-mismatched";
/// SMTP server SSL missing ID.
pub const REALM_SMTP_SERVER_SSL_MISSING_ID: &str = "realm-smtp_server-ssl-missing";
/// SMTP server SSL mismatched ID.
pub const REALM_SMTP_SERVER_SSL_MISMATCHED_ID: &str = "realm-smtp_server-ssl-mismatched";
/// SMTP server SSL invalid ID.
pub const REALM_SMTP_SERVER_SSL_INVALID_ID: &str = "realm-smtp_server-ssl-invalid";
/// Duplicate emails allowed mismatched ID.
pub const REALM_DUPLICATE_EMAILS_ALLOWED_MISMATCHED_ID: &str =
    "realm-duplicate_emails_allowed-mismatched";
/// Edit username allowed mismatched ID.
pub const REALM_EDIT_USERNAME_ALLOWED_MISMATCHED_ID: &str =
    "realm-edit_username_allowed-mismatched";
/// Client attributes OAuth2 device authorization grant enabled invalid ID.
pub const CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_INVALID_ID: &str =
    "clients-client-attributes-oauth2_device_authorization_grant_enabled-invalid";
/// Client attributes OAuth2 device authorization grant enabled missing ID.
pub const CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_MISSING_ID: &str =
    "clients-client-attributes-oauth2_device_authorization_grant_enabled-missing";
/// Client attributes backchannel logout disabled ID.
pub const CLIENTS_CLIENT_ATTRIBUTES_BACKCHANNEL_LOGOUT_DISABLED_ID: &str =
    "clients-client-attributes-backchannel_logout_disabled";
/// Client attributes missing ID.
pub const CLIENTS_CLIENT_ATTRIBUTES_MISSING_ID: &str = "clients-client-attributes-missing";
/// Client base URL invalid ID.
pub const CLIENTS_CLIENT_BASE_URL_INVALID_ID: &str = "clients-client-base_url-invalid";
/// Client base URL missing ID.
pub const CLIENTS_CLIENT_BASE_URL_MISSING_ID: &str = "clients-client-base_url-missing";
/// Client client ID ID.
pub const CLIENTS_CLIENT_CLIENT_ID_ID: &str = "clients-client-client_id";
/// Client consent required ID.
pub const CLIENTS_CLIENT_CONSENT_REQUIRED_ID: &str = "clients-client-consent_required";
/// Client direct access grant enabled ID.
pub const CLIENTS_CLIENT_DIRECT_ACCESS_GRANT_ENABLED_ID: &str =
    "clients-client-direct_access_grants_enabled";
/// Client enabled ID.
pub const CLIENTS_CLIENT_ENABLED_ID: &str = "clients-client-enabled";
/// Client implicit flow enabled ID.
pub const CLIENTS_CLIENT_IMPLICIT_FLOW_ENABLED_ID: &str = "clients-client-implicit_flow_enabled";
/// Client public ID.
pub const CLIENTS_CLIENT_PUBLIC_CLIENT_ID: &str = "clients-client-public_client";
/// Client redirect URIs invalid ID.
pub const CLIENTS_CLIENT_REDIRECT_URIS_INVALID_ID: &str = "clients-client-redirect_uris-invalid";
/// Client redirect URIs missing ID.
pub const CLIENTS_CLIENT_REDIRECT_URIS_MISSING_ID: &str = "clients-client-redirect_uris-missing";
/// Client root URL invalid ID.
pub const CLIENTS_CLIENT_ROOT_URL_INVALID_ID: &str = "clients-client-root_url-invalid";
/// Client root URL missing ID.
pub const CLIENTS_CLIENT_ROOT_URL_MISSING_ID: &str = "clients-client-root_url-missing";
/// Client service accounts enabled ID.
pub const CLIENTS_CLIENT_SERVICE_ACCOUNTS_ENABLED_ID: &str =
    "clients-client-service_accounts_enabled";
/// Client standard flow enabled ID.
pub const CLIENTS_CLIENT_STANDARD_FLOW_ENABLED_ID: &str = "clients-client-standard_flow_enabled";
/// Client missing ID.
pub const CLIENTS_CLIENT_MISSING_ID: &str = "clients-client-missing";
/// Client frontchannel logout enabled ID.
pub const CLIENTS_CLIENT_FRONTCHANNEL_LOGOUT_ENABLED_ID: &str =
    "clients-client-frontchannel_logout_enabled";
/// Groups customer ID.
pub const GROUPS_CUSTOMER_ID: &str = "groups-customer";
/// Groups owner ID.
pub const GROUPS_OWNER_ID: &str = "groups-owner";
/// Roles customer ID.
pub const ROLES_CUSTOMER_ID: &str = "roles-customer_id";
/// Roles users read ID.
pub const ROLES_USERS_READ_ID: &str = "roles-users_read";
/// Roles users write ID.
pub const ROLES_USERS_WRITE_ID: &str = "roles-users_write";
/// Roles child create ID.
pub const ROLES_CHILD_CREATE_ID: &str = "roles-child_create";
/// Roles child write ID.
pub const ROLES_CHILD_WRITE_ID: &str = "roles-child_write";
/// Roles child read ID.
pub const ROLES_CHILD_READ_ID: &str = "roles-child_read";
/// Roles customer key.
pub const ROLES_CUSTOMER_KEY: &str = "roles.customer";
/// Roles users read key.
pub const ROLES_USERS_READ_KEY: &str = "roles.users_read";
/// Roles users write key.
pub const ROLES_USERS_WRITE_KEY: &str = "roles.users_write";
/// Roles child create key.
pub const ROLES_CHILD_CREATE_KEY: &str = "roles.child_create";
/// Roles child write key.
pub const ROLES_CHILD_WRITE_KEY: &str = "roles.child_write";
/// Roles child read key.
pub const ROLES_CHILD_READ_KEY: &str = "roles.child_read";
/// Group roles mapping owner invalid ID.
pub const GROUP_ROLES_MAPPING_OWNER_INVALID_ID: &str = "group-roles-mapping-owner-invalid";
/// Realm internationalization enabled key.
pub const REALM_INTERNATIONALIZATION_ENABLED_KEY: &str = "realm.internationalization_enabled";
/// Realm default locale invalid key.
pub const REALM_DEFAULT_LOCALE_INVALID_KEY: &str = "realm.default_locale.invalid";
/// Realm default locale missing key.
pub const REALM_DEFAULT_LOCALE_MISSING_KEY: &str = "realm.default_locale.missing";
/// Realm login theme invalid key.
pub const REALM_LOGIN_THEME_INVALID_KEY: &str = "realm.login_theme.invalid";
/// Realm login theme missing key.
pub const REALM_LOGIN_THEME_MISSING_KEY: &str = "realm.login_theme.missing";
/// Realm browser flow missing key.
pub const REALM_BROWSER_FLOW_MISSING_KEY: &str = "realm.browser_flow.missing";
/// Realm browser flow invalid key.
pub const REALM_BROWSER_FLOW_INVALID_KEY: &str = "realm.browser_flow.invalid";
/// Realm authentication flow 2FA email missing key.
pub const REALM_AUTHENTICATION_FLOW_2FAEMAIL_MISSING_KEY: &str =
    "realm.authentication_flow_2faemail.missing";
/// Realm email theme invalid key.
pub const REALM_EMAIL_THEME_INVALID_KEY: &str = "realm.email_theme.invalid";
/// Realm email theme missing key.
pub const REALM_EMAIL_THEME_MISSING_KEY: &str = "realm.email_theme.missing";
/// Realm password policy length key.
pub const REALM_PASSWORD_POLICY_LENGTH_KEY: &str = "realm.password_policy.length";
/// Realm password policy symbol key.
pub const REALM_PASSWORD_POLICY_SYMBOL_KEY: &str = "realm.password_policy.symbol";
/// Realm password policy uppercase key.
pub const REALM_PASSWORD_POLICY_UPPERCASE_KEY: &str = "realm.password_policy.uppercase";
/// Realm password policy lowercase key.
pub const REALM_PASSWORD_POLICY_LOWERCASE_KEY: &str = "realm.password_policy.lowercase";
/// Realm password policy digit key.
pub const REALM_PASSWORD_POLICY_DIGIT_KEY: &str = "realm.password_policy.digit";
/// Realm password policy missing key.
pub const REALM_PASSWORD_POLICY_MISSING_KEY: &str = "realm.password_policy.missing";
/// Realm remember me key.
pub const REALM_REMEMBER_ME_KEY: &str = "realm.remember_me";
/// Realm registration allowed key.
pub const REALM_REGISTRATION_ALLOWED_KEY: &str = "realm.registration_allowed";
/// Realm reset password allowed key.
pub const REALM_RESET_PASSWORD_ALLOWED_KEY: &str = "realm.reset_password_allowed";
/// Realm supported locales invalid key.
pub const REALM_SUPPORTED_LOCALES_INVALID_KEY: &str = "realm.supported_locales.invalid";
/// Realm supported locales missing key.
pub const REALM_SUPPORTED_LOCALES_MISSING_KEY: &str = "realm.supported_locales.missing";
/// Realm SMTP server missing key.
pub const REALM_SMTP_SERVER_MISSING_KEY: &str = "realm.smtp_server.missing";
/// Realm SMTP server reply to display name missing key.
pub const REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISSING_KEY: &str =
    "realm.smtp_server.reply_to_display_name.missing";
/// Realm SMTP server reply to display name mismatched key.
pub const REALM_SMTP_SERVER_REPLY_TO_DISPLAY_NAME_MISMATCHED_KEY: &str =
    "realm.smtp_server.reply_to_display_name.mismatched";
/// Realm SMTP server starttls missing key.
pub const REALM_SMTP_SERVER_STARTTLS_MISSING_KEY: &str = "realm.smtp_server.starttls.missing";
/// Realm SMTP server starttls mismatched key.
pub const REALM_SMTP_SERVER_STARTTLS_MISMATCHED_KEY: &str = "realm.smtp_server.starttls.mismatched";
/// Realm SMTP server starttls invalid key.
pub const REALM_SMTP_SERVER_STARTTLS_INVALID_KEY: &str = "realm.smtp_server.starttls.invalid";
/// Realm SMTP server port missing key.
pub const REALM_SMTP_SERVER_PORT_MISSING_KEY: &str = "realm.smtp_server.port.missing";
/// Realm SMTP server port mismatched key.
pub const REALM_SMTP_SERVER_PORT_MISMATCHED_KEY: &str = "realm.smtp_server.port.mismatched";
/// Realm SMTP server port invalid key.
pub const REALM_SMTP_SERVER_PORT_INVALID_KEY: &str = "realm.smtp_server.port.invalid";
/// Realm SMTP server host missing key.
pub const REALM_SMTP_SERVER_HOST_MISSING_KEY: &str = "realm.smtp_server.host.missing";
/// Realm SMTP server host mismatched key.
pub const REALM_SMTP_SERVER_HOST_MISMATCHED_KEY: &str = "realm.smtp_server.host.mismatched";
/// Realm SMTP server host invalid key.
pub const REALM_SMTP_SERVER_HOST_INVALID_KEY: &str = "realm.smtp_server.host.invalid";
/// Realm SMTP server reply to missing key.
pub const REALM_SMTP_SERVER_REPLY_TO_MISSING_KEY: &str = "realm.smtp_server.reply_to.missing";
/// Realm SMTP server reply to mismatched key.
pub const REALM_SMTP_SERVER_REPLY_TO_MISMATCHED_KEY: &str = "realm.smtp_server.reply_to.mismatched";
/// Realm SMTP server from missing key.
pub const REALM_SMTP_SERVER_FROM_MISSING_KEY: &str = "realm.smtp_server.from.missing";
/// Realm SMTP server from mismatched key.
pub const REALM_SMTP_SERVER_FROM_MISMATCHED_KEY: &str = "realm.smtp_server.from.mismatched";
/// Realm SMTP server from invalid key.
pub const REALM_SMTP_SERVER_FROM_INVALID_KEY: &str = "realm.smtp_server.from.invalid";
/// Realm SMTP server from display name missing key.
pub const REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISSING_KEY: &str =
    "realm.smtp_server.from_display_name.missing";
/// Realm SMTP server from display name mismatched key.
pub const REALM_SMTP_SERVER_FROM_DISPLAY_NAME_MISMATCHED_KEY: &str =
    "realm.smtp_server.from_display_name.mismatched";
/// Realm SMTP server SSL missing key.
pub const REALM_SMTP_SERVER_SSL_MISSING_KEY: &str = "realm.smtp_server.ssl.missing";
/// Realm SMTP server SSL mismatched key.
pub const REALM_SMTP_SERVER_SSL_MISMATCHED_KEY: &str = "realm.smtp_server.ssl.mismatched";
/// Realm SMTP server SSL invalid key.
pub const REALM_SMTP_SERVER_SSL_INVALID_KEY: &str = "realm.smtp_server.ssl.invalid";
/// Realm duplicate emails allowed mismatched key.
pub const REALM_DUPLICATE_EMAILS_ALLOWED_MISMATCHED_KEY: &str =
    "realm.duplicate_emails_allowed.mismatched";
/// Realm edit username allowed mismatched key.
pub const REALM_EDIT_USERNAME_ALLOWED_MISMATCHED_KEY: &str =
    "realm.edit_username_allowed.mismatched";
/// Client attributes OAuth2 device authorization grant enabled invalid key.
pub const CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_INVALID_KEY: &str =
    "clients.client.attributes.oauth2_device_authorization_grant_enabled.invalid";
/// Client attributes OAuth2 device authorization grant enabled missing key.
pub const CLIENTS_CLIENT_ATTRIBUTES_OAUTH2_DEVICE_AUTHORIZATION_GRANT_ENABLED_MISSING_KEY: &str =
    "clients.client.attributes.oauth2_device_authorization_grant_enabled.missing";
/// Client attributes missing key.
pub const CLIENTS_CLIENT_ATTRIBUTES_MISSING_KEY: &str = "clients.client.attributes.missing";
/// Client attributes backchannel logout disabled key.
pub const CLIENTS_CLIENT_ATTRIBUTES_BACKCHANNEL_LOGOUT_DISABLED_KEY: &str =
    "clients.client.attributes.backchannel_logout_disabled";
/// Client base URL invalid key.
pub const CLIENTS_CLIENT_BASE_URL_INVALID_KEY: &str = "clients.client.base_url.invalid";
/// Client base URL missing key.
pub const CLIENTS_CLIENT_BASE_URL_MISSING_KEY: &str = "clients.client.base_url.missing";
/// Client client ID key.
pub const CLIENTS_CLIENT_CLIENT_ID_KEY: &str = "clients.client.client_id";
/// Client consent required key.
pub const CLIENTS_CLIENT_CONSENT_REQUIRED_KEY: &str = "clients.client.consent_required";
/// Client direct access grant enabled key.
pub const CLIENTS_CLIENT_DIRECT_ACCESS_GRANT_ENABLED_KEY: &str =
    "clients.client.direct_access_grants_enabled";
/// Client enabled key.
pub const CLIENTS_CLIENT_ENABLED_KEY: &str = "clients.client.enabled";
/// Client implicit flow enabled key.
pub const CLIENTS_CLIENT_IMPLICIT_FLOW_ENABLED_KEY: &str = "clients.client.implicit_flow_enabled";
/// Client public client key.
pub const CLIENTS_CLIENT_PUBLIC_CLIENT_KEY: &str = "clients.client.public_client";
/// Client redirect URIs invalid key.
pub const CLIENTS_CLIENT_REDIRECT_URIS_INVALID_KEY: &str = "clients.client.redirect_uris.invalid";
/// Client redirect URIs missing key.
pub const CLIENTS_CLIENT_REDIRECT_URIS_MISSING_KEY: &str = "clients.client.redirect_uris.missing";
/// Client root URL invalid key.
pub const CLIENTS_CLIENT_ROOT_URL_INVALID_KEY: &str = "clients.client.root_url.invalid";
/// Client root URL missing key.
pub const CLIENTS_CLIENT_ROOT_URL_MISSING_KEY: &str = "clients.client.root_url.missing";
/// Client service accounts enabled key.
pub const CLIENTS_CLIENT_SERVICE_ACCOUNTS_ENABLED_KEY: &str =
    "clients.client.service_accounts_enabled";
/// Client standard flow enabled key.
pub const CLIENTS_CLIENT_STANDARD_FLOW_ENABLED_KEY: &str = "clients.client.standard_flow_enabled";
/// Client missing key.
pub const CLIENTS_CLIENT_MISSING_KEY: &str = "clients.client.missing";
/// Client frontchannel logout enabled key.
pub const CLIENTS_CLIENT_FRONTCHANNEL_LOGOUT_ENABLED_KEY: &str =
    "clients.client.frontchannel_logout_enabled";
