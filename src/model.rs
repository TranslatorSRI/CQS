use crate::schema::sql_types::JobStatusType;
use chrono::prelude::*;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Write;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = crate::schema::sql_types::JobStatusType)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JobStatus::Queued => write!(f, "Queued"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

impl ToSql<JobStatusType, Pg> for JobStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            JobStatus::Queued => out.write_all(b"queued")?,
            JobStatus::Running => out.write_all(b"running")?,
            JobStatus::Completed => out.write_all(b"completed")?,
            JobStatus::Failed => out.write_all(b"failed")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<JobStatusType, Pg> for JobStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"queued" => Ok(JobStatus::Queued),
            b"running" => Ok(JobStatus::Running),
            b"completed" => Ok(JobStatus::Completed),
            b"failed" => Ok(JobStatus::Failed),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::jobs, treat_none_as_null = true)]
pub struct Job {
    pub id: i32,
    pub status: JobStatus,
    pub date_submitted: NaiveDateTime,
    pub date_started: Option<NaiveDateTime>,
    pub date_finished: Option<NaiveDateTime>,
    pub query: Vec<u8>,
    pub response: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Insertable)]
#[diesel(table_name = crate::schema::jobs, treat_none_as_null = true)]
pub struct NewJob {
    pub status: JobStatus,
    pub date_submitted: NaiveDateTime,
    pub date_started: Option<NaiveDateTime>,
    pub date_finished: Option<NaiveDateTime>,
    pub query: Vec<u8>,
    pub response: Option<Vec<u8>>,
}

impl NewJob {
    pub fn new(status: JobStatus, query: Vec<u8>) -> NewJob {
        NewJob {
            status,
            date_submitted: Utc::now().naive_utc(),
            date_started: None,
            date_finished: None,
            query,
            response: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct CQSCompositeScoreKey {
    pub subject: String,
    // pub predicate: String,
    pub object: String,
}

impl CQSCompositeScoreKey {
    pub fn new(subject: String, object: String) -> CQSCompositeScoreKey {
        CQSCompositeScoreKey { subject, object }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CQSCompositeScoreValue {
    pub resource_id: String,
    pub knowledge_graph_key: String,
    pub log_odds_ratio: Option<f64>,
    pub total_sample_size: Option<i64>,
}

impl CQSCompositeScoreValue {
    pub fn new(resource_id: String, knowledge_graph_key: String) -> CQSCompositeScoreValue {
        CQSCompositeScoreValue {
            resource_id,
            knowledge_graph_key,
            log_odds_ratio: None,
            total_sample_size: None,
        }
    }
}
