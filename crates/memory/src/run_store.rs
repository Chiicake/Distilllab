use rusqlite::{Connection, Result, params};
use schema::RunRecord;

pub fn insert_run(conn: &Connection, run: &RunRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO runs (id, run_type, status, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            run.id,
            format!("{:?}", run.run_type),
            format!("{:?}", run.status),
            run.created_at
        ],
    )?;
    Ok(())
}
