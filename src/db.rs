use crate::processes::Process;
use once_cell::sync::Lazy;
use rusqlite::{params, Connection, Result};
use std::fs::create_dir_all;

static DATA_FOLDER: Lazy<String> = Lazy::new(|| {
	let username = whoami::username();

	format!("C:\\Users\\{}\\AppData\\Local\\ProcessTracker", username)
});

static DATABASE_PATH: Lazy<String> = Lazy::new(|| DATA_FOLDER.to_string() + "\\db.sqlite");

pub(crate) fn setup_database() -> Result<Connection, rusqlite::Error> {
	create_dir_all(DATA_FOLDER.to_string()).expect("Failed to create data folder");

	let conn = Connection::open(DATABASE_PATH.to_string())?;

	conn.execute(
		"CREATE TABLE IF NOT EXISTS processes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            pretty_name TEXT NOT NULL,
            path TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
		(),
	)?;

	conn.execute(
		"CREATE TABLE IF NOT EXISTS process_times (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            process_id INTEGER NOT NULL REFERENCES processes(id) ON DELETE CASCADE,
            process_count INTEGER NOT NULL,
            running_time INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
		(),
	)?;

	Ok(conn)
}

pub(crate) fn update_processes(conn: &Connection, process_list: &Vec<Process>) -> Result<()> {
	for process in process_list {
		let process_id = match conn.query_row(
			"SELECT id FROM processes WHERE name = ?1",
			params![process.name],
			|row| row.get(0),
		) {
			Ok(id) => id,
			Err(_) => {
				conn.execute(
					"INSERT INTO processes (name, pretty_name, path) VALUES (?1, ?2, ?3)",
					params![process.name, process.pretty_name, process.path],
				)?;

				conn.last_insert_rowid()
			}
		};

		conn.execute(
			"INSERT INTO process_times (process_id, process_count, running_time) VALUES (?1, ?2, ?3)",
			params![process_id, process.count, process.running_time],
		)?;
	}

	Ok(())
}
