use diesel::debug_query;
use diesel::pg::Pg;
use diesel::prelude::*;

use crate::db;
use crate::model;
use crate::model::JobStatus;
use crate::schema::jobs;

#[allow(dead_code)]
pub fn find_all(limit: Option<i64>) -> Result<Vec<model::Job>, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let results = match limit {
        Some(l) => {
            let select_statement = jobs::dsl::jobs.order(jobs::dsl::date_submitted.asc()).limit(l).select(model::Job::as_select());
            debug!("{}", debug_query::<Pg, _>(&select_statement).to_string());
            select_statement.load(&mut conn).expect("failed to find all")
        }
        None => {
            let select_statement = jobs::dsl::jobs.order(jobs::dsl::date_submitted.asc()).select(model::Job::as_select());
            debug!("{}", debug_query::<Pg, _>(&select_statement).to_string());
            select_statement.load(&mut conn).expect("failed to find all")
        }
    };
    Ok(results)
}

pub fn find_by_id(gid: i32) -> Result<Option<model::Job>, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let job = jobs::dsl::jobs.filter(jobs::dsl::id.eq(gid)).select(model::Job::as_select());
    debug!("{}", debug_query::<Pg, _>(&job).to_string());
    let results = job.first(&mut conn).optional()?;
    Ok(results)
}

pub fn find_undone() -> Result<Vec<model::Job>, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = jobs::dsl::jobs
        .filter(jobs::dsl::status.ne(JobStatus::Completed).and(jobs::dsl::status.ne(JobStatus::Failed)))
        .order(jobs::dsl::date_submitted.asc())
        .select(model::Job::as_select());
    debug!("{}", debug_query::<Pg, _>(&statement).to_string());
    let results = statement.load(&mut conn).expect("failed to find all");
    Ok(results)
}

pub fn insert(new_job: &model::NewJob) -> Result<i32, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let insert = diesel::insert_into(jobs::table).values(new_job);
    debug!("{}", debug_query::<Pg, _>(&insert).to_string());
    let result: QueryResult<i32> = insert.returning(jobs::id).get_result(&mut conn);
    Ok(result.expect("did not get job id"))
}

#[allow(dead_code)]
pub fn delete(gid: &i32) -> Result<bool, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = diesel::delete(jobs::table.filter(jobs::dsl::id.eq(gid)));
    debug!("{}", debug_query::<Pg, _>(&statement).to_string());
    let num_deleted = statement.execute(&mut conn)?;
    debug!("num_deleted: {}", num_deleted);
    Ok(num_deleted == 1)
}

pub fn update(job: &model::Job) -> Result<bool, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = diesel::update(jobs::table.filter(jobs::dsl::id.eq(job.id))).set(job);
    debug!("{}", debug_query::<Pg, _>(&statement).to_string());
    let num_updated = statement.execute(&mut conn)?;
    debug!("num_updated: {}", num_updated);
    Ok(num_updated == 1)
}
