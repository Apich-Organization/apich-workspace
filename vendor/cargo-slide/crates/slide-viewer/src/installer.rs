//! System installer and cross-platform native file picker for `slide-viewer`.

use std::path::PathBuf;

/// Install `slide-viewer` binary into the user/system PATH and register `.slide` file associations.
pub fn install_viewer_to_system() -> Result<String, Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("Could not determine HOME directory")?;

        // 1. Install binary to ~/.local/bin/slide-viewer
        let bin_dir = home.join(".local/bin");
        std::fs::create_dir_all(&bin_dir)?;
        let target_exe = bin_dir.join("slide-viewer");
        std::fs::copy(&current_exe, &target_exe)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target_exe, std::fs::Permissions::from_mode(0o755));
        }

        // 2. Install desktop entry ~/.local/share/applications/slide-viewer.desktop
        let apps_dir = home.join(".local/share/applications");
        std::fs::create_dir_all(&apps_dir)?;
        let desktop_file = apps_dir.join("slide-viewer.desktop");
        let desktop_content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Slide Viewer\n\
             GenericName=Presentation Player\n\
             Comment=Universal presentation player for Cargo Slide (.slide)\n\
             Exec={} %f\n\
             Icon=x-office-presentation\n\
             Terminal=false\n\
             MimeType=application/x-slide;\n\
             Categories=Office;Presentation;Viewer;Graphics;\n\
             StartupNotify=true\n",
            target_exe.display()
        );
        std::fs::write(&desktop_file, desktop_content)?;

        // 3. Register MIME type ~/.local/share/mime/packages/cargo-slide.xml
        let mime_dir = home.join(".local/share/mime/packages");
        std::fs::create_dir_all(&mime_dir)?;
        let mime_file = mime_dir.join("cargo-slide.xml");
        let mime_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-slide">
    <comment>Cargo Slide Presentation Package</comment>
    <glob pattern="*.slide"/>
    <icon name="x-office-presentation"/>
  </mime-type>
</mime-info>
"#;
        std::fs::write(&mime_file, mime_content)?;

        // 4. Update desktop & mime databases if tools exist
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&apps_dir)
            .output();
        let _ = std::process::Command::new("update-mime-database")
            .arg(home.join(".local/share/mime"))
            .output();

        // 5. Append default association to ~/.config/mimeapps.list if present
        let config_dir = home.join(".config");
        let mimeapps_path = config_dir.join("mimeapps.list");
        let maybe_content = std::fs::read_to_string(&mimeapps_path).ok();
        if let Some(mut content) = maybe_content.filter(|c| !c.contains("application/x-slide")) {
            if let Some(pos) = content.find("[Default Applications]") {
                let insert_pos = pos + "[Default Applications]\n".len();
                content.insert_str(insert_pos, "application/x-slide=slide-viewer.desktop\n");
                let _ = std::fs::write(&mimeapps_path, content);
            } else {
                content.push_str(
                    "\n[Default Applications]\napplication/x-slide=slide-viewer.desktop\n",
                );
                let _ = std::fs::write(&mimeapps_path, content);
            }
        }

        Ok(format!(
            "Installed slide-viewer to {}",
            target_exe.display()
        ))
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("Could not determine HOME directory")?;

        // 1. Install CLI binary to ~/.local/bin/slide-viewer
        let bin_dir = home.join(".local/bin");
        std::fs::create_dir_all(&bin_dir)?;
        let target_exe = bin_dir.join("slide-viewer");
        std::fs::copy(&current_exe, &target_exe)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target_exe, std::fs::Permissions::from_mode(0o755));
        }

        // 2. Create macOS App Bundle: ~/Applications/Slide Viewer.app
        let app_dir = home.join("Applications").join("Slide Viewer.app");
        let macos_dir = app_dir.join("Contents").join("MacOS");
        let res_dir = app_dir.join("Contents").join("Resources");
        std::fs::create_dir_all(&macos_dir)?;
        std::fs::create_dir_all(&res_dir)?;

        let app_exe = macos_dir.join("slide-viewer");
        std::fs::copy(&current_exe, &app_exe)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&app_exe, std::fs::Permissions::from_mode(0o755));
        }

        let plist_path = app_dir.join("Contents").join("Info.plist");
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>slide-viewer</string>
    <key>CFBundleIdentifier</key>
    <string>org.cargo-slide.viewer</string>
    <key>CFBundleName</key>
    <string>Slide Viewer</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.3</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Cargo Slide Presentation</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>slide</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
"#;
        std::fs::write(&plist_path, plist_content)?;

        Ok(format!(
            "Installed Slide Viewer.app to {}",
            app_dir.display()
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .ok_or("Could not determine user appdata directory")?;

        let install_dir = local_app_data.join("Programs").join("cargo-slide");
        std::fs::create_dir_all(&install_dir)?;
        let target_exe = install_dir.join("slide-viewer.exe");
        std::fs::copy(&current_exe, &target_exe)?;

        let exe_str = target_exe.to_string_lossy();
        let cmd_str = format!("\"{}\" \"%1\"", exe_str);

        // Register .slide association in HKCU
        let _ = std::process::Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Classes\.slide",
                "/ve",
                "/d",
                "CargoSlide.Package",
                "/f",
            ])
            .output();
        let _ = std::process::Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Classes\CargoSlide.Package",
                "/ve",
                "/d",
                "Cargo Slide Presentation",
                "/f",
            ])
            .output();
        let _ = std::process::Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Classes\CargoSlide.Package\shell\open\command",
                "/ve",
                "/d",
                &cmd_str,
                "/f",
            ])
            .output();
        let _ = std::process::Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Classes\Applications\slide-viewer.exe\shell\open\command",
                "/ve",
                "/d",
                &cmd_str,
                "/f",
            ])
            .output();
        let _ = std::process::Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\slide-viewer.exe",
                "/ve",
                "/d",
                &format!("\"{}\"", exe_str),
                "/f",
            ])
            .output();

        Ok(format!(
            "Installed slide-viewer.exe to {}",
            target_exe.display()
        ))
    }
}

/// Prompt the user with a native file chooser dialog to pick a `.slide`, `.typ`, or `.json` presentation file.
pub fn pick_presentation_file() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let zenity_res = std::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--title=Select Slide Presentation (.slide, .typ, .json)",
                "--file-filter=Presentations (*.slide, *.typ, *.json) | *.slide *.typ *.json",
                "--file-filter=All Files | *",
            ])
            .output();

        if let Some(output) = zenity_res.ok().filter(|o| o.status.success()) {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let p = PathBuf::from(path_str);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        let kdialog_res = std::process::Command::new("kdialog")
            .args([
                "--getopenfilename",
                ".",
                "*.slide *.typ *.json|Slide Presentations",
            ])
            .output();

        if let Some(output) = kdialog_res.ok().filter(|o| o.status.success()) {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let p = PathBuf::from(path_str);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let script = r#"POSIX path of (choose file of type {"slide", "typ", "json", "public.item"} with prompt "Select Slide Presentation")"#;
        if let Ok(output) = std::process::Command::new("osascript")
            .args(["-e", script])
            .output()
        {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let p = PathBuf::from(path_str);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$f = New-Object System.Windows.Forms.OpenFileDialog
$f.Title = 'Select Slide Presentation'
$f.Filter = 'Slide Presentations (*.slide;*.typ;*.json)|*.slide;*.typ;*.json|All files (*.*)|*.*'
if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    Write-Output $f.FileName
}
"#;
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let p = PathBuf::from(path_str);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}
