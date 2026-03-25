use rusqlite::{Connection, Result};

// 打开一个 SQLite 数据库连接。
pub fn open_database(path: &str) -> Result<Connection> {
    Connection::open(path)
}
