use humantime::format_duration;
use once_cell::sync::Lazy;
use std::thread::sleep;
use std::time::Duration;
use sysinfo::{ProcessExt, ProcessStatus, System, SystemExt};

const IGNORED_PROCESSES: [&str; 20] = [
    // spell-checker:disable
    "cargo.exe",
    "CefSharp.BrowserSubprocess.exe",
    "crashpad_handler.exe",
    "explorer.exe",
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

#[allow(dead_code)]
struct Process {
    /// Name of the process
    name: String,
    /// Path to the executable
    path: String,
    /// Number of instances of this process
    count: u32,
    /// Number of seconds this process has been running for
    running_time: u64,
    memory: u64,
}

fn main() {
    println!("Hello, world!");

    let mut system = System::new_all();

    loop {
        let mut process_list = process_process_list(&mut system);

        process_list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // clear terminal
        print!("\x1B[2J\x1B[1;1H");

        for p in &process_list {
            let duration = format_duration(Duration::from_secs(p.running_time));

            println!(
                "{} ({}): Running for: {} ({})",
                p.name,
                p.count,
                duration,
                pretty_bytes(p.memory.into())
            );
        }

        println!("Total processes: {}", process_list.len());

        sleep(Duration::from_secs(5));
    }
}

fn process_process_list(system: &mut System) -> Vec<Process> {
    system.refresh_processes();

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

            p.memory = process.memory();

            let run_time = process.run_time();
            if p.running_time < run_time {
                p.running_time = run_time;
            }
        } else {
            process_list.push(Process {
                name: name.to_string(),
                path: path.to_string(),
                count: 1,
                running_time: process.run_time(),
                memory: process.memory(),
            });
        }
    }

    process_list
}

fn is_relevant_process(name: &str, path: &str, status: ProcessStatus) -> bool {
    (path.len() > 0)
        & (status == ProcessStatus::Run)
        & !IGNORED_PROCESSES.contains(&name)
        & !IGNORED_PATHS.iter().any(|p| path.starts_with(&*p))
}

fn pretty_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}
