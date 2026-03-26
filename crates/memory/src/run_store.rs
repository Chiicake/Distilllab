use rusqlite::{params, types::Type, Connection, Error, Result};
use schema::run::RunType;
use schema::{RunRecord, RunState};

pub fn insert_run(conn: &Connection, run: &RunRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO runs (id, run_type, status, primary_object_type, primary_object_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            run.id,
            run.run_type.as_str(),
            run.status.as_str(),
            run.primary_object_type.as_str(),
            run.primary_object_id.as_str(),
            run.created_at
        ],
    )?;
    Ok(())
}

pub fn list_runs(conn: &Connection) -> Result<Vec<RunRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_type, status, primary_object_type, primary_object_id, created_at FROM runs ORDER BY created_at DESC",
    )?;

    let run_iter = stmt.query_map([], |row| {
        let run_type_str: String = row.get(1)?;
        let status_str: String = row.get(2)?;

        let run_type = RunType::from_str(&run_type_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                1,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid run type: {run_type_str}"),
                )),
            )
        })?;

        let status = RunState::from_str(&status_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                2,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid run status: {status_str}"),
                )),
            )
        })?;

        Ok(RunRecord {
            id: row.get(0)?,
            run_type,
            status,
            primary_object_type: row.get(3)?,
            primary_object_id: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    let mut runs = Vec::new();
    for run in run_iter {
        runs.push(run?);
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    #[test]
    fn lists_inserted_runs() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let run = RunRecord {
            id: "run-1".to_string(),
            run_type: RunType::Demo,
            status: RunState::Completed,
            primary_object_type: "source".to_string(),
            primary_object_id: "source-1".to_string(),
            created_at: "2026-03-25T00:00:00Z".to_string(),
        };

        insert_run(&conn, &run).expect("failed to insert run");

        let runs = list_runs(&conn).expect("failed to list runs");

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, "run-1");
        assert_eq!(runs[0].run_type.as_str(), "demo");
        assert_eq!(runs[0].status.as_str(), "completed");
        assert_eq!(runs[0].primary_object_type, "source");
        assert_eq!(runs[0].primary_object_id, "source-1");
    }
}
