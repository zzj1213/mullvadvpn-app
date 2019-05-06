#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]

use super::{TunnelEvent, TunnelMetadata};
use crate::process::tinc::TincOperator;
use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        mpsc,
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use std::str::FromStr;
use talpid_types::net::tinc;

use tinc_plugin;

#[cfg(target_os = "linux")]
use which;
use std::net::{IpAddr, Ipv4Addr};

#[cfg(not(windows))]
const DEFAULT_TINC_HOME: &str = "/root/tinc/";

#[cfg(windows)]
const DEFAULT_TINC_HOME: &str = "C:/tinc/";

/// Results from fallible operations on the Tinc tunnel.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can happen when using the Tinc tunnel.
#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Unable to start
    #[error(display = "Start Tinc Error")]
    StartTincError,

    /// Unable to start, wait for or kill the Tinc process.
    #[error(display = "Error in Tinc process management: {}", _0)]
    ChildProcessError(&'static str, #[error(cause)] io::Error),

    /// No TAP adapter was detected
    #[cfg(windows)]
    #[error(display = "No TAP adapter was detected")]
    MissingTapAdapter,

    /// TAP adapter seems to be disabled
    #[cfg(windows)]
    #[error(display = "The TAP adapter appears to be disabled")]
    DisabledTapAdapter,

    /// Tinc process died unexpectedly
    #[error(display = "Tinc process died unexpectedly")]
    ChildProcessDied,

    /// The IP routing program was not found.
    #[cfg(target_os = "linux")]
    #[error(display = "The IP routing program `ip` was not found")]
    IpRouteNotFound(#[error(cause)] failure::Compat<which::Error>),

    /// The Tinc binary was not found.
    #[error(display = "No Tinc binary found at {}", _0)]
    TincNotFound(String),

    /// Error while writing credentials to temporary file.
    #[error(display = "Error while writing credentials to temporary file")]
    CredentialsWriteError(#[error(cause)] io::Error),

    /// Failure in Windows syscall.
    #[cfg(windows)]
    #[error(display = "Failure in Windows syscall")]
    WinnetError(#[error(cause)] crate::winnet::Error),


    /// process::tinc::Error
    #[error(display = "Tinc Operator Error")]
    TincOperatorError(#[error(cause)] crate::process::tinc::Error),
}


#[cfg(unix)]
static TINC_DIE_TIMEOUT: Duration = Duration::from_secs(4);
#[cfg(windows)]
static TINC_DIE_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(unix)]
const TINC_BIN_FILENAME: &str = "tincd";
#[cfg(windows)]
const TINC_BIN_FILENAME: &str = "tincd.exe";

/// Struct for monitoring an Tinc process.
pub struct TincMonitor {
    tinc:               TincOperator,
    on_event:           Box<dyn Fn(TunnelEvent) + Send + Sync + 'static>,
    log_path:           Option<PathBuf>,
    event_rx:           mpsc::Receiver<tinc_plugin::EventType>,
    child:              Arc<duct::Handle>,
    closed:             Arc<AtomicBool>,
}

impl TincMonitor {
    /// Creates a new `TincMonitor` with the given listener and using the plugin at the given
    /// path.
    pub fn start<L>(
        on_event:       L,
        params:         &tinc::TunnelParameters,
        log_file:       Option<PathBuf>,
        resource_dir:   &Path,
    ) -> Result<Self>
        where
            L: Fn(TunnelEvent) + Send + Sync + 'static,
    {
        let resource_dir_str = resource_dir.to_str().unwrap();
        // TODO 统一到mullvad path系统
        // 现在resource_dir 为debug目录, 不方便tinc调试 修改到/root/tinc
//        let mut tinc_operator = TincOperator::new(resource_dir_str.to_string());
        let mut tinc_operator = TincOperator::new(DEFAULT_TINC_HOME.to_string());
        let child = tinc_operator.start_tinc().map_err(|_|Error::StartTincError)?;

        let event_rx = tinc_plugin::spawn();

        let on_event = Box::new(on_event);
        return Ok(TincMonitor {
            tinc: tinc_operator,
            on_event,
            log_path: log_file,
            event_rx,
            child: Arc::new(child),
            closed: Arc::new(AtomicBool::new(false)),
        });
    }

    fn tunnel_up(&self) -> Result<()> {
        let vip_str = self.tinc.get_vip().map_err(Error::TincOperatorError)?;
        let vip = Ipv4Addr::from_str(&vip_str).unwrap();
        let ips = vec![IpAddr::from(vip.clone())];

        let interface_name = "tun0";
        let metadata = TunnelMetadata {
            interface: interface_name.to_string(),
            ips,
            ipv4_gateway: vip,
            ipv6_gateway: None
        };
        (self.on_event)(TunnelEvent::Up(metadata));
        Ok(())
    }

    /// Consumes the monitor and waits for both proxy and tunnel, as applicable.
    pub fn wait(mut self) -> Result<()> {
        let wait_result = match self.event_rx.recv() {
            Ok(tinc_plugin::EventType::Up) => {
                self.tunnel_up()?;
                return Ok(());
            },
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        };
        (self.on_event)(TunnelEvent::Down);
        wait_result
    }

    /// Creates a handle to this monitor, allowing the tunnel to be closed while some other
    /// thread is blocked in `wait`.
    pub fn close_handle(&self) -> TincCloseHandle {
        TincCloseHandle {
            child: self.child.clone(),
            closed: self.closed.clone(),
        }
    }
}

/// 用于关闭tinc进程
pub struct TincCloseHandle {
    child:              Arc<duct::Handle>,
    closed:             Arc<AtomicBool>,
}

impl TincCloseHandle {
    /// Kills the underlying Tinc process,
    /// making the `TincMonitor::wait` method return.
    pub fn close(self) -> io::Result<()> {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.child.kill()
        } else {
            Ok(())
        }
    }
}