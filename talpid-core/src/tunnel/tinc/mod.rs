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
};
use talpid_types::net::tinc;

use std::net::{TcpStream, SocketAddr};
use std::io::Write;

use tinc_plugin;

#[cfg(target_os = "linux")]
use which;
use std::net::IpAddr;

mod ping_monitor;

// amount of seconds to run `ping` until it returns.
const PING_TIMEOUT: u16 = 7;

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

/// Struct for monitoring an Tinc process.
pub struct TincMonitor {
    tinc:               TincOperator,
    on_event:           Box<dyn Fn(TunnelEvent) + Send + Sync + 'static>,
    _log_path:           Option<PathBuf>,
    resource_dir:       PathBuf,
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
            L: Fn(TunnelEvent) + Send + Sync + Clone + 'static,
    {
        let resource_dir_str = resource_dir.to_str().unwrap();

        let mut tinc_operator = TincOperator::new(resource_dir_str.to_string() + "/tinc/");

        tinc_operator.set_info_to_local(&params.config.tinc_info).map_err(Error::TincOperatorError)?;

        let child = tinc_operator.start_tinc().map_err(|_|Error::StartTincError)?;

        let event_rx = tinc_plugin::spawn();

        let pinger_event = on_event.clone();
        {
            let vip = tinc_operator.get_local_vip().map_err(Error::TincOperatorError)?;
            let vip_ipv4 = match vip {
                IpAddr::V4(x) => x,
                IpAddr::V6(_) => return Err(Error::StartTincError),
            };
            let ips = vec![vip.clone()];

            let interface_name = "dnet";
            let metadata = TunnelMetadata {
                interface: interface_name.to_string(),
                ips,
                ipv4_gateway: vip_ipv4,
                ipv6_gateway: None
            };

            ::std::thread::spawn(move || {
                if let Ok(()) = ping_monitor::ping(
                    IpAddr::from(vip.clone()),
                    PING_TIMEOUT,
                    &interface_name,
                    true) {
                    (pinger_event)(TunnelEvent::Up(metadata));
                };
            });
        }

        let on_event = Box::new(on_event.clone());

        return Ok(TincMonitor {
            tinc: tinc_operator,
            on_event,
            _log_path: log_file,
            resource_dir: resource_dir.to_owned(),
            event_rx,
            child: Arc::new(child),
            closed: Arc::new(AtomicBool::new(false)),
        });
    }

    fn tunnel_up(&self) -> Result<()> {
        let vip = self.tinc.get_local_vip().map_err(Error::TincOperatorError)?;
        let ips = vec![
            vip.clone(),
        ];

        let vip_ipv4 = match vip {
            IpAddr::V4(x) => x,
            IpAddr::V6(_) => return Err(Error::StartTincError),
        };

        let interface_name = "dnet";
        let metadata = TunnelMetadata {
            interface: interface_name.to_string(),
            ips,
            ipv4_gateway: vip_ipv4,
            ipv6_gateway: None
        };
        (self.on_event)(TunnelEvent::Up(metadata));
        Ok(())
    }

    /// Consumes the monitor and waits for both proxy and tunnel, as applicable.
    pub fn wait(self) -> Result<()> {
        let wait_result = match self.event_rx.recv() {
            Ok(tinc_plugin::EventType::Up) => {
                self.tunnel_up()?;
                return Ok(());
            },
            Ok(tinc_plugin::EventType::Down) => {
                (self.on_event)(TunnelEvent::Down);
                return Ok(());
            },
            Ok(_) => {
                Ok(())
            },
            Err(_) => {
                Ok(())
            },
        };
        wait_result
    }

    /// Creates a handle to this monitor, allowing the tunnel to be closed while some other
    /// thread is blocked in `wait`.
    pub fn close_handle(&self) -> TincCloseHandle {
        TincCloseHandle {
            child: self.child.clone(),
            closed: self.closed.clone(),
            pid_file: self.resource_dir.clone().join("/tinc/tinc.pid"),
        }
    }
}

/// 用于关闭tinc进程
pub struct TincCloseHandle {
    child:              Arc<duct::Handle>,
    closed:             Arc<AtomicBool>,
    pid_file:           PathBuf
}

impl TincCloseHandle {
    /// Kills the underlying Tinc process,
    /// making the `TincMonitor::wait` method return.
    pub fn close(self) -> io::Result<()> {
        if !self.closed.swap(true, Ordering::SeqCst) {
            if let Err(e) = tinc_plugin::control::stop(
                self.pid_file.to_str().unwrap()){
                log::warn!("{}", e);
                let _ = self.child.kill();
                sender_tinc_close();
            };
            Ok(())
        } else {
            Ok(())
        }
    }
}

fn sender_tinc_close() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 50070));
    let mut stream = TcpStream::connect(&addr).expect("Tinc Monitor not exist");
    stream.write("Down".as_bytes()).expect("Tinc Monitor not exist");
}
