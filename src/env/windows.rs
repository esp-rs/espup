use log::warn;
use std::env;
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

#[cfg(windows)]
/// Sets an environment variable for the current user.
pub fn set_environment_variable(key: &str, value: &str) -> Result<(), Error> {
    env::set_var(key, value);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_WRITE)?;
    environment_key.set_value(key, &value)?;
    Ok(())
}

#[cfg(windows)]
/// Deletes an environment variable for the current user.
pub fn delete_environment_variable(key: &str) -> Result<(), Error> {
    if env::var_os(key).is_none() {
        return Ok(());
    }

    env::remove_var(key);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    environment_key.delete_value(key)?;
    Ok(())
}
