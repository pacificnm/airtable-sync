//! In-process CLI dispatch for the GUI host.

use std::ffi::OsString;
use std::path::PathBuf;

use nest_error::NestError;

use crate::cli_app;

/// Dispatches CLI subcommands from the GUI without duplicating business logic.
#[derive(Clone, Debug)]
pub struct CommandDispatch {
    config_path: Option<PathBuf>,
}

/// Outcome of a single dispatched command.
#[derive(Debug)]
pub struct DispatchResult {
    /// Whether `try_run_with` returned `Ok(())`.
    pub success: bool,
    /// Captured standard output (Unix only; empty on other platforms).
    pub stdout: String,
    /// Structured error when the command failed.
    pub error: Option<NestError>,
}

impl CommandDispatch {
    /// Creates a dispatcher that passes `--config` when `config_path` is set.
    pub fn new(config_path: Option<PathBuf>) -> Self {
        Self { config_path }
    }

    /// Runs `airtable-sync` with the given subcommand args (e.g. `["report", "summary"]`).
    ///
    /// When `json` is true, `--json` is inserted after global flags.
    pub fn run(&self, subcommand_args: &[&str], json: bool) -> DispatchResult {
        let mut args: Vec<OsString> = Vec::new();
        args.push(OsString::from("airtable-sync"));

        if let Some(path) = &self.config_path {
            args.push(OsString::from("--config"));
            args.push(path.as_os_str().to_os_string());
        }

        if json {
            args.push(OsString::from("--json"));
        }

        for arg in subcommand_args {
            args.push(OsString::from(*arg));
        }

        let (result, stdout) = capture_stdout(|| cli_app().try_run_with(args));

        match result {
            Ok(()) => DispatchResult {
                success: true,
                stdout,
                error: None,
            },
            Err(error) => DispatchResult {
                success: false,
                stdout,
                error: Some(error),
            },
        }
    }
}

fn capture_stdout<F>(f: F) -> (nest_error::NestResult<()>, String)
where
    F: FnOnce() -> nest_error::NestResult<()>,
{
    #[cfg(unix)]
    {
        capture_stdout_unix(f)
    }
    #[cfg(not(unix))]
    {
        (f(), String::new())
    }
}

#[cfg(unix)]
fn capture_stdout_unix<F>(f: F) -> (nest_error::NestResult<()>, String)
where
    F: FnOnce() -> nest_error::NestResult<()>,
{
    use std::ffi::c_int;
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::io::{FromRawFd, IntoRawFd};

    extern "C" {
        fn pipe(fds: *mut c_int) -> c_int;
        fn dup(oldfd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
    }

    let mut fds: [c_int; 2] = [0, 0];
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
        return (f(), String::new());
    }

    let stdout_fd = unsafe { dup(1) };
    if stdout_fd < 0 {
        unsafe {
            close(fds[0]);
            close(fds[1]);
        }
        return (f(), String::new());
    }

    if unsafe { dup2(fds[1], 1) } < 0 {
        unsafe {
            close(fds[0]);
            close(fds[1]);
            close(stdout_fd);
        }
        return (f(), String::new());
    }
    unsafe {
        close(fds[1]);
    }

    let result = f();

    unsafe {
        dup2(stdout_fd, 1);
        close(stdout_fd);
    }

    let mut read_file = unsafe { File::from_raw_fd(fds[0]) };
    let mut output = String::new();
    let _ = read_file.read_to_string(&mut output);
    let _ = read_file.into_raw_fd();

    (result, output)
}

