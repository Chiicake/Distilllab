use rusqlite::{Connection, Result, params};
use schema::RunRecord;

pub fn insert_run(conn: &Connection, run: &RunRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO runs (id, run_type, status, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            run.id,
            run.run_type.as_str(),
            run.status.as_str(),
            run.created_at
        ],
    )?;
    Ok(())
}
