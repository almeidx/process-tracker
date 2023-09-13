use once_cell::sync::Lazy;
use regex::Regex;
use sysinfo::{ProcessExt, ProcessStatus, System, SystemExt};

pub(crate) struct Process {
	/// Name of the process
	pub(crate) name: String,
	/// Pretty version of the process name
	pub(crate) pretty_name: String,
	/// Path to the executable
	pub(crate) path: String,
	/// Number of instances of this process
	pub(crate) count: u32,
	/// Number of seconds this process has been running for
	pub(crate) running_time: u64,
}

const IGNORED_PROCESSES: [&str; 21] = [
	// spell-checker:disable
	"cargo.exe",
	"CefSharp.BrowserSubprocess.exe",
	"crashpad_handler.exe",
	"explorer.exe",
	"fsnotifier.exe",
	"GoogleDriveFS.exe",
	"LSB.exe",
	"MbamBgNativeMsg.exe",
	"mbamtray.exe",
	"msedgewebview2.exe",
	"MSPCManagerService.exe",
	"nvcontainer.exe",
	"NVIDIA Share.exe",
	"NVIDIA Web Helper.exe",
	"nvsphelper64.exe",
	"OneDrive.exe",
	"QSHelper.exe",
	"Razer Synapse Service Process.exe",
	"steamwebhelper.exe",
	"vctip.exe",
	"XboxGameBarSpotify.exe",
	// spell-checker:enable
];

static IGNORED_PATHS: Lazy<Vec<String>> = Lazy::new(|| {
	let username = whoami::username();

	vec![
		// spell-checker:disable
		"C:\\Program Files (x86)\\Lenovo\\VantageService".to_string(),
		"C:\\Program Files\\Git".to_string(),
		"C:\\Program Files\\PowerToys\\PowerToys.".to_string(),
		"C:\\Program Files\\WindowsApps\\MicrosoftWindows.Client".to_string(),
		"C:\\Windows".to_string(),
		format!("C:\\Users\\{}\\.rustup", username).to_string(),
		format!("C:\\Users\\{}\\.vscode", username).to_string(),
		format!("C:\\Users\\{}\\.wakatime", username).to_string(),
		// spell-checker:enable
	]
});

const NAME_SEPARATORS: [&str; 3] = ["-", "_", "."];
const EXTENSION: &str = ".exe";

pub(crate) fn setup_system() -> System {
	System::new_with_specifics(sysinfo::RefreshKind::new().with_processes(sysinfo::ProcessRefreshKind::new()))
}

#[must_use]
/// Returns a list of processes that are relevant to the user
pub(crate) fn get_process_list(system: &mut System) -> Vec<Process> {
	system.refresh_processes_specifics(sysinfo::ProcessRefreshKind::new());

	let processes = system.processes();
	let mut process_list: Vec<Process> = Vec::new();

	for process in processes.values() {
		let name = process.name();
		let path = process.exe().to_str().unwrap();

		if !is_relevant_process(&name, &path, process.status()) {
			continue;
		}

		if let Some(p) = process_list.iter_mut().find(|p| p.name == name) {
			p.count += 1;

			let run_time = process.run_time();
			if p.running_time < run_time {
				p.running_time = run_time;
			}
		} else {
			process_list.push(Process {
				name: name.to_string(),
				pretty_name: pretty_process_name(&name),
				path: path.to_string(),
				count: 1,
				running_time: process.run_time(),
			});
		}
	}

	process_list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

	process_list
}

/// Returns a pretty version of a process executable
#[must_use]
pub(crate) fn pretty_process_name(name: &str) -> String {
	// trim the .exe extension
	let name = name.trim_end_matches(EXTENSION).to_string();

	// if name contains a separator, make it Title Case
	if let Some(separator) = NAME_SEPARATORS.iter().find(|s| name.contains(*s)) {
		return name
			.split(separator)
			.map(|part| {
				let first_char = part.chars().next().unwrap().to_uppercase().to_string();
				let rest = part.chars().skip(1).collect::<String>();

				first_char + &rest
			})
			.collect::<Vec<String>>()
			.join(" ");
	}

	// if name is all lowercase, make it Title Case
	if name.chars().all(|c| c.is_lowercase() | c.is_numeric()) {
		return name.chars().next().unwrap().to_uppercase().to_string() + &name.chars().skip(1).collect::<String>();
	}

	// if name is in PascalCase, make it Title Case
	let re = Regex::new(r"([A-Z][a-z]+)").unwrap();
	let name = re.replace_all(&name, " $1").trim_start().to_string();

	// trim extra whitespace
	name.split_whitespace()
		.map(|s| s.to_string())
		.collect::<Vec<String>>()
		.join(" ")
}

#[must_use]
fn is_relevant_process(name: &str, path: &str, status: ProcessStatus) -> bool {
	(path.len() > 0)
		& (status == ProcessStatus::Run)
		& !IGNORED_PROCESSES.contains(&name)
		& !IGNORED_PATHS.iter().any(|p| path.starts_with(&*p))
}

#[cfg(test)]
mod tests {
	use super::pretty_process_name;

	#[test]
	fn test_pretty_process_name() {
		assert_eq!(pretty_process_name("chrome.exe"), "Chrome");
		assert_eq!(pretty_process_name("Discord.exe"), "Discord");
		assert_eq!(pretty_process_name("LegionFanControl.exe"), "Legion Fan Control");
		assert_eq!(pretty_process_name("Microsoft.SharePoint.exe"), "Microsoft SharePoint");
		assert_eq!(pretty_process_name("process-tracker.exe"), "Process Tracker");
		assert_eq!(pretty_process_name("Razer Central.exe"), "Razer Central");
		assert_eq!(pretty_process_name("ShareX.exe"), "ShareX");
		assert_eq!(pretty_process_name("wallpaper32.exe"), "Wallpaper32");
	}
}
