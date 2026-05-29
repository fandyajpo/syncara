use std::path::Path;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "syncara",
    version,
    about = "Smart Traffic Brain — reverse proxy and load balancer",
    long_about = "\
Syncara is a fast, deterministic reverse proxy and load balancer \
for real-time WebSocket and HTTP applications.

Zero-config mode:
  syncara start --backend localhost:3000

Config mode:
  syncara init
  syncara validate
  syncara start

Monitoring:
  syncara status
  syncara doctor
  syncara tune

Manage:
  syncara update
  syncara uninstall
"
)]
struct Cli {
    #[arg(short, long, default_value = "syncara.yml", global = true, help = "Config file path")]
    config: String,

    #[arg(long, default_value = "info", global = true, help = "Log level (trace, debug, info, warn, error)")]
    log_level: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a default configuration file
    Init {
        #[arg(default_value = "syncara.yml")]
        file: String,

        #[arg(long, short)]
        example: Option<String>,
    },

    /// Parse and validate the configuration file
    Validate,

    /// Start the Syncara proxy server (default if no command given)
    Start {
        #[arg(long)]
        backend: Option<String>,

        #[arg(long, default_value = "8080")]
        port: u16,
    },

    /// Show live upstream/pool status from a running Syncara process
    Status {
        #[arg(long, default_value = "http://127.0.0.1:9090")]
        admin: String,
    },

    /// Send reload (SIGHUP) to a running Syncara process
    Reload,

    /// Run diagnostic checks on configuration and process
    Doctor,

    /// System tuning recommendations and diagnostics
    Tune,

    /// Update Syncara to the latest version
    Update {
        #[arg(long, help = "Install a specific version (e.g. 0.1.0)")]
        version: Option<String>,
    },

    /// Uninstall Syncara and remove related files
    Uninstall {
        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Start { backend: None, port: 8080 }) {
        Commands::Init { file, example } => cmd_init(&file, example.as_deref()),
        Commands::Validate => cmd_validate(&cli.config),
        Commands::Start { backend, port } => {
            if let Some(backend) = backend {
                cmd_start_quick(&backend, port, &cli.log_level)
            } else {
                cmd_start(&cli.config, &cli.log_level)
            }
        }
        Commands::Status { admin } => cmd_status(&admin),
        Commands::Reload => cmd_reload(&cli.config),
        Commands::Doctor => cmd_doctor(&cli.config),
        Commands::Tune => cmd_tune(&cli.config),
        Commands::Update { version } => cmd_update(version.as_deref()),
        Commands::Uninstall { force } => cmd_uninstall(force),
    }
}

// ──────────────────────────────────────────────
// Command implementations
// ──────────────────────────────────────────────

fn cmd_init(path: &str, example: Option<&str>) -> anyhow::Result<()> {
    match example {
        None => {
            if Path::new(path).exists() {
                eprintln!("⚠  {path} already exists — not overwriting");
                eprintln!("   Remove it first, or specify a different path:");
                eprintln!("   syncara init <other-file.yml>");
                std::process::exit(1);
            }

            let yaml = syncara_core::config::default_config_yaml();
            std::fs::write(path, &yaml)?;

            eprintln!("✓  Created {path}");
            eprintln!();
            eprintln!("   Next steps:");
            eprintln!("     syncara validate    Check the configuration");
            eprintln!("     syncara start       Start the proxy server");
            eprintln!();
            eprintln!("   Or try an example:");
            eprintln!("     syncara init --example sticky-sessions");
            eprintln!("     syncara init --example brain");
            eprintln!("     syncara init --example websocket");
        }
        Some(name) => {
            let example_path = find_example(name)?;
            let yaml = std::fs::read_to_string(&example_path)
                .map_err(|e| anyhow::anyhow!("failed to read example '{name}': {e}"))?;

            if Path::new(path).exists() {
                eprintln!("⚠  {path} already exists — not overwriting");
                std::process::exit(1);
            }

            std::fs::write(path, &yaml)?;
            eprintln!("✓  Created {path} from example '{name}'");
            eprintln!();
            eprintln!("   Next steps:");
            eprintln!("     syncara validate    Check the configuration");
            eprintln!("     syncara start       Start the proxy server");
        }
    }
    Ok(())
}

