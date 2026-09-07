fn main() -> Result<(), Box<dyn std::error::Error>> {
    let windows = tauri_build::WindowsAttributes::new()
        .window_icon_path("icons/icon.ico")
        .app_manifest(
            r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#,
        );
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))?;
    Ok(())
}
