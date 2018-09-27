use std::collections::HashMap;
use std::env::{self, current_exe, var};
use std::io::Error as IoError;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};

pub struct CliError {
    pub code: i32,
    pub message: String,
}

impl CliError {
    pub fn new(code: i32, message: String) -> Self {
        CliError {
            code: code,
            message: message,
        }
    }
}

pub type CliResult = Result<i32, CliError>;

pub type CliParseResults = Result<(), CliParseError>;

pub struct CliParseError {
    pub fatal: bool,
    pub message: String,
}

impl From<CliParseError> for CliError {
    fn from(err: CliParseError) -> CliError {
        CliError::new(101, err.message)
    }
}

pub fn execute_external_command(
    cmd: &PathBuf,
    args: &[String],
    extra_env: HashMap<String, String>,
) -> CliResult {
    let command_exe = format!("{:?}{}", cmd.to_str().unwrap(), env::consts::EXE_SUFFIX);

    return match run_command(command_exe, args, extra_env, false) {
        (_, _, Ok(code)) => Ok(code),
        (_, _, Err(err)) => Err(err),
    };
}

pub fn execute_external_command_for_output(
    cmd: &PathBuf,
    args: &[String],
    extra_env: HashMap<String, String>,
) -> Result<String, CliError> {
    let command_exe = format!("{}{}", cmd.to_str().unwrap(), env::consts::EXE_SUFFIX);

    return match run_command(command_exe, args, extra_env, true) {
        (stdout, _, Ok(_)) => Ok(stdout.trim().to_string()),
        (stdout, stderr, Err(err)) => {
            for line in stdout.lines() {
                error!("OUT: {}", line);
            }
            for line in stderr.lines() {
                error!("ERR: {}", line);
            }
            Err(err)
        }
    };
}

fn run_command(
    cmd: String,
    args: &[String],
    extra_env: HashMap<String, String>,
    capture_output: bool,
) -> (String, String, Result<i32, CliError>) {
    let mut command_string = String::new();
    command_string.push_str(cmd.as_str());
    for arg in args.iter() {
        command_string.push_str(" ");
        command_string.push_str(arg.as_str());
    }

    let (stdout, stderr) = if capture_output {
        (Stdio::piped(), Stdio::piped())
    } else {
        (Stdio::inherit(), Stdio::inherit())
    };

    let env_map = build_env_updates(extra_env);
    let child = match spawn_cmd(command_string, env_map, stdout, stderr) {
        Err(value) => {
            return (
                s!(""),
                s!(""),
                Err(CliError {
                    code: 10,
                    message: format!("Unable to execute command: {}", value),
                }),
            )
        }
        Ok(child) => child,
    };

    let result = wait_for_output(child);

    return match result {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            Ok(output.status.code().unwrap_or_else(|| 0)),
        ),
        Err(value) => (
            s!(""),
            s!(""),
            Err(CliError {
                code: 10,
                message: format!("Unable to run {:?} it returned {}", args, value),
            }),
        ),
    };
}

#[cfg(windows)]
fn wait_for_output(child: Child) -> Result<Output, std::io::Error> {
    return child.wait_with_output();
}

#[cfg(unix)]
fn wait_for_output(child: Child) -> Result<Output, std::io::Error> {
    use libc::{kill, SIGKILL, SIGTERM};
    use std::thread::sleep;
    use std::time::Duration;
    let child_id = child.id();

    let signal = unsafe {
        match signal_hook::register(signal_hook::SIGINT, move || {
            kill(child_id as i32, SIGTERM);
            sleep(Duration::from_millis(100));
            for i in (0..50).rev() {
                let is_dead: i32 = kill(child_id as i32, 0 as i32);
                if is_dead == 0 {
                    return;
                }
                if i % 10 == 0 && i >= 10 {
                    info!("Waiting {} more seconds for process to go away", i / 10);
                }

                sleep(Duration::from_millis(100));
            }
            warn!("Killing process!");
            kill(child_id as i32, SIGKILL);
        }) {
            Ok(sig) => sig,
            Err(_) => {
                panic!("Unable to register callback for control-c");
            }
        }
    };

    let result = child.wait_with_output();

    signal_hook::unregister(signal);

    return result;
}

#[cfg(windows)]
fn spawn_cmd<'a>(
    command: String,
    env: HashMap<String, String>,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<Child, IoError> {
    return Command::new("cmd")
        .arg("/C")
        .stdout(stdout)
        .stderr(stderr)
        .arg(command)
        .envs(&env)
        .spawn();
}

#[cfg(unix)]
fn spawn_cmd(
    command: String,
    env: HashMap<String, String>,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<Child, IoError> {
    return Command::new("sh")
        .arg("-c")
        .stdout(stdout)
        .stderr(stderr)
        .arg(command)
        .envs(&env)
        .spawn();
}

fn build_env_updates(extra_env: HashMap<String, String>) -> HashMap<String, String> {
    let mut results: HashMap<String, String> = HashMap::new();
    results.insert(String::from("PATH"), build_path());

    for (key, value) in env::vars() {
        results.insert(key, value);
    }

    for (key, value) in extra_env {
        results.insert(key, value);
    }

    debug!("Using ENV: {:?}", results);

    return results;
}

fn build_path() -> String {
    let path_extension = if let Ok(path) = current_exe() {
        let mut path = path.canonicalize().unwrap();
        path.pop();
        format!(":{}", path.as_os_str().to_str().unwrap())
    } else {
        String::new()
    };

    let path = var("PATH").unwrap();
    return format!("{}{}", path, path_extension);
}
