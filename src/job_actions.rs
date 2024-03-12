use diesel::prelude::*;

use crate::db;
use crate::model::*;
use crate::schema::jobs;
use crate::schema::jobs::dsl::*;

#[allow(dead_code)]
pub fn find_all(limit: Option<i64>) -> Result<Vec<Job>, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let results = match limit {
        Some(l) => {
            let select_statement = jobs.order(date_submitted.asc()).limit(l).select(Job::as_select());
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&select_statement).to_string());
            select_statement.load(&mut conn).expect("failed to find all")
        }
        None => {
            let select_statement = jobs.order(date_submitted.asc()).select(Job::as_select());
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&select_statement).to_string());
            select_statement.load(&mut conn).expect("failed to find all")
        }
    };
    Ok(results)
}

pub fn find_by_id(gid: i32) -> Result<Option<Job>, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let job = jobs.filter(id.eq(gid)).select(Job::as_select());
    // debug!("{}", debug_query::<diesel::pg::Pg, _>(&job).to_string());
    let results = job.first(&mut conn).optional()?;
    Ok(results)
}

pub fn find_undone() -> Result<Vec<Job>, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = jobs
        .filter(status.eq(JobStatus::Queued))
        .order(date_submitted.asc())
        // .limit(1)
        .select(Job::as_select());
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    let results = statement.load(&mut conn).expect("failed to find all");
    Ok(results)
}

#[allow(dead_code)]
pub fn reset_running() -> Result<(), diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = jobs.filter(status.eq(JobStatus::Running)).order(date_submitted.asc()).select(Job::as_select());
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    let mut results = statement.load(&mut conn).expect("failed to find all");
    for job in results.iter_mut() {
        job.date_started = None;
        job.status = JobStatus::Queued;
        update(job).expect(format!("Could not update Job: {}", job.id).as_str());
    }
    Ok(())
}

pub fn insert(new_job: &NewJob) -> Result<i32, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let insert = diesel::insert_into(jobs::table).values(new_job);
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&insert).to_string());
    let result: QueryResult<i32> = insert.returning(id).get_result(&mut conn);
    Ok(result.expect("did not get job id"))
}

#[allow(dead_code)]
pub fn delete(gid: &i32) -> Result<bool, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = diesel::delete(jobs.filter(id.eq(gid)));
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    let num_deleted = statement.execute(&mut conn)?;
    debug!("num_deleted: {}", num_deleted);
    Ok(num_deleted == 1)
}

pub fn update(job: &Job) -> Result<bool, diesel::result::Error> {
    let mut conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let statement = diesel::update(jobs::table.filter(id.eq(job.id))).set(job);
    // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
    let num_updated = statement.execute(&mut conn)?;
    debug!("num_updated: {}", num_updated);
    Ok(num_updated == 1)
}
