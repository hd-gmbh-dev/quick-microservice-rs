use crate::model::*;
use qm_entity::ids::InfraId;
use sqlx::types::Uuid;
use sqlx::PgPool;
use std::sync::Arc;

pub const DEFAULT_TYPE: &str = "none";

const NAME_MAX_LEN: usize = 1024;
const TY_MAX_LEN: usize = 16;
const INPUT_SLICE_MAX_SIZE: usize = 1024 * 1024 * 1024;

fn check_max_size(name: &str, v: Option<&str>, max_len: usize) -> anyhow::Result<()> {
    if let Some(v) = v {
        if v.len() > max_len {
            anyhow::bail!("The value of '{name}' name is bigger than {max_len} characters");
        }
    }
    Ok(())
}

fn check_max_size_input_slice<T>(name: &str, v: &[T]) -> anyhow::Result<()> {
    let mem_size = std::mem::size_of_val(v);
    if mem_size > INPUT_SLICE_MAX_SIZE {
        anyhow::bail!(
            "The input length of '{name}' is bigger than {} bytes",
            INPUT_SLICE_MAX_SIZE
        );
    }
    Ok(())
}

pub async fn create_customer(
    pool: &PgPool,
    id: Option<i64>,
    name: &str,
    ty: Option<&str>,
    created_by: &Uuid,
) -> anyhow::Result<QmCustomer> {
    check_max_size("Customer name", Some(name), NAME_MAX_LEN)?;
    check_max_size("Customer ty", ty, TY_MAX_LEN)?;
    if let Some(id) = id {
        let rec = sqlx::query!(
            r#"
INSERT INTO customers ( id, name, ty, created_by )
VALUES ( $1, $2, $3, $4 )
RETURNING
    id,
    name,
    ty,
    created_by,
    created_at,
    updated_by,
    updated_at
"#,
            id,
            name,
            ty.unwrap_or(DEFAULT_TYPE),
            created_by
        )
        .fetch_one(pool)
        .await?;
        Ok(QmCustomer {
            id: rec.id.into(),
            name: Arc::from(rec.name),
            ty: Arc::from(rec.ty),
            created_by: rec.created_by,
            created_at: rec.created_at,
            updated_by: rec.updated_by,
            updated_at: rec.updated_at,
        })
    } else {
        let rec = sqlx::query!(
            r#"
INSERT INTO customers ( name, ty, created_by )
VALUES ( $1, $2, $3 )
RETURNING
    id,
    name,
    ty,
    created_by,
    created_at,
    updated_by,
    updated_at
"#,
            name,
            ty.unwrap_or(DEFAULT_TYPE),
            created_by
        )
        .fetch_one(pool)
        .await?;

        Ok(QmCustomer {
            id: rec.id.into(),
            name: Arc::from(rec.name),
            ty: Arc::from(rec.ty),
            created_by: rec.created_by,
            created_at: rec.created_at,
            updated_by: rec.updated_by,
            updated_at: rec.updated_at,
        })
    }
}

pub async fn update_customer(
    pool: &PgPool,
    id: InfraId,
    name: &str,
    updated_by: &Uuid,
) -> anyhow::Result<QmCustomer> {
    check_max_size("Customer name", Some(name), NAME_MAX_LEN)?;
    let rec = sqlx::query!(
        r#"
UPDATE customers AS v
SET name = $2, updated_by = $3, updated_at = NOW()
WHERE v.id = $1
RETURNING
    v.id as id,
    v.name as name,
    v.ty as ty,
    v.created_by as created_by,
    v.created_at as created_at,
    v.updated_by as updated_by,
    v.updated_at as updated_at
"#,
        id.as_ref(),
        name,
        updated_by
    )
    .fetch_one(pool)
    .await?;

    Ok(QmCustomer {
        id: rec.id.into(),
        name: Arc::from(rec.name),
        ty: Arc::from(rec.ty),
        created_by: rec.created_by,
        created_at: rec.created_at,
        updated_by: rec.updated_by,
        updated_at: rec.updated_at,
    })
}

pub async fn remove_customer(pool: &PgPool, id: InfraId) -> anyhow::Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM customers WHERE id = $1", id.as_ref())
            .execute(pool)
            .await?
            .rows_affected() as u64,
    )
}