fn find_example(name: &str) -> anyhow::Result<String> {
    let binary_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    let search_paths = [
        std::env::current_dir().unwrap_or_default(),
        binary_dir.clone(),
        binary_dir.join(".."),
        binary_dir.join("..").join(".."),
    ];

    for base in &search_paths {
        let examples_dir = base.join("examples");
        if !examples_dir.is_dir() {
            continue;
        }
        let direct = examples_dir.join(name).join("syncara.yml");
        if direct.exists() {
            return Ok(direct.to_string_lossy().to_string());
        }
        if let Ok(entries) = std::fs::read_dir(&examples_dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                let display = fname.split_once('-').map(|(_, n)| n).unwrap_or(&fname);
                if display == name {
                    let candidate = entry.path().join("syncara.yml");
                    if candidate.exists() {
                        return Ok(candidate.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    let mut available = Vec::new();
    for base in &search_paths {
        let examples_dir = base.join("examples");
        if let Ok(entries) = std::fs::read_dir(&examples_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    let display = fname.split_once('-').map(|(_, n)| n).unwrap_or(&fname);
                    if !available.contains(&display.to_string()) {
                        available.push(display.to_string());
                    }
                }
            }
        }
    }
    available.sort();

    eprintln!("✗  Unknown example '{name}'");
    eprintln!();
    eprintln!("   Available examples:");
    for ex in &available {
        eprintln!("     syncara init --example {ex}");
    }
    std::process::exit(1);
}

fn cmd_validate(path: &str) -> anyhow::Result<()> {
    eprintln!("── Config: {path}");

    let config = match syncara_core::config::load(Path::new(path)) {
        Ok(cfg) => cfg,
        Err(e) => {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(ioe) => {
                    eprintln!("✗  Could not read file: {ioe}");
                    std::process::exit(1);
                }
            };
            eprintln!("✗  Parse error: {e}");
            eprintln!();
            friendly_hint(&e.to_string());
            eprintln!();
            let err_msg = e.to_string();
            if let Some(line_info) = err_msg.split(" at line ").nth(1) {
                let line_num: usize = line_info.split(' ').next().and_then(|s| s.parse().ok()).unwrap_or(0);
                show_yaml_context(&content, line_num);
            }
            std::process::exit(1);
        }
    };

    match syncara_core::config::validate(&config) {
        Ok(()) => {
            let listener_count = config.listeners.len();
            let route_count = config.routes.len();
            let pool_count = config.pools.len();
            let upstream_count: usize = config.pools.iter().map(|p| p.upstreams.len()).sum();

            eprintln!("✓  Configuration is valid");
            eprintln!();
            eprintln!("   Listeners: {listener_count}");
            for l in &config.listeners {
                let addr = format!("{}:{}", l.host, l.port);
                eprintln!("     → {addr}");
            }
            eprintln!("   Routes: {route_count}");
            eprintln!("   Pools: {pool_count} ({upstream_count} upstreams)");
            Ok(())
        }
        Err(e) => {
            eprintln!("✗  {e}");
            eprintln!();
            friendly_hint(&e.to_string());
            std::process::exit(1);
        }
    }
}

fn friendly_hint(msg: &str) {
    let msg = msg.to_lowercase();

    if msg.contains("round_robin") || msg.contains("least_connections") || msg.contains("ip_hash") {
        eprintln!("   💡 Tip: Strategy names use hyphens, not underscores.");
        eprintln!("      Try: round-robin, least-connections, ip-hash, weighted, sticky, brain");
    } else if msg.contains("unknown field") && msg.contains("`max_") {
        eprintln!("   💡 Tip: The field name might be slightly different.");
        eprintln!("      Check the config reference for the correct name.");
    } else if msg.contains("deny_unknown_fields") || msg.contains("unknown field") {
        eprintln!("   💡 Tip: Check for typos in field names.");
        eprintln!("      Remove any fields that don't belong.");
    } else if msg.contains("no routes") {
        eprintln!("   💡 Tip: Add at least one route to your config.");
        eprintln!("      Example: routes:\n        - path: /\n          proxy: http://localhost:3000");
    } else if msg.contains("no listeners") {
        eprintln!("   💡 Tip: Add at least one listener to your config.");
        eprintln!("      Example: listeners:\n        - port: 8080");
    } else if msg.contains("no upstreams") {
        eprintln!("   💡 Tip: Each pool needs at least one upstream.");
        eprintln!("      Example: upstreams:\n        - addr: \"localhost:9001\"");
    } else if msg.contains("emfile") || msg.contains("too many open files") {
        eprintln!("   💡 Tip: You may need to increase the open file limit.");
        eprintln!("      Try: ulimit -n 65535");
    } else if msg.contains("address already in use") || msg.contains("eaddrinuse") {
        eprintln!("   💡 Tip: The port is already in use.");
        eprintln!("      Check for another process on the same port.");
    }
}

fn cmd_start(config_path: &str, log_level: &str) -> anyhow::Result<()> {
    let config = syncara_core::config::load(Path::new(config_path))
        .map_err(|e| anyhow::anyhow!("config error: {e}"))?;
    syncara_core::config::validate(&config)
        .map_err(|e| anyhow::anyhow!("config validation failed:\n{e}"))?;

    print_banner(&config);
    write_pid_file();

    let result = syncara_core::bootstrap(config_path, log_level, false);
    let _ = std::fs::remove_file("syncara.pid");
    result
}

fn cmd_start_quick(backend: &str, port: u16, log_level: &str) -> anyhow::Result<()> {
    let config = syncara_core::config::quick_config(backend, port);

    eprintln!("╭──────────────────────────────────────────────╮");
    eprintln!("│  Syncara v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("│  Zero-config mode");
    eprintln!("│  Listening on 0.0.0.0:{port}");
    eprintln!("│  Proxying to {backend}");
    eprintln!("│  Admin: 127.0.0.1:9090");
    eprintln!("╰──────────────────────────────────────────────╯");
    eprintln!();

    write_pid_file();
    let result = syncara_core::bootstrap_with_config(config, log_level);
    let _ = std::fs::remove_file("syncara.pid");
    result
}

fn cmd_status(admin_url: &str) -> anyhow::Result<()> {
    let url = format!("{}/status", admin_url.trim_end_matches('/'));

    let resp = match ureq::get(&url).call() {
        Ok(r) => r.into_string().unwrap_or_default(),
        Err(e) => {
            eprintln!("✗  Could not connect to Syncara at {admin_url}");
            eprintln!("   Make sure the process is running.");
            eprintln!("   Error: {e}");
            std::process::exit(1);
        }
    };

    match serde_json::from_str::<serde_json::Value>(&resp) {
        Ok(json) => {
            println!("── Syncara Status ──────────────────────────");
            println!("Version: {}", json["version"].as_str().unwrap_or("?"));
            println!();

            if let Some(pools) = json["pools"].as_array() {
                for pool in pools {
                    let name = pool["name"].as_str().unwrap_or("?");
                    let strategy = pool["strategy"].as_str().unwrap_or("?");
                    println!("  Pool: {name} (strategy: {strategy})");

                    if let Some(ups) = pool["upstreams"].as_array() {
                        for u in ups {
                            let addr = u["addr"].as_str().unwrap_or("?");
                            let healthy = u["healthy"].as_bool().unwrap_or(false);
                            let active = u["active_connections"].as_u64().unwrap_or(0);
                            let lat = u["latency_ms"].as_f64().unwrap_or(0.0);
                            let status = if healthy { "✓" } else { "✗" };
                            let health_str = if healthy { "healthy" } else { "DOWN" };
                            println!("    {status} {addr:25} {health_str:8}  active: {active:<4}  {lat:.1}ms");
                        }
                    }
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("✗  Failed to parse status response: {e}");
            eprintln!("   Raw: {resp}");
        }
    }
    Ok(())
}

fn cmd_reload(config_path: &str) -> anyhow::Result<()> {
    eprintln!("Checking configuration before reload...");
    cmd_validate(config_path)?;

    eprintln!();
    let pid = read_pid_file()?;

    eprintln!("Sending reload signal to PID {pid}...");
    let status = std::process::Command::new("kill")
        .args(["-s", "HUP", &pid.to_string()])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to send signal: {e}"))?;

    if status.success() {
        eprintln!("✓  Reload signal sent to PID {pid}");
        Ok(())
    } else {
        anyhow::bail!("kill command failed (exit code: {:?})", status.code());
    }
}

fn cmd_doctor(config_path: &str) -> anyhow::Result<()> {
    eprintln!("── Syncara Doctor ──────────────────────────");
    eprintln!();

    // 1. Config check
    eprint!("Config file... ");
    match syncara_core::config::load(Path::new(config_path)) {
        Ok(cfg) => match syncara_core::config::validate(&cfg) {
            Ok(()) => eprintln!("✓  {config_path} is valid"),
            Err(e) => eprintln!("✗  {e}"),
        },
        Err(e) => eprintln!("✗  {e}"),
    }

    // 2. Process check
    eprint!("Process status... ");
    match read_pid_file() {
        Ok(pid) if is_pid_alive(pid) => eprintln!("✓  running (PID {pid})"),
        Ok(pid) => eprintln!("✗  PID file exists but process {pid} is not running (stale)"),
        Err(_) => eprintln!("⚠  not running (no PID file at syncara.pid)"),
    }

    // 3. Port connectivity check
    eprint!("Port checks... ");
    if let Ok(cfg) = syncara_core::config::load(Path::new(config_path)) {
        let mut all_open = true;
        for l in &cfg.listeners {
            let addr = format!("{}:{}", l.host, l.port);
            if tcp_probe(&addr, std::time::Duration::from_secs(2)) {
                eprintln!();
                eprint!("       → {addr} open");
            } else {
                eprintln!();
                eprint!("       → {addr} not responding (expected if server is not running)");
                all_open = false;
            }
        }
        if all_open {
            eprintln!();
            eprintln!("       All listener ports are reachable");
        }
    } else {
        eprintln!("⚠  cannot determine (config invalid)");
    }

    eprintln!();
    eprintln!("── Done ────────────────────────────────────");

    Ok(())
}

fn cmd_tune(config_path: &str) -> anyhow::Result<()> {
    eprintln!("── Syncara Tune ───────────────────────────");
    eprintln!();

    // 1. System info
    eprintln!("System:");
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    eprintln!("  CPU cores: {cpus}");

    // Memory info via sysinfo (or fallback)
    let total_mem_mb = probe_total_memory();
    eprintln!("  Total memory: {} MB", total_mem_mb);

    // Open file limit
    let file_limit = probe_file_limit();
    eprintln!("  Max open files (soft): {file_limit}");

    // 2. Config-based recommendations
    eprintln!();
    eprintln!("Recommendations:");

    if let Ok(cfg) = syncara_core::config::load(Path::new(config_path)) {
        // Upstream count
        let total_upstreams: usize = cfg.pools.iter().map(|p| p.upstreams.len()).sum();
        eprintln!("  Upstreams: {total_upstreams}");

        // Connection limits recommendation
        let conn_limit = cfg.security.connections.as_ref()
            .map(|c| c.max_active)
            .unwrap_or(10_000);

        let suggested_conn = std::cmp::min(file_limit as u32 / 2, 100_000);
        if conn_limit > suggested_conn {
            eprintln!("  ⚠  Connection limit ({conn_limit}) exceeds safe value ({suggested_conn})");
            eprintln!("     Consider reducing security.connections.max_active");
        }

        // Rate limit check
        if cfg.security.rate_limit.as_ref().map(|r| r.enabled).unwrap_or(false) {
            let rpm = cfg.security.rate_limit.as_ref().map(|r| r.requests_per_minute).unwrap_or(300);
            eprintln!("  Rate limit: {rpm} req/min per IP");
        } else {
            eprintln!("  ⚠  Rate limiting is disabled — consider enabling it");
        }
    }

    // 3. Network tuning
    eprintln!();
    eprintln!("Network tuning (if running at scale):");

    let somaxconn_path = "/proc/sys/net/core/somaxconn";
    if Path::new(somaxconn_path).exists() {
        if let Ok(val) = std::fs::read_to_string(somaxconn_path) {
            eprintln!("  net.core.somaxconn = {}", val.trim());
            if let Ok(v) = val.trim().parse::<u32>() {
                if v < 4096 {
                    eprintln!("  ⚠  Low listen backlog — consider: sysctl -w net.core.somaxconn=4096");
                }
            }
        }
    }

    // TCP keepalive
    let tcp_keepalive_path = "/proc/sys/net/ipv4/tcp_keepalive_time";
    if Path::new(tcp_keepalive_path).exists() {
        if let Ok(val) = std::fs::read_to_string(tcp_keepalive_path) {
            eprintln!("  net.ipv4.tcp_keepalive_time = {}s", val.trim());
        }
    }

    // ulimit hint
    if file_limit < 65535 {
        eprintln!("  ⚠  Low file descriptor limit ({file_limit})");
        eprintln!("     For high traffic, set: ulimit -n 65535");
        eprintln!("     Or system-wide:  echo 'fs.file-max = 100000' >> /etc/sysctl.conf");
    }

    eprintln!();
    eprintln!("── Done ────────────────────────────────────");

    Ok(())
}

fn cmd_update(version: Option<&str>) -> anyhow::Result<()> {
    let repo = "fandyajpo/syncara";

    // Resolve version
    let ver = match version {
        Some(v) => v.trim_start_matches('v').to_string(),
        None => {
            eprint!("Checking latest version... ");
            let url = format!("https://api.github.com/repos/{repo}/releases/latest");
            let resp = ureq::get(&url)
                .set("User-Agent", "syncara")
                .call()
                .map_err(|e| anyhow::anyhow!("failed to fetch latest release: {e}"))?;
            let json: serde_json::Value = serde_json::from_str(&resp.into_string()?)?;
            let tag = json["tag_name"].as_str().unwrap_or("v0.1.0");
            let v = tag.trim_start_matches('v').to_string();
            eprintln!("v{v}");
            v
        }
    };

    // Detect platform
    let arch = std::process::Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let os = std::process::Command::new("uname")
        .arg("-s")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let target_arch = match arch.as_str() {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => anyhow::bail!("unsupported architecture: {arch}"),
    };

    let target_os = match os.as_str() {
        "Linux" => "unknown-linux-gnu",
        "Darwin" => "apple-darwin",
        _ => anyhow::bail!("unsupported OS: {os}"),
    };

    let target = format!("{target_arch}-{target_os}");
    let artifact = format!("syncara-{ver}-{target}.tar.gz");
    let url = format!("https://github.com/{repo}/releases/download/v{ver}/{artifact}");

    eprintln!("Downloading Syncara v{ver} ({target})...");

    let tmpdir = tempfile::tempdir().map_err(|e| anyhow::anyhow!("failed to create temp dir: {e}"))?;
    let archive_path = tmpdir.path().join(&artifact);

    let resp = ureq::get(&url)
        .set("User-Agent", "syncara")
        .call()
        .map_err(|e| anyhow::anyhow!("download failed: {e}"))?;

    let mut body = resp.into_reader();
    let mut file = std::fs::File::create(&archive_path)?;
    std::io::copy(&mut body, &mut file)?;
    drop(file);

    // Extract
    eprintln!("Extracting...");
    let archive = std::fs::File::open(&archive_path)?;
    let tar = flate2::read::GzDecoder::new(archive);
    let extracted_dir = tmpdir.path().join("extracted");
    std::fs::create_dir_all(&extracted_dir)?;
    tar::Archive::new(tar).unpack(&extracted_dir)?;

    let binary = extracted_dir.join("syncara");
    if !binary.exists() {
        // Check for syncara.exe on Windows
        let binary_exe = extracted_dir.join("syncara.exe");
        if binary_exe.exists() {
            anyhow::bail!("Windows is not supported for self-update yet");
        }
        anyhow::bail!("downloaded archive does not contain syncara binary");
    }

    // Replace current binary
    let current_exe = std::env::current_exe()?;
    let backup = current_exe.with_extension("old");

    eprintln!("Installing to {}...", current_exe.display());

    // Rename current to backup, move new in place, then remove backup
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }
    std::fs::rename(&current_exe, &backup)?;
    if let Err(e) = std::fs::rename(&binary, &current_exe) {
        // Restore backup on failure
        let _ = std::fs::rename(&backup, &current_exe);
        anyhow::bail!("failed to install update: {e}");
    }
    let _ = std::fs::remove_file(&backup);

    // Make executable (should be already, but just in case)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&current_exe)?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&current_exe, perms)?;
    }

    eprintln!("✓  Updated to Syncara v{ver}");
    eprintln!("   Restart Syncara to use the new version.");

    Ok(())
}

fn cmd_uninstall(force: bool) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;

    if !force {
        eprint!("Remove Syncara binary at {}? (y/N) ", current_exe.display());
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    // Remove PID file
    let pid_path = Path::new("syncara.pid");
    if pid_path.exists() {
        std::fs::remove_file(pid_path)?;
        eprintln!("✓  Removed syncara.pid");
    }

    // Remove binary
    match std::fs::remove_file(&current_exe) {
        Ok(()) => {
            eprintln!("✓  Removed {}", current_exe.display());
            eprintln!("Syncara has been uninstalled.");
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("⚠  Need sudo to remove {}:", current_exe.display());
            let status = std::process::Command::new("sudo")
                .args(["rm", &current_exe.to_string_lossy()])
                .status()
                .map_err(|_| anyhow::anyhow!("sudo failed"))?;
            if status.success() {
                eprintln!("✓  Removed {}", current_exe.display());
                eprintln!("Syncara has been uninstalled.");
            } else {
                anyhow::bail!("sudo rm failed");
            }
        }
        Err(e) => anyhow::bail!("failed to remove binary: {e}"),
    }

    Ok(())
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

fn print_banner(config: &syncara_core::config::Config) {
    let listener_count = config.listeners.len();
    let route_count = config.routes.len();
    let pool_count = config.pools.len();
    let upstream_count: usize = config.pools.iter().map(|p| p.upstreams.len()).sum();

    eprintln!("╭──────────────────────────────────────────────╮");
    eprintln!("│  Syncara v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("│  Listening on {listener_count} port(s)");
    for l in &config.listeners {
        let addr = format!("{}:{}", l.host, l.port);
        eprintln!("│    → {addr}");
    }
    eprintln!("│  {route_count} route(s), {pool_count} pool(s), {upstream_count} upstream(s)");
    let admin = &config.admin;
    eprintln!("│  Admin: {admin_host}:{admin_port}", admin_host = admin.host, admin_port = admin.port);
    eprintln!("╰──────────────────────────────────────────────╯");
    eprintln!();
}

fn write_pid_file() {
    let pid = std::process::id();
    if let Err(e) = std::fs::write("syncara.pid", pid.to_string()) {
        eprintln!("⚠  Could not write PID file: {e}");
    }
}

fn read_pid_file() -> anyhow::Result<u32> {
    let content = std::fs::read_to_string("syncara.pid")
        .map_err(|e| anyhow::anyhow!("cannot read syncara.pid: {e}"))?;
    let pid: u32 = content
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid PID in syncara.pid: {e}"))?;
    Ok(pid)
}

fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn tcp_probe(addr: &str, timeout: std::time::Duration) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    let ok = addr.to_socket_addrs().ok().and_then(|addrs| {
        for a in addrs {
            if TcpStream::connect_timeout(&a, timeout).is_ok() {
                return Some(true);
            }
        }
        None
    });
    ok.unwrap_or(false)
}

fn show_yaml_context(content: &str, line_num: usize) {
    let lines: Vec<&str> = content.lines().collect();
    let start = if line_num > 3 { line_num - 3 } else { 0 };
    let end = std::cmp::min(line_num + 2, lines.len());

    eprintln!("   Context:");
    for (i, line) in lines[start..end].iter().enumerate() {
        let actual_line = start + i + 1;
        let marker = if actual_line == line_num { "→" } else { " " };
        eprintln!("     {marker} {actual_line:>4} │ {line}");
    }
}

/// Probe total system memory in MB (best-effort, cross-platform).
fn probe_total_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024;
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("sysctl").arg("-n").arg("hw.memsize").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Ok(bytes) = s.trim().parse::<u64>() {
                    return bytes / 1024 / 1024;
                }
            }
        }
    }
    // Fallback: assume 4 GB
    4096
}

/// Probe the number of file descriptors that can be opened.
fn probe_file_limit() -> u64 {
    // Try `ulimit -n` via shell
    use std::process::Command;
    if let Ok(output) = Command::new("sh").args(["-c", "ulimit -n"]).output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(n) = s.trim().parse::<u64>() {
                return n;
            }
        }
    }
    // Fallback: 1024 (POSIX default)
    1024
}
