use rusqlite::{params, types::Type, Connection, Error, Result};
use schema::{Asset, AssetType};

pub fn insert_asset(conn: &Connection, asset: &Asset) -> Result<()> {
    conn.execute(
        "INSERT INTO assets (id, project_id, asset_type, title, summary) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            asset.id,
            asset.project_id,
            asset.asset_type.as_str(),
            asset.title,
            asset.summary
        ],
    )?;

    Ok(())
}

pub fn list_assets(conn: &Connection) -> Result<Vec<Asset>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, asset_type, title, summary FROM assets ORDER BY title ASC",
    )?;

    let asset_iter = stmt.query_map([], |row| {
        let asset_type_str: String = row.get(2)?;
        let asset_type = AssetType::from_str(&asset_type_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                2,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid asset type: {asset_type_str}"),
                )),
            )
        })?;

        Ok(Asset {
            id: row.get(0)?,
            project_id: row.get(1)?,
            asset_type,
            title: row.get(3)?,
            summary: row.get(4)?,
        })
    })?;

    let mut assets = Vec::new();
    for asset in asset_iter {
        assets.push(asset?);
    }

    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    #[test]
    fn inserts_and_lists_assets() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let asset = Asset {
            id: "asset-1".to_string(),
            project_id: "project-1".to_string(),
            asset_type: AssetType::Insight,
            title: "Runtime Insight".to_string(),
            summary: "A minimal demo asset".to_string(),
        };

        insert_asset(&conn, &asset).expect("failed to insert asset");

        let assets = list_assets(&conn).expect("failed to list assets");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "asset-1");
        assert_eq!(assets[0].asset_type.as_str(), "insight");
        assert_eq!(assets[0].title, "Runtime Insight");
    }
}