pub async fn remove_customers(pool: &PgPool, ids: &[i64]) -> anyhow::Result<u64> {
    check_max_size_input_slice("Customer ids", ids)?;
    let result = sqlx::query!(
        "DELETE FROM customers WHERE id IN (SELECT UNNEST($1::int8[]))",
        &ids[..] as &[i64]
    )
    .execute(pool)
    .await?
    .rows_affected() as u64;
    Ok(result)
}

pub async fn create_organization(
    pool: &PgPool,
    id: Option<i64>,
    name: &str,
    ty: Option<&str>,
    customer_id: InfraId,
    created_by: &Uuid,
) -> anyhow::Result<QmOrganization> {
    check_max_size("Organization name", Some(name), NAME_MAX_LEN)?;
    check_max_size("Organization ty", ty, TY_MAX_LEN)?;
    if let Some(id) = id {
        let rec = sqlx::query!(
            r#"
    INSERT INTO organizations ( id, name, ty, customer_id, created_by )
    VALUES ( $1, $2, $3, $4, $5 )
    RETURNING
        id,
        customer_id,
        name,
        ty,
        created_by,
        created_at,
        updated_by,
        updated_at
    "#,
            id,
            name,
            ty.unwrap_or(DEFAULT_TYPE),
            customer_id.as_ref(),
            created_by
        )
        .fetch_one(pool)
        .await?;

        Ok(QmOrganization {
            id: rec.id.into(),
            customer_id: rec.customer_id.into(),
            name: Arc::from(rec.name),
            ty: Arc::from(rec.ty),
            created_by: rec.created_by,
            created_at: rec.created_at,
            updated_by: rec.updated_by,
            updated_at: rec.updated_at,
        })
    } else {
        let rec = sqlx::query!(
            r#"
    INSERT INTO organizations ( name, ty, customer_id, created_by )
    VALUES ( $1, $2, $3, $4 )
    RETURNING
        id,
        customer_id,
        name,
        ty,
        created_by,
        created_at,
        updated_by,
        updated_at
    "#,
            name,
            ty.unwrap_or(DEFAULT_TYPE),
            customer_id.as_ref(),
            created_by
        )
        .fetch_one(pool)
        .await?;

        Ok(QmOrganization {
            id: rec.id.into(),
            customer_id: rec.customer_id.into(),
            name: Arc::from(rec.name),
            ty: Arc::from(rec.ty),
            created_by: rec.created_by,
            created_at: rec.created_at,
            updated_by: rec.updated_by,
            updated_at: rec.updated_at,
        })
    }
}

pub async fn update_organization(
    pool: &PgPool,
    id: InfraId,
    name: &str,
    updated_by: &Uuid,
) -> anyhow::Result<QmOrganization> {
    let rec = sqlx::query!(
        r#"
UPDATE organizations AS v
SET name = $2, updated_by = $3, updated_at = NOW()
WHERE v.id = $1
RETURNING
    v.id as id,
    v.customer_id as customer_id,
    v.name as name,
    v.ty as ty,
    v.created_by as created_by,
    v.created_at as created_at,
    v.updated_by as updated_by,
    v.updated_at as updated_at
"#,
        id.as_ref(),
        name,
        updated_by,
    )
    .fetch_one(pool)
    .await?;

    Ok(QmOrganization {
        id: rec.id.into(),
        customer_id: rec.customer_id.into(),
        name: Arc::from(rec.name),
        ty: Arc::from(rec.ty),
        created_by: rec.created_by,
        created_at: rec.created_at,
        updated_by: rec.updated_by,
        updated_at: rec.updated_at,
    })
}

pub async fn remove_organization(pool: &PgPool, id: InfraId) -> anyhow::Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM organizations WHERE id = $1", id.as_ref())
            .execute(pool)
            .await?
            .rows_affected() as u64,
    )
}

pub async fn remove_organizations(pool: &PgPool, ids: &[i64]) -> anyhow::Result<u64> {
    check_max_size_input_slice("Organization ids", ids)?;
    let result = sqlx::query!(
        "DELETE FROM organizations WHERE id IN (SELECT UNNEST($1::int8[]))",
        &ids[..] as &[i64]
    )
    .execute(pool)
    .await?
    .rows_affected() as u64;
    Ok(result)
}

