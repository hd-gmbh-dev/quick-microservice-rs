use crate::model::*;
use qm_pg::DB;
use sqlx::query_as;

pub async fn fetch_users(db: &DB, realm: &str) -> anyhow::Result<Vec<KcUserQuery>> {
    Ok(query_as!(
        KcUserQuery,
        r#"
SELECT
    u.id AS id,
    u.first_name AS firstname,
    u.last_name AS lastname,
    u.username AS username,
    u.email AS email,
    u.enabled AS enabled
FROM realm re
    JOIN user_entity u on re.id = u.realm_id
WHERE re.name = $1;"#,
        realm
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_user_groups(db: &DB, realm: &str) -> anyhow::Result<Vec<KcUserGroupQuery>> {
    Ok(query_as!(
        KcUserGroupQuery,
        r#"
SELECT
    gm.user_id as user_id,
    gm.group_id as group_id
FROM realm re
        JOIN keycloak_group g ON re.id = g.realm_id
        JOIN user_group_membership gm ON g.id = gm.group_id
WHERE re.name = $1;"#,
        realm
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_roles(db: &DB, realm: &str) -> anyhow::Result<Vec<KcRoleQuery>> {
    Ok(query_as!(
        KcRoleQuery,
        r#"
SELECT
    r.id AS role_id,
    r.name AS role_name
FROM realm re
    JOIN public.keycloak_role r ON r.realm_id = re.id
    WHERE re.name = $1;"#,
        realm
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_user_roles(db: &DB, user_id: &str) -> anyhow::Result<Vec<KcUserRoleQuery>> {
    Ok(query_as!(
        KcUserRoleQuery,
        r#"
SELECT
    rm.user_id AS user_id,
    rm.role_id AS role_id
FROM realm re
        JOIN public.keycloak_role r ON r.realm_id = re.id
        JOIN public.user_role_mapping rm ON rm.role_id = r.id
WHERE re.name = $1;"#,
        user_id
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_groups(db: &DB, realm: &str) -> anyhow::Result<Vec<KcGroupQuery>> {
    Ok(query_as!(
        KcGroupQuery,
        r#"
SELECT
    g.id AS id,
    g.parent_group AS parent_group,
    g.name AS name
FROM realm re
    JOIN public.keycloak_group g ON g.realm_id = re.id
WHERE re.name = $1;
    "#,
        realm
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_group_attributes(
    db: &DB,
    realm: &str,
) -> anyhow::Result<Vec<KcGroupDetailsQuery>> {
    Ok(query_as!(
        KcGroupDetailsQuery,
        r#"
SELECT
    g.id as group_id,
    a.value as context,
    b.value as allowed_access_levels,
    c.value as display_name,
    d.value as built_in
FROM realm re
    JOIN public.keycloak_group g ON g.realm_id = re.id
    LEFT JOIN group_attribute a ON a.group_id = g.id AND a.name = 'context'
    LEFT JOIN group_attribute b ON b.group_id = g.id AND b.name = 'allowed_access_levels'
    LEFT JOIN group_attribute c ON c.group_id = g.id AND c.name = 'display_name'
    LEFT JOIN group_attribute d ON d.group_id = g.id AND d.name = 'built_in'
WHERE re.name = $1;
    "#,
        realm
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_realm_info(db: &DB, name: &str) -> anyhow::Result<Option<KcRealmQuery>> {
    Ok(query_as!(
        KcRealmQuery,
        r#"
        SELECT
        r0.id as id
    FROM realm r0
    WHERE r0.name = $1;"#,
        name
    )
    .fetch_optional(db.pool())
    .await?)
}

pub async fn fetch_customers(db: &DB) -> anyhow::Result<Vec<Customer>> {
    Ok(query_as!(
        Customer,
        r#"
SELECT
    id,
    name,
    ty,
    created_by,
    created_at,
    updated_by,
    updated_at
FROM customers;"#
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_organizations(db: &DB) -> anyhow::Result<Vec<Organization>> {
    Ok(query_as!(
        Organization,
        r#"
SELECT
    id,
    name,
    ty,
    customer_id,
    created_by,
    created_at,
    updated_by,
    updated_at
FROM organizations;"#
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_institutions(db: &DB) -> anyhow::Result<Vec<Institution>> {
    Ok(query_as!(
        Institution,
        r#"
SELECT
    id,
    name,
    ty,
    customer_id,
    organization_id,
    created_by,
    created_at,
    updated_by,
    updated_at
FROM institutions;"#
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_organization_units(db: &DB) -> anyhow::Result<Vec<OrganizationUnit>> {
    Ok(query_as!(
        OrganizationUnitQuery,
        r#"
SELECT
    v.id as id,
    v.name as name,
    ty,
    v.customer_id as customer_id,
    v.organization_id as organization_id,
    v.created_by as created_by,
    v.created_at as created_at,
    v.updated_by as updated_by,
    v.updated_at as updated_at
FROM organization_units v"#
    )
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(Into::into)
    .collect())
}
