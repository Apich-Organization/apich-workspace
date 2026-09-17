//! Command Security Guard & Sandbox Hardening Service.
//!
//! Validates terminal and sandbox commands prior to execution, blocking server-probing
//! commands (such as `df -H`, `lsblk`, `fdisk`), hardware discovery tools, kernel logs,
//! sensitive `/proc` and `/sys` inspections, and network probes to internal host addresses.

use std::path::Path;

/// Categorization of a security policy violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityViolation {
    /// Host storage inspection (e.g., `df`, `lsblk`, `fdisk`, `mount`).
    StorageInspection { command: String },
    /// Host hardware & CPU topology discovery (e.g., `lscpu`, `lshw`, `lspci`).
    HardwareDiscovery { command: String },
    /// Host system logs & kernel state (e.g., `dmesg`, `journalctl`, `sysctl`).
    SystemLogs { command: String },
    /// Host uptime & user session detection (e.g., `uptime`, `who`, `w`, `last`).
    UptimeOrUserDetection { command: String },
    /// System power & service control (e.g., `shutdown`, `reboot`, `systemctl`).
    SystemControl { command: String },
    /// Host network discovery & interface probing (e.g., `ip`, `ifconfig`, `netstat`, `route`).
    NetworkDiscovery { command: String },
    /// Direct reading of sensitive `/proc` or `/sys` host files.
    SensitiveHostPath { path: String },
    /// Reaching internal server / host network endpoints (e.g. `127.0.0.1`, `localhost`).
    InternalHostReach { target: String },
}

impl SecurityViolation {
    /// User-facing terminal message explaining the restriction.
    pub fn to_terminal_message(&self) -> String {
        match self {
            | Self::StorageInspection { command } => {
                if command == "df" {
                    format!(
                        "apich-terminal: command 'df' is disabled for security reasons: accessing real server storage or filesystem structure is restricted.\n(Note: project storage usage and quotas are safely managed under Project Settings)."
                    )
                } else {
                    format!(
                        "apich-terminal: command '{command}' is disabled for security reasons: accessing real server storage or filesystem structure is restricted."
                    )
                }
            },
            | Self::HardwareDiscovery { command } => {
                format!(
                    "apich-terminal: command '{command}' is disabled for security reasons: inspecting host hardware and device topology is restricted."
                )
            },
            | Self::SystemLogs { command } => {
                format!(
                    "apich-terminal: command '{command}' is disabled for security reasons: accessing host kernel logs and low-level system state is restricted."
                )
            },
            | Self::UptimeOrUserDetection { command } => {
                format!(
                    "apich-terminal: command '{command}' is disabled for security reasons: inspecting host server uptime and system sessions is restricted."
                )
            },
            | Self::SystemControl { command } => {
                format!(
                    "apich-terminal: command '{command}' is disabled for security reasons: server lifecycle and service management commands are not permitted."
                )
            },
            | Self::NetworkDiscovery { command } => {
                format!(
                    "apich-terminal: command '{command}' is disabled for security reasons: inspecting host network interfaces or topology is restricted."
                )
            },
            | Self::SensitiveHostPath { path } => {
                format!(
                    "apich-terminal: access to '{path}' is disabled for security reasons: reading host server system files is restricted."
                )
            },
            | Self::InternalHostReach { target } => {
                format!(
                    "apich-terminal: access to internal host target '{target}' is disabled for security reasons: reaching real server infrastructure is restricted."
                )
            },
        }
    }
}

pub struct CommandSecurityGuard;

impl CommandSecurityGuard {
    /// Validates a raw shell command line string.
    ///
    /// Returns `Ok(())` if the command is permitted, or `Err(SecurityViolation)`
    /// with the specific policy violation if restricted.
    pub fn validate_command(command: &str) -> Result<(), SecurityViolation> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        // 1. Check for command substitutions `$(...)` and backticks `` `...` ``
        Self::validate_subshells(trimmed)?;

        // 2. Split compound commands on shell chaining operators: ;, &&, ||, |, &, \n
        for segment in Self::split_compound_commands(trimmed) {
            let seg = segment.trim();
            if seg.is_empty() {
                continue;
            }
            Self::validate_segment(seg)?;
        }

