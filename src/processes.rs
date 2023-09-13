use once_cell::sync::Lazy;
use regex::Regex;
use std::{collections::HashSet, ffi::OsString, os::windows::ffi::OsStringExt, ptr::null_mut, time::Duration};
use winapi::{
	shared::{
		minwindef::{DWORD, LPARAM, MAX_PATH},
		windef::HWND,
	},
	um::{
		processthreadsapi::OpenProcess,
		psapi::GetModuleFileNameExW,
		winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
		winuser::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible},
	},
};

pub(crate) struct Process {
	/// Name of the process
	pub(crate) name: String,
	/// Pretty version of the process name
	pub(crate) pretty_name: String,
	/// Path to the executable
	pub(crate) path: String,
}

const IGNORED_PROCESSES: [&str; 2] = [
	"mbamtray.exe", // spell-checker:disable-line
	"NVIDIA Share.exe",
];

const IGNORED_PATHS: [&str; 1] = [
	"C:\\Windows", //
];

const SPECIAL_CASES: [(&str, &str); 2] = [
	("Spotify.exe", "Spotify"),
	("datagrip64.exe", "DataGrip"), // spell-checker:disable-line
];

const NAME_SEPARATORS: [&str; 3] = ["-", "_", "."];
const EXTENSION: &str = ".exe";

pub(crate) static QUERY_INTERVAL: Lazy<Duration> = Lazy::new(|| {
	let interval = if let Ok(interval) = std::env::var("PT_INTERVAL") {
		if let Ok(interval) = humantime::parse_duration(&interval) {
			interval
		} else {
			panic!("PT_INTERVAL env var is not a valid duration");
		}
	} else {
		Duration::from_secs(10)
	};

	if interval.as_secs() > 3600 {
		panic!("PT_INTERVAL env var is too large");
	} else if interval.as_secs() < 1 {
		panic!("PT_INTERVAL env var is too small");
	}

	interval
});

#[must_use]
/// Returns a list of processes that are relevant to the user
pub(crate) fn get_process_list() -> Vec<Process> {
	let mut windows: Vec<(String, String)> = Vec::new();

	unsafe {
		EnumWindows(
			Some(enum_windows_callback),
			&mut windows as *mut Vec<(String, String)> as LPARAM,
		);
	}

	let mut seen_paths: HashSet<&String> = HashSet::new();

	windows
		.iter()
		.filter_map(|(name, path)| {
			if !seen_paths.contains(&path) {
				seen_paths.insert(path);
				Some(Process {
					name: name.to_string(),
					pretty_name: pretty_process_name(&path, &name),
					path: path.to_string(),
				})
			} else {
				None // Skip duplicate processes
			}
		})
		.collect::<Vec<Process>>()
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, data: LPARAM) -> i32 {
	let windows = &mut *(data as *mut Vec<(String, String)>);

	let mut buffer: [u16; 512] = [0; 512];
	if IsWindowVisible(hwnd) != 0 && GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) > 0 {
		let title = OsString::from_wide(&buffer);

		let process_id = get_process_id_for_window(hwnd);
		if process_id != 0 {
			if let Some(path) = get_application_name(process_id) {
				let title = title.to_string_lossy().replace("\0", "").to_string();
				let path = path.to_string_lossy().to_string();

				if is_relevant_process(&path) {
					windows.push((title, path));
				}
			}
		}
	}

	1 // Continue enumeration
}

#[must_use]
fn get_process_id_for_window(hwnd: HWND) -> DWORD {
	let mut process_id: DWORD = 0;
	unsafe {
		GetWindowThreadProcessId(hwnd, &mut process_id);
	}
	process_id
}

#[must_use]
fn get_application_name(process_id: DWORD) -> Option<OsString> {
	unsafe {
		let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process_id);
		if process_handle.is_null() {
			return None;
		}

		let mut exe_path: Vec<u16> = vec![0; MAX_PATH];
		let exe_path_len = GetModuleFileNameExW(process_handle, null_mut(), exe_path.as_mut_ptr(), MAX_PATH as DWORD);

		if exe_path_len == 0 {
			return None;
		}

		let exe_path = OsString::from_wide(&exe_path[..(exe_path_len as usize)]);
		Some(exe_path)
	}
}

/// Returns a pretty version of a process executable
#[must_use]
pub(crate) fn pretty_process_name(path: &str, title: &str) -> String {
	// Special cases
	if let Some((_, pretty_name)) = SPECIAL_CASES.iter().find(|(n, _)| path.ends_with(n)) {
		return pretty_name.to_string();
	}

	if title.contains(" - ") {
		return title.split(" - ").last().unwrap().to_string();
	} else if title.len() > 0 {
		return title.to_string();
	}

	panic!("Could not get pretty name for process: {} ({})", path, title);

	let name = if path.starts_with("C:\\") {
		path.split('\\').last().unwrap().to_string()
	} else {
		path.to_string()
	};

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
fn is_relevant_process(path: &str) -> bool {
	let name = path.split('\\').last().unwrap();
	!IGNORED_PROCESSES.contains(&name) & !IGNORED_PATHS.iter().any(|p| path.starts_with(&*p))
}

#[cfg(test)]
mod tests {
	use super::pretty_process_name;

	#[test]
	fn test_pretty_process_name() {
		assert_eq!(
			pretty_process_name("chrome.exe", "Jay3 - Twitch - Google Chrome"),
			"Google Chrome"
		);
		assert_eq!(
			pretty_process_name("Discord.exe", "#general | Lurkr Support - Discord"),
			"Discord"
		);
		assert_eq!(
			pretty_process_name("LegionFanControl.exe", "LegionFanControl"),
			"LegionFanControl"
		);
		assert_eq!(
			pretty_process_name("Microsoft.SharePoint.exe", ""),
			"Microsoft SharePoint"
		);
		assert_eq!(pretty_process_name("process-tracker.exe", ""), "Process Tracker");
		assert_eq!(pretty_process_name("Razer Central.exe", ""), "Razer Central");
		assert_eq!(pretty_process_name("ShareX.exe", "ShareX"), "ShareX");
		assert_eq!(pretty_process_name("wallpaper32.exe", ""), "Wallpaper32");
		assert_eq!(pretty_process_name("ui32.exe", "Wallpaper UI"), "Wallpaper UI");
	}
}
