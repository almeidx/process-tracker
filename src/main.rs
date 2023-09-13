mod db;
mod processes;

use crate::processes::QUERY_INTERVAL;
use humantime::format_duration;
use rusqlite::Error;
use std::thread::sleep;

fn main() -> Result<(), Error> {
	let conn = db::setup_database()?;
	let formatted_interval = format_duration(*QUERY_INTERVAL);

	loop {
		clear_terminal();

		let mut process_list = processes::get_process_list();

		process_list.sort_by(|a, b| a.pretty_name.to_lowercase().cmp(&b.pretty_name.to_lowercase()));

		println!(
			"Found {} processes. Updating in {}",
			process_list.len(),
			formatted_interval
		);

		for p in &process_list {
			println!("- {} ({})", p.pretty_name, p.path);
		}

		db::update_processes(&conn, &process_list)?;

		sleep(*QUERY_INTERVAL);
	}
}

#[inline(always)]
fn clear_terminal() {
	print!("\x1B[2J\x1B[1;1H");
}
