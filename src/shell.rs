use std::process::Stdio;
use std::io::{Write};
use std::env;
use clap::Arg;
use clap_nested::{Command, Commander, MultiCommand};

#[cfg(windows)]
pub fn run_command(shell: String, arguments: Vec<String>, command: String) -> std::result::Result<(), clap::Error> {
    // println!("arguments = {:?}", arguments);
    let mut child_process = std::process::Command::new(shell)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let child_stdin = child_process.stdin.as_mut().unwrap();
        child_stdin.write_all(&*command.into_bytes())?;
        // Close stdin to finish and avoid indefinite blocking
        drop(child_stdin);

    }
    let _output = child_process.wait_with_output()?;

    // println!("output = {:?}", output);

    Ok(())
}


#[cfg(unix)]
pub fn run_command(shell: String, arguments: Vec<String>, command: String) -> std::result::Result<(), clap::Error> {
    // Unix - pass command as parameter for initializer
    let mut arguments = arguments.clone();
    if !command.is_empty() {
        arguments.push(command);
    }

    //println!("arguments = {:?}", arguments);
    let mut child_process = std::process::Command::new(shell)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {

    }
    let output = child_process.wait_with_output()?;
    //println!("output = {:?}", output);
    Ok(())
}

pub fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
#[cfg(windows)]
pub fn set_env_variable(key:&str, value:String) {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (env, _) = hkcu.create_subkey("Environment").unwrap(); // create_subkey opens with write permissions
    env.set_value(key, &value).unwrap();
    // It's necessary to notify applications about update of the environment
    // https://stackoverflow.com/questions/19705401/how-to-set-system-environment-variable-in-c/19705691#19705691
    let param = wide_null("Environment").as_ptr() as winapi::shared::minwindef::LPARAM;
    unsafe {
        winapi::um::winuser::SendNotifyMessageW(
            winapi::um::winuser::HWND_BROADCAST,
            winapi::um::winuser::WM_SETTINGCHANGE,
            0,
            param
        );
    }
}

fn append_path(original_path: &str, new_path: &str) -> String {
    if original_path.len() == 0 {
        return new_path.to_string();
    }

    if original_path.contains(new_path) {
        return original_path.to_string();
    }

    if original_path.chars().last().unwrap() != ';' {
        return format!("{};{};", original_path, new_path);
    }

    format!("{}{};", original_path, new_path)
}

#[cfg(test)]
mod tests {
    use crate::shell::append_path;

    #[test]
    fn test_append_path() {
        assert_eq!(append_path("",""), "");
        assert_eq!(append_path("a",""), "a");
        assert_eq!(append_path("a","b"), "a;b;");
        assert_eq!(append_path("","b"), "b");
        assert_eq!(append_path("a;b;","b"), "a;b;");
        assert_eq!(append_path("a;c;","b"), "a;c;b;");
    }

}

#[cfg(windows)]
pub fn update_env_variable(variable_name: &str, value: &str) {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey("Environment").unwrap();
    let env_path:String = env.get_value(variable_name).unwrap();
    let updated_env_path = append_path(env_path.as_str(), value);
    set_env_variable(variable_name, updated_env_path);
}

#[cfg(windows)]
pub fn update_env_path(value: &str) {
    update_env_variable("PATH", value);
}

#[cfg(unix)]
pub fn update_env_variable(variable_name: &str, value: &str) {
    env::set_var(variable_name, value);
}

#[cfg(unix)]
pub fn update_env_path(value: &str) {
    let env_path:String = env::var("PATH").unwrap();
    let updated_env_path = append_path(env_path.as_str(), value);
    env::set_var("PATH", updated_env_path);
}

#[cfg(unix)]
pub fn set_env_variable(key:&str, value:&str) {

}


// pub fn get_cmd<'a>() -> Command<'a, str> {
//     Command::new("append")
//         .description("Append path to environment variable")
//         .options(|app| {
//             app.arg(
//                 Arg::with_name("variable")
//                     .short("v")
//                     .long("variable")
//                     .help("Name of environment variable")
//                     .takes_value(true),
//             )
//                 .arg(
//                     Arg::with_name("path")
//                         .short("p")
//                         .long("path")
//                         .help("Path which should be added to the variable")
//                         .takes_value(true),
//                 )
//         })
//         .runner(|_args, matches| {
//             let variable_name = matches.value_of("variable").unwrap().to_string();
//             let path_value = matches.value_of("path").unwrap().to_string();
//             update_env_variable(variable_name.as_str(), path_value.as_str());
//             Ok(())
//         })
// }

// pub fn get_multi_cmd<'a>() -> MultiCommand<'a, str, str> {
//     let multi_cmd: MultiCommand<str, str> = Commander::new()
//         .add_cmd(get_cmd())
//         .into_cmd("shell")

//         // Optionally specify a description
//         .description("Detection of Antivirus and handling exception registration.");

//     return multi_cmd;
// }
