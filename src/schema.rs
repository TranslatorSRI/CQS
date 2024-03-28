// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType, QueryId)]
    #[diesel(postgres_type(name = "Job_Status_Type"))]
    pub struct JobStatusType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::JobStatusType;

    jobs (id) {
        id -> Int4,
        status -> JobStatusType,
        date_submitted -> Timestamptz,
        date_started -> Nullable<Timestamptz>,
        date_finished -> Nullable<Timestamptz>,
        query -> Bytea,
        response -> Nullable<Bytea>,
    }
}
