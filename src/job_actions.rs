use crate::model::*;
use crate::schema::jobs;
use crate::schema::jobs::dsl::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[allow(dead_code)]
pub async fn find_all(limit: Option<i64>) -> Result<Vec<Job>, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
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
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
            Ok(vec![])
        }
    }
}

pub async fn find_by_id(gid: i32) -> Result<Option<Job>, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
            let job = jobs.filter(id.eq(gid)).select(Job::as_select());
            // debug!("{}", debug_query::<diesel::pg::Pg, _>(&job).to_string());
            job.first(&mut conn).await.optional()
        }
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
            Ok(None)
        }
    }
}

pub async fn find_undone() -> Result<Vec<Job>, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
            let statement = jobs
                .filter(status.eq(JobStatus::Queued))
                .order(date_submitted.asc())
                // .limit(1)
                .select(Job::as_select());
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
            statement.load::<Job>(&mut conn).await
        }
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
            Ok(vec![])
        }
    }
}

pub async fn insert(new_job: &NewJob) -> Result<i32, diesel::result::Error> {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
            let insert = diesel::insert_into(jobs::table).values(new_job);
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&insert).to_string());
            insert.returning(id).get_result(&mut conn).await
        }
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
            Ok(0)
        }
    }
}

#[allow(dead_code)]
pub async fn delete(gid: &i32) {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
            let statement = diesel::delete(jobs.filter(id.eq(gid)));
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
            let num_deleted = statement.execute(&mut conn).await;
            debug!("num_deleted: {}", num_deleted.unwrap());
            // Ok(num_deleted.unwrap() == 1)
        }
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
        }
    }
}

pub async fn delete_many(ids: Vec<i32>) {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
            let statement = diesel::delete(jobs.filter(id.eq_any(ids)));
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
            let num_deleted = statement.execute(&mut conn).await;
            debug!("num_deleted: {}", num_deleted.unwrap());
        }
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
        }
    }
}

pub async fn update(job: &Job) {
    let pool = crate::DB_POOL.get().await;
    match pool.get().await {
        Ok(mut conn) => {
            let statement = diesel::update(jobs::table.filter(id.eq(job.id))).set(job);
            // debug!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&statement).to_string());
            let num_updated = statement.execute(&mut conn).await;
            debug!("num_updated: {}", num_updated.unwrap());
            // Ok(num_updated.unwrap() == 1)
        }
        Err(e) => {
            error!("There was a problem getting a connection: {}", e);
        }
    }
}