pub async fn create_institution(
    pool: &PgPool,
    id: Option<i64>,
    name: &str,
    ty: Option<&str>,
    customer_id: InfraId,
    organization_id: InfraId,
    created_by: &Uuid,
) -> anyhow::Result<QmInstitution> {
    check_max_size("Institution name", Some(name), NAME_MAX_LEN)?;
    check_max_size("Institution ty", ty, TY_MAX_LEN)?;
    if let Some(id) = id {
        let rec = sqlx::query!(
            r#"
INSERT INTO institutions ( id, name, ty, customer_id, organization_id, created_by )
VALUES ( $1, $2, $3, $4, $5, $6 )
RETURNING
    id,
    customer_id,
    organization_id,
    name,
    ty,
    created_by,
    created_at,
    updated_by,
    updated_at
"#,
            id,
            name,
            ty.unwrap_or(DEFAULT_TYPE),
            customer_id.as_ref(),
            organization_id.as_ref(),
            created_by
        )
        .fetch_one(pool)
        .await?;

        Ok(QmInstitution {
            id: rec.id.into(),
            customer_id: rec.customer_id.into(),
            organization_id: rec.organization_id.into(),
            name: Arc::from(rec.name),
            ty: Arc::from(rec.ty),
            created_by: rec.created_by,
            created_at: rec.created_at,
            updated_by: rec.updated_by,
            updated_at: rec.updated_at,
        })
    } else {
        let rec = sqlx::query!(
            r#"
INSERT INTO institutions ( name, ty, customer_id, organization_id, created_by )
VALUES ( $1, $2, $3, $4, $5 )
RETURNING
    id,
    customer_id,
    organization_id,
    name,
    ty,
    created_by,
    created_at,
    updated_by,
    updated_at
"#,
            name,
            ty.unwrap_or(DEFAULT_TYPE),
            customer_id.as_ref(),
            organization_id.as_ref(),
            created_by
        )
        .fetch_one(pool)
        .await?;

        Ok(QmInstitution {
            id: rec.id.into(),
            customer_id: rec.customer_id.into(),
            organization_id: rec.organization_id.into(),
            name: Arc::from(rec.name),
            ty: Arc::from(rec.ty),
            created_by: rec.created_by,
            created_at: rec.created_at,
            updated_by: rec.updated_by,
            updated_at: rec.updated_at,
        })
    }
}

pub async fn update_institution(
    pool: &PgPool,
    id: InfraId,
    name: &str,
    updated_by: &Uuid,
) -> anyhow::Result<QmInstitution> {
    check_max_size("Institution name", Some(name), NAME_MAX_LEN)?;
    let rec = sqlx::query!(
        r#"
UPDATE institutions AS v
SET name = $2, updated_by = $3, updated_at = NOW()
WHERE v.id = $1
RETURNING
    v.id as id,
    v.customer_id as customer_id,
    v.organization_id as organization_id,
    v.name as name,
    v.ty as ty,
    v.created_by as created_by,
    v.created_at as created_at,
    v.updated_by as updated_by,
    v.updated_at as updated_at
"#,
        id.as_ref(),
        name,
        updated_by,
    )
    .fetch_one(pool)
    .await?;

    Ok(QmInstitution {
        id: rec.id.into(),
        customer_id: rec.customer_id.into(),
        organization_id: rec.organization_id.into(),
        name: Arc::from(rec.name),
        ty: Arc::from(rec.ty),
        created_by: rec.created_by,
        created_at: rec.created_at,
        updated_by: rec.updated_by,
        updated_at: rec.updated_at,
    })
}

pub async fn remove_institution(pool: &PgPool, id: InfraId) -> anyhow::Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM institutions WHERE id = $1", id.as_ref())
            .execute(pool)
            .await?
            .rows_affected() as u64,
    )
}

pub async fn remove_institutions(pool: &PgPool, ids: &[i64]) -> anyhow::Result<u64> {
    check_max_size_input_slice("Institution ids", ids)?;
    let result = sqlx::query!(
        "DELETE FROM institutions WHERE id IN (SELECT UNNEST($1::int8[]))",
        &ids[..] as &[i64]
    )
    .execute(pool)
    .await?
    .rows_affected() as u64;
    Ok(result)
}
