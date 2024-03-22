
use chrono::NaiveDateTime;
use diesel::data_types::PgTimestamp;
use diesel::dsl::now;
use diesel::sql_types::Timestamp;
use diesel::{
    Insertable, QueryDsl, Queryable, RunQueryDsl,
    Selectable, SelectableHelper,
};
use diesel::prelude::*;
use diesel::pg::Pg;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::pg::db_schema;

// Define a struct for inserting new posts into the database
#[derive(Deserialize, Insertable)]
#[diesel(table_name = db_schema::customers)] // Use the 'posts' table
pub struct NewCustomer {
    pub name: String,
    pub created_by: Uuid,
}

// Define a struct representing the database schema for posts
#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = db_schema::customers)] // Use the 'posts' table
#[diesel(check_for_backend(diesel::pg::Pg))] // Check compatibility with PostgreSQL
pub struct Customer {
    pub id: i32,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<NaiveDateTime>,
}

pub async fn create_customer(
    pool: &deadpool_diesel::postgres::Pool,
    new_customer: NewCustomer,
) -> anyhow::Result<Customer> {
    // Get a database connection from the pool and handle any potential errors
    let conn = pool.get().await?;

    // Insert the new post into the 'posts' table, returning the inserted post
    let res = conn
        .interact(|conn| {
            diesel::insert_into(db_schema::customers::table)
                .values(new_customer)
                .returning(Customer::as_returning()) // Return the inserted post
                .get_result(conn)
        })
        .await.map_err(|err| {
            anyhow::anyhow!("{err:#?}")
        })??;

    // Adapt the database representation to the application's domain model
    Ok(res)
}

pub async fn find_customers(
    pool: &deadpool_diesel::postgres::Pool,
) -> anyhow::Result<Vec<Customer>> {
    let conn = pool.get().await?;
    let res = conn
        .interact(|conn| {
            db_schema::customers::table.select(Customer::as_select()).load(conn)
        })
        .await.map_err(|err| {
            anyhow::anyhow!("{err:#?}")
        })??;
    Ok(res)
}

pub async fn remove_customers(
    pool: &deadpool_diesel::postgres::Pool,
) -> anyhow::Result<usize> {
    let conn = pool.get().await?;
    let res = conn
        .interact(|conn| {
            diesel::delete(db_schema::customers::table).execute(conn)
        })
        .await.map_err(|err| {
            anyhow::anyhow!("{err:#?}")
        })??;
    let conn = pool.get().await?;
    let _ = conn
        .interact(|conn| {
            diesel::sql_query("ALTER SEQUENCE customers_id_seq RESTART WITH 1").execute(conn)
        })
        .await.map_err(|err| {
            anyhow::anyhow!("{err:#?}")
        })??;
    
    Ok(res)
}

pub async fn update_customer(
    pool: &deadpool_diesel::postgres::Pool,
    user_id: Uuid,
    customer_id: i32,
    customer_name: String,
) -> anyhow::Result<usize> {
    let conn = pool.get().await?;
    let res = conn
        .interact(move |conn| {
            diesel::update(db_schema::customers::table)
                .filter(db_schema::customers::dsl::id.eq(customer_id))
                .set((
                    db_schema::customers::dsl::name.eq(customer_name),
                    db_schema::customers::dsl::updated_by.eq(user_id),
                    db_schema::customers::dsl::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await.map_err(|err| {
            anyhow::anyhow!("{err:#?}")
        })??;
    Ok(res)
}


#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn create_customer_test() -> anyhow::Result<()> {
        let cfg = qm_pg::DbConfig::new()?;
        qm_pg::setup::run(&cfg).await?;
        let db = qm_pg::DB::new("qm-tokio-test", &cfg).await?;
        crate::pg::run_migrations(&db.pool()).await?;
        let customers = super::remove_customers(&db.pool()).await?;

        println!("removed {customers} customers");
        let user_id = uuid::Uuid::new_v4();
        for idx in 1..=16 {
            let result = super::create_customer(db.pool(), super::NewCustomer {
                name: format!("cust{idx:0>2}"),
                created_by: user_id,
            }).await?;
            println!("{result:#?}");
        }
        let customers = super::find_customers(db.pool()).await?;
        println!("{customers:#?}");

        super::update_customer(&db.pool(), user_id, customers.first().unwrap().id, "newname".to_string()).await?;
        let customers = super::find_customers(db.pool()).await?;
        println!("{customers:#?}");


        Ok(())
    }
}