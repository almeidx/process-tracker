use crate::processes::{Process, QUERY_INTERVAL};
use chrono::NaiveDateTime;
use once_cell::sync::Lazy;
use rusqlite::{params, Connection, Result};
use std::{collections::HashSet, fs::create_dir_all, time::SystemTime};

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
			running_time INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
		(),
	)?;

	Ok(conn)
}

/// Updates the database with the latest process list
///
/// The way this works is that it first will get the latest process times for each process from the last hour
/// It will then iterate over this list and find the corresponding process in the current process list
/// If it finds a match, it calculate the time at which the row was created and adds the time since then to the running time
/// If it doesn't find a match, it will create a new process row
///
/// After this, it will iterate over the current process list and insert any processes that don't exist in the database
pub(crate) fn update_processes(conn: &Connection, process_list: &Vec<Process>) -> Result<()> {
	let mut last_processes_stmt = conn.prepare(
		"SELECT path, process_id, running_time, MAX(process_times.created_at)
		FROM process_times
		INNER JOIN processes ON processes.id = process_times.process_id
		WHERE process_times.created_at > datetime('now', '-1 hour')
		GROUP BY process_id, path",
	)?;

	let mut last_processes = last_processes_stmt.query([])?;

	let mut existing_processes = HashSet::<String>::new();

	while let Some(last_process) = last_processes.next()? {
		if let Some(process) = process_list
			.iter()
			.find(|p| p.path == last_process.get::<usize, String>(0).unwrap())
		{
			let process_id = last_process.get::<usize, i64>(1).unwrap();
			let running_time = last_process.get::<usize, u64>(2).unwrap();
			let created_at = last_process.get::<usize, String>(3).unwrap();

			let running_time = get_new_running_time(created_at, running_time);

			conn.execute(
				"UPDATE process_times SET running_time = ?1 WHERE process_id = ?2",
				params![running_time, process_id],
			)?;

			existing_processes.insert(process.path.clone());
		}
	}

	for process in process_list {
		if existing_processes.contains(&process.path) {
			continue;
		}

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
			"INSERT INTO process_times (process_id) VALUES (?1)",
			params![process_id],
		)?;
	}

	Ok(())
}

/// Returns an approximation for the time elapsed between the last time the process was updated and now
///
/// In the even that the time was updated more than 2x the query interval, it will just return the query interval
/// Given that probably means the program was closed and reopened
///
/// Otherwise, it will return the time elapsed since the last update
fn get_new_running_time(created_at: String, running_time: u64) -> u64 {
	let created_at = NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S").unwrap();

	let elapsed_time = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs() - created_at.timestamp() as u64;
	if elapsed_time > (QUERY_INTERVAL.as_secs() * 2) {
		return running_time + QUERY_INTERVAL.as_secs();
	}

	running_time + elapsed_time
}
