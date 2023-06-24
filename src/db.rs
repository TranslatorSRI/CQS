use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use std::error::Error;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

lazy_static! {
    pub static ref DB_POOL: DbPool = create_db_connection_pool();
}

pub fn create_db_connection_pool() -> DbPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().max_size(4).build(manager).expect("Failed to create pool.");
    pool
}

pub fn init_db() -> Result<(), Box<dyn Error>> {
    let mut conn = DB_POOL.get().expect("failed to get db connection from pool");
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    Ok(())
}