        Ok(())
    }

    /// Check command substitutions $(...) and `...`
    fn validate_subshells(command: &str) -> Result<(), SecurityViolation> {
        // Look for $(...)
        let mut rest = command;
        while let Some(start) = rest.find("$(") {
            let after_start = &rest[start + 2..];
            if let Some(end) = after_start.find(')') {
                let inner = &after_start[..end];
                Self::validate_command(inner)?;
                rest = &after_start[end + 1..];
            } else {
                break;
            }
        }

        // Look for `...`
        let mut rest_bt = command;
        while let Some(start) = rest_bt.find('`') {
            let after_start = &rest_bt[start + 1..];
            if let Some(end) = after_start.find('`') {
                let inner = &after_start[..end];
                Self::validate_command(inner)?;
                rest_bt = &after_start[end + 1..];
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Splits a command string across shell statement separators while respecting quotes.
    fn split_compound_commands(input: &str) -> Vec<String> {
        let mut segments = Vec::new();
        let mut cur = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                | '\'' if !in_double => {
                    in_single = !in_single;
                    cur.push(c);
                },
                | '"' if !in_single => {
                    in_double = !in_double;
                    cur.push(c);
                },
                | ';' | '\n' if !in_single && !in_double => {
                    if !cur.trim().is_empty() {
                        segments.push(cur.trim().to_string());
                    }
                    cur.clear();
                },
                | '&' if !in_single && !in_double => {
                    if chars.peek() == Some(&'&') {
                        chars.next(); // consume second &
                    }
                    if !cur.trim().is_empty() {
                        segments.push(cur.trim().to_string());
                    }
                    cur.clear();
                },
                | '|' if !in_single && !in_double => {
                    if chars.peek() == Some(&'|') {
                        chars.next(); // consume second |
                    }
                    if !cur.trim().is_empty() {
                        segments.push(cur.trim().to_string());
                    }
                    cur.clear();
                },
                | _ => {
                    cur.push(c);
                },
            }
        }

        if !cur.trim().is_empty() {
            segments.push(cur.trim().to_string());
        }

        segments
    }

    /// Tokenizes a single command segment into shell arguments, respecting quotes.
    fn tokenize_segment(segment: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cur = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut escaped = false;

        for c in segment.chars() {
            if escaped {
                cur.push(c);
                escaped = false;
                continue;
            }

            match c {
                | '\\' if !in_single => {
                    escaped = true;
                },
                | '\'' if !in_double => {
                    in_single = !in_single;
                },
                | '"' if !in_single => {
                    in_double = !in_double;
                },
                | ' ' | '\t' if !in_single && !in_double => {
                    if !cur.is_empty() {
                        tokens.push(cur.clone());
                        cur.clear();
                    }
                },
                | _ => {
                    cur.push(c);
                },
            }
        }

        if !cur.is_empty() {
            tokens.push(cur);
        }

        tokens
    }

    /// Validates an individual command segment and its arguments.
    fn validate_segment(segment: &str) -> Result<(), SecurityViolation> {
        let tokens = Self::tokenize_segment(segment);
        if tokens.is_empty() {
            return Ok(());
        }

        // Skip leading environment variable assignments (e.g. `FOO=bar cmd`)
        let mut idx = 0;
        while idx < tokens.len() {
            let t = &tokens[idx];
            if Self::is_env_assignment(t) {
                idx += 1;
            } else {
                break;
            }
        }

        if idx >= tokens.len() {
            return Ok(());
        }

        // Skip common command wrappers (e.g. `sudo`, `doas`, `nohup`, `time`, `exec`, `env`)
        while idx < tokens.len() {
            let raw = &tokens[idx];
            let base = Self::clean_command_base(raw);

            if matches!(
                base.as_str(),
                "sudo" | "doas" | "nohup" | "time" | "exec" | "command" | "builtin" | "xargs" | "watch"
            ) {
                idx += 1;
                // Skip any immediate flags of the wrapper (like `sudo -u root`)
                while idx < tokens.len() && tokens[idx].starts_with('-') {
                    if (tokens[idx] == "-u" || tokens[idx] == "-g") && idx + 1 < tokens.len() {
                        idx += 2;
                    } else {
                        idx += 1;
                    }
                }
            } else if base == "env" {
                idx += 1;
                while idx < tokens.len() && (tokens[idx].starts_with('-') || Self::is_env_assignment(&tokens[idx])) {
                    idx += 1;
                }
            } else {
                break;
            }
        }

        if idx >= tokens.len() {
            return Ok(());
        }

        let cmd_token = &tokens[idx];
        let cmd_base = Self::clean_command_base(cmd_token);

        // Check if command is a subshell runner like `sh -c "..."` or `bash -c "..."`
        if matches!(cmd_base.as_str(), "sh" | "bash" | "dash" | "zsh" | "ksh" | "busybox") {
            let mut arg_idx = idx + 1;
            while arg_idx < tokens.len() {
                if tokens[arg_idx] == "-c" && arg_idx + 1 < tokens.len() {
                    let sub_cmd = &tokens[arg_idx + 1];
                    Self::validate_command(sub_cmd)?;
                    break;
                }
                arg_idx += 1;
            }
        }

        // Check for language inline eval runners e.g. python3 -c "...", perl -e "...", node -e "..."
        if matches!(cmd_base.as_str(), "python" | "python3" | "python3.11" | "python3.12" | "python3.13" | "node" | "perl" | "ruby") {
            let mut arg_idx = idx + 1;
            while arg_idx < tokens.len() {
                if (tokens[arg_idx] == "-c" || tokens[arg_idx] == "-e") && arg_idx + 1 < tokens.len() {
                    let inline_code = &tokens[arg_idx + 1];
                    Self::validate_inline_code(inline_code)?;
                    break;
                }
                arg_idx += 1;
            }
        }

        // 1. Check prohibited command registry
        Self::check_command_name(&cmd_base)?;

        // 2. Check all arguments for sensitive `/proc`, `/sys`, or host system paths
        for arg in &tokens[idx..] {
            Self::check_argument_path(arg)?;
        }

        // 3. If network client, check for targets reaching internal host services
        if matches!(
            cmd_base.as_str(),
            "curl" | "wget" | "nc" | "netcat" | "socat" | "telnet" | "ping" | "ping6" | "ftp" | "ssh" | "ncat"
        ) {
            for arg in &tokens[idx + 1..] {
                Self::check_network_target(arg)?;
            }
        }

        Ok(())
    }

    /// Checks if a string looks like a shell environment variable assignment (e.g. `VAR=value`).
    fn is_env_assignment(token: &str) -> bool {
        if let Some((k, _)) = token.split_once('=') {
            !k.is_empty()
                && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !k.starts_with(|c: char| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Normalizes a command token by stripping quotes, backslashes, and directory paths.
    fn clean_command_base(token: &str) -> String {
        let unquoted = token.trim_matches(|c| c == '"' || c == '\'' || c == '\\');
        Path::new(unquoted)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(unquoted)
            .to_lowercase()
    }

    /// Check command base against prohibited categories.
    fn check_command_name(cmd: &str) -> Result<(), SecurityViolation> {
        // Storage & Disks
        if matches!(
            cmd,
            "df" | "lsblk"
                | "fdisk"
                | "cfdisk"
                | "sfdisk"
                | "parted"
                | "gdisk"
                | "findmnt"
                | "mount"
                | "umount"
                | "tune2fs"
                | "dumpe2fs"
                | "fsck"
                | "e2fsck"
                | "btrfs"
                | "zfs"
                | "vgdisplay"
                | "lvdisplay"
                | "pvdisplay"
                | "hdparm"
                | "smartctl"
                | "nvme"
        ) {
            return Err(SecurityViolation::StorageInspection {
                command: cmd.to_string(),
            });
        }

        // Hardware & CPU Topology
        if matches!(
            cmd,
            "lscpu"
                | "lshw"
                | "lspci"
                | "lsusb"
                | "dmidecode"
                | "inxi"
                | "hwinfo"
                | "sensors"
        ) {
            return Err(SecurityViolation::HardwareDiscovery {
                command: cmd.to_string(),
            });
        }

        // Kernel Logs & Low-Level State
        if matches!(
            cmd,
            "dmesg" | "journalctl" | "sysctl" | "kexec" | "modprobe" | "insmod" | "rmmod" | "lsmod"
        ) {
            return Err(SecurityViolation::SystemLogs {
                command: cmd.to_string(),
            });
        }

        // Host Uptime & Multi-User Detection
        if matches!(cmd, "uptime" | "w" | "who" | "last" | "lastlog" | "users" | "finger") {
            return Err(SecurityViolation::UptimeOrUserDetection {
                command: cmd.to_string(),
            });
        }

        // System Control & Lifecycle
        if matches!(
            cmd,
            "shutdown"
                | "reboot"
                | "poweroff"
                | "halt"
                | "init"
                | "telinit"
                | "systemctl"
                | "service"
        ) {
            return Err(SecurityViolation::SystemControl {
                command: cmd.to_string(),
            });
        }

        // Network Discovery & Interfaces
        if matches!(
            cmd,
            "ip" | "ifconfig"
                | "route"
                | "netstat"
                | "ss"
                | "arp"
                | "mii-tool"
                | "ethtool"
                | "nmap"
                | "traceroute"
                | "tracepath"
                | "mtr"
                | "iptables"
                | "nft"
                | "ufw"
                | "firewall-cmd"
                | "ebtables"
                | "brctl"
        ) {
            return Err(SecurityViolation::NetworkDiscovery {
                command: cmd.to_string(),
            });
        }

        Ok(())
    }

    /// Check argument strings for prohibited host file paths.
    fn check_argument_path(arg: &str) -> Result<(), SecurityViolation> {
        let lower = arg.to_lowercase();

        let prohibited_patterns = [
            "/proc/partitions",
            "/proc/mounts",
            "/proc/diskstats",
            "/proc/cpuinfo",
            "/proc/meminfo",
            "/proc/version",
            "/proc/cmdline",
            "/proc/kallsyms",
            "/proc/modules",
            "/sys/block",
            "/sys/devices",
            "/sys/class/net",
            "/etc/shadow",
            "/etc/sudoers",
        ];

        for pattern in prohibited_patterns {
            if lower.contains(pattern) {
                return Err(SecurityViolation::SensitiveHostPath {
                    path: pattern.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check network client tool arguments for host loopback or internal gateway addresses.
    fn check_network_target(arg: &str) -> Result<(), SecurityViolation> {
        let lower = arg.to_lowercase();

        let prohibited_endpoints = [
            "localhost",
            "127.0.0.1",
            "0.0.0.0",
            "::1",
            "[::1]",
            "host.containers.internal",
            "host.docker.internal",
            "172.17.0.1",
            "10.0.2.2",
            "169.254.169.254",
        ];

        for endpoint in prohibited_endpoints {
            if lower.contains(endpoint) {
                return Err(SecurityViolation::InternalHostReach {
                    target: endpoint.to_string(),
                });
            }
        }

        // Check for 127.x.x.x addresses
        if lower.contains("127.") {
            return Err(SecurityViolation::InternalHostReach {
                target: "127.0.0.0/8".to_string(),
            });
        }

        Ok(())
    }

    /// Check inline script codes (e.g. python -c "...") for embedded prohibited commands.
    fn validate_inline_code(code: &str) -> Result<(), SecurityViolation> {
        let lower = code.to_lowercase();

        // Check for embedded calls to prohibited storage & host commands
        let dangerous_calls = [
            "df -", "df ", "lsblk", "fdisk", "/proc/partitions", "/proc/mounts",
            "/proc/cpuinfo", "127.0.0.1", "localhost", "host.containers.internal"
        ];

        for call in dangerous_calls {
            if lower.contains(call) {
                if call.starts_with("df") || call == "lsblk" || call == "fdisk" {
                    return Err(SecurityViolation::StorageInspection {
                        command: call.trim().to_string(),
                    });
                } else if call.starts_with("/proc") {
                    return Err(SecurityViolation::SensitiveHostPath {
                        path: call.to_string(),
                    });
                } else {
                    return Err(SecurityViolation::InternalHostReach {
                        target: call.to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_df_commands_blocked() {
        assert!(matches!(
            CommandSecurityGuard::validate_command("df -H"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("df -h"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("df"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("/bin/df -H"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("sudo df -H"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("sudo -u root df -h"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("LC_ALL=C df -H"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
    }

    #[test]
    fn test_other_storage_commands_blocked() {
        assert!(matches!(
            CommandSecurityGuard::validate_command("lsblk"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("fdisk -l"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("mount /dev/sda1 /mnt"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("findmnt"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
    }

    #[test]
    fn test_hardware_and_kernel_blocked() {
        assert!(matches!(
            CommandSecurityGuard::validate_command("lscpu"),
            Err(SecurityViolation::HardwareDiscovery { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("lspci"),
            Err(SecurityViolation::HardwareDiscovery { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("dmesg"),
            Err(SecurityViolation::SystemLogs { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("uptime"),
            Err(SecurityViolation::UptimeOrUserDetection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("reboot"),
            Err(SecurityViolation::SystemControl { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("systemctl restart nginx"),
            Err(SecurityViolation::SystemControl { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("ip a"),
            Err(SecurityViolation::NetworkDiscovery { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("ifconfig"),
            Err(SecurityViolation::NetworkDiscovery { .. })
        ));
    }

    #[test]
    fn test_chained_and_subshell_blocked() {
        assert!(matches!(
            CommandSecurityGuard::validate_command("echo hello && df -H"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("echo hello; df -h"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("df -h | grep /"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("echo $(df -H)"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("echo `df -H`"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("sh -c 'df -H'"),
            Err(SecurityViolation::StorageInspection { .. })
        ));
    }

    #[test]
    fn test_sensitive_proc_and_sys_blocked() {
        assert!(matches!(
            CommandSecurityGuard::validate_command("cat /proc/partitions"),
            Err(SecurityViolation::SensitiveHostPath { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("head -n 20 /proc/mounts"),
            Err(SecurityViolation::SensitiveHostPath { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("grep cpu /proc/cpuinfo"),
            Err(SecurityViolation::SensitiveHostPath { .. })
        ));
    }

    #[test]
    fn test_internal_network_targets_blocked() {
        assert!(matches!(
            CommandSecurityGuard::validate_command("curl http://127.0.0.1:3000"),
            Err(SecurityViolation::InternalHostReach { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("wget http://localhost:5432"),
            Err(SecurityViolation::InternalHostReach { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("nc -zv 127.0.0.1 3000"),
            Err(SecurityViolation::InternalHostReach { .. })
        ));
        assert!(matches!(
            CommandSecurityGuard::validate_command("ping 172.17.0.1"),
            Err(SecurityViolation::InternalHostReach { .. })
        ));
    }

    #[test]
    fn test_allowed_developer_commands() {
        assert!(CommandSecurityGuard::validate_command("cargo build --release").is_ok());
        assert!(CommandSecurityGuard::validate_command("git status").is_ok());
        assert!(CommandSecurityGuard::validate_command("git commit -m 'Initial commit'").is_ok());
        assert!(CommandSecurityGuard::validate_command("python3 analysis.py").is_ok());
        assert!(CommandSecurityGuard::validate_command("Rscript script.R").is_ok());
        assert!(CommandSecurityGuard::validate_command("typst compile report.typ").is_ok());
        assert!(CommandSecurityGuard::validate_command("pdflatex document.tex").is_ok());
        assert!(CommandSecurityGuard::validate_command("ls -la").is_ok());
        assert!(CommandSecurityGuard::validate_command("cat README.md | grep title").is_ok());
        assert!(CommandSecurityGuard::validate_command("mkdir -p src && touch src/main.rs").is_ok());
        assert!(CommandSecurityGuard::validate_command("curl https://crates.io").is_ok());
    }
}
