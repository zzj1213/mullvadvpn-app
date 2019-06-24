use std::{
    io,
    net::IpAddr,
    thread,
    time::{Duration, Instant},
};

#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "Failed to run ping command")]
    PingError(#[error(cause)] io::Error),

    #[error(display = "Ping timed out")]
    TimeoutError,
}

pub fn monitor_ping(ip: IpAddr, timeout_secs: u16, interface: &str) -> Result<(), Error> {
    loop {
        let start = Instant::now();
        ping(ip, timeout_secs, &interface, false)?;
        if let Some(remaining) =
            Duration::from_secs(timeout_secs.into()).checked_sub(start.elapsed())
        {
            thread::sleep(remaining);
        }
    }
}

pub fn ping(
    ip: IpAddr,
    timeout_secs: u16,
    interface: &str,
    exit_on_first_reply: bool,
) -> Result<(), Error> {
    let output = ping_cmd(ip, timeout_secs, interface, exit_on_first_reply)
        .run()
        .map_err(Error::PingError)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::TimeoutError)
    }
}

fn ping_cmd(
    ip: IpAddr,
    timeout_secs: u16,
    interface: &str,
    exit_on_first_reply: bool,
) -> duct::Expression {
    let mut args = vec!["-n", "-i", "1"];

    let timeout_flag = if cfg!(target_os = "linux") || cfg!(target_os = "android") {
        "-w"
    } else {
        "-t"
    };
    let timeout_secs = timeout_secs.to_string();

    args.extend_from_slice(&[timeout_flag, &timeout_secs]);

    let interface_flag = if cfg!(target_os = "linux") {
        Some("-I")
    } else if cfg!(target_os = "macos") {
        Some("-b")
    } else {
        None
    };

    if let Some(interface_flag) = interface_flag {
        args.extend_from_slice(&[interface_flag, interface]);
    }

    if exit_on_first_reply {
        if cfg!(target_os = "macos") {
            args.push("-o");
        } else {
            args.extend_from_slice(&["-c", "1"])
        }
    }

    let ip = ip.to_string();
    args.push(&ip);

    duct::cmd("ping", args)
        .stdin_null()
        .stdout_null()
        .unchecked()
}
