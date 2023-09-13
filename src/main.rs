mod db;
mod processes;

use humantime::format_duration;
use rusqlite::Error;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), Error> {
	let mut system = processes::setup_system();

	let conn = db::setup_database()?;
	let interval = get_interval();
	let formatted_interval = format_duration(interval);

	loop {
		clear_terminal();

		let process_list = processes::get_process_list(&mut system);

		for p in &process_list {
			let duration = format_duration(Duration::from_secs(p.running_time));

			println!("{} ({}): Running for: {}", p.pretty_name, p.count, duration);
		}

		db::update_processes(&conn, &process_list)?;

		println!(
			"Found {} processes. Updating in {}",
			process_list.len(),
			formatted_interval
		);

		sleep(interval);
	}
}

#[inline(always)]
fn clear_terminal() {
	print!("\x1B[2J\x1B[1;1H");
}

fn get_interval() -> Duration {
	if let Ok(interval) = std::env::var("PT_INTERVAL") {
		if let Ok(interval) = humantime::parse_duration(&interval) {
			interval
		} else {
			panic!("PT_INTERVAL env var is not a valid duration");
		}
	} else {
		Duration::from_secs(10)
	}
}
