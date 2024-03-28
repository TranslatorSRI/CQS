use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::model::*;
use crate::schema::jobs;
use crate::schema::jobs::dsl::*;

#[allow(dead_code)]
pub async fn find_all(limit: Option<i64>) -> Result<Vec<Job>, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    let mut conn = pool.get().await.unwrap();
    match limit {
        Some(l) => {
            let select_statement = jobs.order(date_submitted.asc()).limit(l).select(Job::as_select());
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&select_statement).to_string());
            select_statement.load(&mut conn).await
        }
        None => {
            let select_statement = jobs.order(date_submitted.asc()).select(Job::as_select());
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&select_statement).to_string());
            select_statement.load(&mut conn).await
        }
    }
}

pub async fn find_by_id(gid: i32) -> Result<Option<Job>, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    let mut conn = pool.get().await.unwrap();
    let job = jobs.filter(id.eq(gid)).select(Job::as_select());
    // debug!("{}", debug_query::<diesel::pg::Pg, _>(&job).to_string());
    job.first(&mut conn).await.optional()
}

pub async fn find_undone() -> Result<Vec<Job>, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    let mut conn = pool.get().await.unwrap();
    let statement = jobs
        .filter(status.eq(JobStatus::Queued))
        .order(date_submitted.asc())
        // .limit(1)
        .select(Job::as_select());
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    statement.load::<Job>(&mut conn).await
}

pub async fn insert(new_job: &NewJob) -> Result<i32, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    let mut conn = pool.get().await.unwrap();
    let insert = diesel::insert_into(jobs::table).values(new_job);
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&insert).to_string());
    insert.returning(id).get_result(&mut conn).await
}

#[allow(dead_code)]
pub async fn delete(gid: &i32) {
    let pool = crate::DB_POOL.get().await;
    let mut conn = pool.get().await.unwrap();
    let statement = diesel::delete(jobs.filter(id.eq(gid)));
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    let num_deleted = statement.execute(&mut conn).await;
    debug!("num_deleted: {}", num_deleted.unwrap());
    // Ok(num_deleted.unwrap() == 1)
}

pub async fn update(job: &Job) {
    let pool = crate::DB_POOL.get().await;
    let mut conn = pool.get().await.unwrap();
    let statement = diesel::update(jobs::table.filter(id.eq(job.id))).set(job);
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    let num_updated = statement.execute(&mut conn).await;
    debug!("num_updated: {}", num_updated.unwrap());
    // Ok(num_updated.unwrap() == 1)
}
