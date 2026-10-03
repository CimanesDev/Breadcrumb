#[cfg(windows)]
pub fn register() -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = format!("\"{}\" --inspect \"%1\"", exe.display());
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (menu, _) = root
        .create_subkey("Software\\Classes\\*\\shell\\Breadcrumb")
        .map_err(|e| e.to_string())?;
    menu.set_value("", &"Show file trail").map_err(|e| e.to_string())?;
    menu.set_value("Icon", &exe.to_string_lossy().as_ref()).map_err(|e| e.to_string())?;
    let (menu_command, _) = menu.create_subkey("command").map_err(|e| e.to_string())?;
    menu_command.set_value("", &command).map_err(|e| e.to_string())?;

    let (run, _) = root
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;
    run.set_value("Breadcrumb", &format!("\"{}\" --background", exe.display()))
        .map_err(|e| e.to_string())?;
    Ok(())
}
