use rusqlite::{Connection, Error, Result, params, types::Type};
use schema::{WorkItem, WorkItemType};

pub fn insert_work_item(conn: &Connection, work_item: &WorkItem) -> Result<()> {
    conn.execute(
        "INSERT INTO work_items (id, project_id, work_item_type, title, summary) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            work_item.id,
            work_item.project_id,
            work_item.work_item_type.as_str(),
            work_item.title,
            work_item.summary
        ],
    )?;

    Ok(())
}

pub fn list_work_items(conn: &Connection) -> Result<Vec<WorkItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, work_item_type, title, summary FROM work_items ORDER BY id ASC",
    )?;

    let work_item_iter = stmt.query_map([], |row| {
        let work_item_type_str: String = row.get(2)?;
        let work_item_type = WorkItemType::from_str(&work_item_type_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                2,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid work item type: {work_item_type_str}"),
                )),
            )
        })?;

        Ok(WorkItem {
            id: row.get(0)?,
            project_id: row.get(1)?,
            work_item_type,
            title: row.get(3)?,
            summary: row.get(4)?,
        })
    })?;

    let mut work_items = Vec::new();
    for work_item in work_item_iter {
        work_items.push(work_item?);
    }

    Ok(work_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    #[test]
    fn inserts_and_lists_work_items() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let item = WorkItem {
            id: "work-item-1".to_string(),
            project_id: "project-1".to_string(),
            work_item_type: WorkItemType::Note,
            title: "First Work Item".to_string(),
            summary: "Derived from chunks".to_string(),
        };

        insert_work_item(&conn, &item).expect("failed to insert work item");

        let work_items = list_work_items(&conn).expect("failed to list work items");

        assert_eq!(work_items.len(), 1);
        assert_eq!(work_items[0].id, "work-item-1");
        assert_eq!(work_items[0].work_item_type.as_str(), "note");
        assert_eq!(work_items[0].title, "First Work Item");
    }
}
