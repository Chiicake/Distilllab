use rusqlite::{Connection, Result, params};
use schema::Project;

pub fn insert_project(conn: &Connection, project: &Project) -> Result<()> {
    conn.execute(
        "INSERT INTO projects (id, name, summary) VALUES (?1, ?2, ?3)",
        params![project.id, project.name, project.summary],
    )?;

    Ok(())
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare("SELECT id, name, summary FROM projects ORDER BY name ASC")?;

    let project_iter = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            summary: row.get(2)?,
        })
    })?;

    let mut projects = Vec::new();
    for project in project_iter {
        projects.push(project?);
    }

    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    #[test]
    fn inserts_and_lists_projects() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let project = Project {
            id: "project-1".to_string(),
            name: "Distilllab".to_string(),
            summary: "Demo project".to_string(),
        };

        insert_project(&conn, &project).expect("failed to insert project");

        let projects = list_projects(&conn).expect("failed to list projects");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, "project-1");
        assert_eq!(projects[0].name, "Distilllab");
    }
}
