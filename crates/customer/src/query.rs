use crate::model::*;
use qm_pg::DB;
use sqlx::query_as;

pub async fn fetch_users(db: &DB, realm: &str) -> anyhow::Result<Vec<KcUserQuery>> {
    Ok(query_as!(
        KcUserQuery,
        r#"
SELECT
    u.id AS user_id,
    gm.group_id AS group_id,
    rm.role_id AS role_id,
    u.first_name AS firstname,
    u.last_name AS lastname,
    u.username AS username,
    u.email AS email,
    u.enabled AS enabled
FROM realm re
    JOIN user_entity u on re.id = u.realm_id
    JOIN public.user_group_membership gm ON gm.user_id = u.id
    JOIN public.user_role_mapping rm ON rm.user_id = u.id
    JOIN public.keycloak_group g ON g.id = gm.group_id
    JOIN public.keycloak_role r ON r.id = rm.role_id
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

pub async fn fetch_user_roles(db: &DB, user_id: &str) -> anyhow::Result<Vec<KcRoleQuery>> {
    Ok(query_as!(
        KcRoleQuery,
        r#"
SELECT
    r0.role_id as role_id,
    r1.name as role_name
FROM user_role_mapping r0
JOIN keycloak_role r1 on r1.id = r0.role_id
    WHERE user_id = $1;"#,
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
    g.id AS group_id,
    g.name AS group_name,
    a.value AS context,
    b.value AS built_in
FROM realm re
    JOIN public.keycloak_group g ON g.realm_id = re.id
    LEFT JOIN public.group_attribute a ON a.group_id = g.id AND a.name = 'context'
    LEFT JOIN public.group_attribute b ON b.group_id = g.id AND b.name = 'built_in'
WHERE re.name = $1;
    "#,
        realm
    )
    .fetch_all(db.pool())
    .await?)
}

pub async fn fetch_user_groups(db: &DB, user_id: &str) -> anyhow::Result<Vec<KcGroupQuery>> {
    Ok(query_as!(
        KcGroupQuery,
        r#"
SELECT
    r0.group_id as group_id,
    r1.name as group_name,
    a.value AS context,
    b.value AS built_in
FROM user_group_membership r0
JOIN keycloak_group r1 on r1.id = r0.group_id
LEFT JOIN public.group_attribute a ON a.group_id = r1.id AND a.name = 'context'
LEFT JOIN public.group_attribute b ON b.group_id = r1.id AND b.name = 'built_in'
    WHERE user_id = $1;"#,
        user_id
    )
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(|v| KcGroupQuery {
        group_name: v.group_name.map(|n| format!("/{n}")),
        group_id: v.group_id,
        built_in: v.built_in,
        context: v.context,
    })
    .collect())
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
