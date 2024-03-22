// @generated automatically by Diesel CLI.

diesel::table! {
    customers (id) {
        id -> Int4,
        #[max_length = 50]
        name -> Varchar,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_by -> Nullable<Uuid>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    institutions (id) {
        id -> Int4,
        customer_id -> Int4,
        organization_id -> Int4,
        #[max_length = 50]
        name -> Varchar,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_by -> Nullable<Uuid>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    organization_unit_members (organization_unit_id, customer_id, organization_id, institution_id) {
        organization_unit_id -> Int4,
        customer_id -> Int4,
        organization_id -> Int4,
        institution_id -> Int4,
    }
}

diesel::table! {
    organization_units (id) {
        id -> Int4,
        customer_id -> Int4,
        organization_id -> Nullable<Int4>,
        #[max_length = 50]
        name -> Varchar,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_by -> Nullable<Uuid>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    organizations (id) {
        id -> Int4,
        customer_id -> Int4,
        #[max_length = 50]
        name -> Varchar,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_by -> Nullable<Uuid>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(institutions -> customers (customer_id));
diesel::joinable!(institutions -> organizations (organization_id));
diesel::joinable!(organization_unit_members -> customers (customer_id));
diesel::joinable!(organization_unit_members -> institutions (institution_id));
diesel::joinable!(organization_unit_members -> organization_units (organization_unit_id));
diesel::joinable!(organization_unit_members -> organizations (organization_id));
diesel::joinable!(organization_units -> customers (customer_id));
diesel::joinable!(organization_units -> organizations (organization_id));
diesel::joinable!(organizations -> customers (customer_id));

diesel::allow_tables_to_appear_in_same_query!(
    customers,
    institutions,
    organization_unit_members,
    organization_units,
    organizations,
);
