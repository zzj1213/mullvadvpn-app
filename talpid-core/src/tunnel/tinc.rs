#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]

use super::{TunnelEvent, TunnelMetadata};
use crate::{
    mktemp,
    process::{
        tinc::TincCommand,
        stoppable_process::StoppableProcess,
    }
};
#[cfg(target_os = "linux")]
use failure::ResultExt as FailureResultExt;
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};
use talpid_ipc;
use talpid_types::net::tinc;

use tinc_plugin;

#[cfg(target_os = "linux")]
use which;
use std::net::{IpAddr, Ipv4Addr};


/// Results from fallible operations on the Tinc tunnel.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can happen when using the Tinc tunnel.
#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Unable to start, wait for or kill the Tinc process.
    #[error(display = "Error in Tinc process management: {}", _0)]
    ChildProcessError(&'static str, #[error(cause)] io::Error),

    /// Unable to start or manage the IPC server listening for events from Tinc.
    #[error(display = "Unable to start or manage the event dispatcher IPC server")]
    EventDispatcherError(#[error(cause)] talpid_ipc::Error),

    /// The Tinc event dispatcher exited unexpectedly
    #[error(display = "The Tinc event dispatcher exited unexpectedly")]
    EventDispatcherExited,

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

    /// The Tinc plugin was not found.
    #[error(display = "No Tinc plugin found at {}", _0)]
    PluginNotFound(String),

    /// Error while writing credentials to temporary file.
    #[error(display = "Error while writing credentials to temporary file")]
    CredentialsWriteError(#[error(cause)] io::Error),

    /// Failures related to the proxy service.
    #[error(display = "Unable to start the proxy service")]
    StartProxyError(#[error(cause)] io::Error),

    /// Error while monitoring proxy service
    #[error(display = "Error while monitoring proxy service")]
    MonitorProxyError(#[error(cause)] io::Error),

    /// The proxy exited unexpectedly
    #[error(
    display = "The proxy exited unexpectedly providing these details: {}",
    _0
    )]
    ProxyExited(String),

    /// Failure in Windows syscall.
    #[cfg(windows)]
    #[error(display = "Failure in Windows syscall")]
    WinnetError(#[error(cause)] crate::winnet::Error),
}


#[cfg(unix)]
static TINC_DIE_TIMEOUT: Duration = Duration::from_secs(4);
#[cfg(windows)]
static TINC_DIE_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(unix)]
const TINC_BIN_FILENAME: &str = "tincd";
#[cfg(windows)]
const TINC_BIN_FILENAME: &str = "tincd.exe";

pub struct Config {
    ips: Vec<IpAddr>,
    ipv4_gateway: Ipv4Addr,
}

/// Struct for monitoring an Tinc process.
#[derive(Debug)]
pub struct TincMonitor {
    child:              Arc<duct::ProcessHandle>,
    on_event:           Box<dyn Fn(TunnelEvent) + Send + Sync + 'static>,
    log_path:           Option<PathBuf>,
    closed:             Arc<AtomicBool>,
    event_rx:           mpsc::Receiver<tinc_plugin::EventType>,

//    预留给tinc host-up host-down
//    event_callback:
//    msg_receiver: mpsc::Receiver<>,
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
        let pidfile: PathBuf = resource_dir.join("tinc.pid");
        let pidfile = pidfile.as_path();

        let mut logfile = resource_dir.join("tinc.log");

        if let Some(x) = log_file.clone() {
            logfile = x.to_owned();
        };

        let logfile = logfile.as_path();

        let cmd = Self::create_tinc_cmd(
            resource_dir,
            pidfile,
            logfile,
        )?;

        Self::new_internal(
            cmd,
            on_event,
            resource_dir,
            log_file,
        )
    }

    pub fn event_server<L>(on_event: L,
                           event_rx: mpsc::Receiver<tinc_plugin::EventType>)
                           -> Result<()>
        where
            L: Fn(tinc_plugin::EventType, HashMap<String, String>) + Send + Sync + 'static,
    {
        let mut evn = HashMap::new();
        evn.insert("dev".to_string(), "tun0".to_string());
        evn.insert("ifconfig_local".to_string(), "[10.253.1.3]".to_string());
        evn.insert("route_vpn_gateway".to_string(), "10.255.255.1".to_string());

//        let event_handle = TincEventApiImpl { on_event };
        let event = event_rx.recv().map_err(|_|Error::EventDispatcherError)?;
        on_event(event, evn);
        return Ok(());
    }

    fn tunnel_up(&self, config: &Config) {
        let interface_name = "tun0";
        let metadata = TunnelMetadata {
            interface: interface_name.to_string(),
            ips: config.ips.clone(),
            ipv4_gateway: config.ipv4_gateway.clone(),
            ipv6_gateway: None
        };
        (self.event_callback)(TunnelEvent::Up(metadata));
    }
}

impl<C: TincBuilder + 'static> TincMonitor {
    fn new_internal<L>(
        mut cmd: C,
        on_event: L,
        resource_dir: &Path,
        log_path: Option<PathBuf>,
    ) -> Result<TincMonitor>
        where
            L: Fn(tinc_plugin::EventType) + Send + Sync + 'static,
    {
        let child = cmd
            .start()
            .map_err(|e| Error::ChildProcessError("Failed to start", e))?;

        let event_rx = tinc_plugin::spawn();

        let on_event = Box::new(on_event);
        Ok(TincMonitor {
            child: Arc::new(child),
            closed: Arc::new(AtomicBool::new(false)),
            on_event,
            log_path,
            event_rx,
        })
    }

    /// Creates a handle to this monitor, allowing the tunnel to be closed while some other
    /// thread is blocked in `wait`.
    pub fn close_handle(&self) -> TincCloseHandle<C::ProcessHandle> {
        TincCloseHandle {
            child: self.child.clone(),
            closed: self.closed.clone(),
        }
    }

    /// Consumes the monitor and waits for both proxy and tunnel, as applicable.
    pub fn wait(mut self) -> Result<()> {
        let wait_result = match self.event_rx.recv() {
            Ok(tinc_plugin::EventType::Up) => Ok(()),
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        };
        (self.on_event)(TunnelEvent::Down);
        wait_result
    }

    fn create_tinc_cmd(
        resource_dir:   &Path,
        pidfile:        &Path,
        logfile:        &Path,
    ) -> Result<TincCommand> {
        if let Some(res_dir) = resource_dir.to_str() {
            let mut cmd = TincCommand::new(res_dir.to_string());
            cmd.config(resource_dir);
            cmd.pidfile(pidfile);
            cmd.logfile(logfile);
            Ok(cmd)
        }
    }

    fn get_tinc_bin(resource_dir: &Path) -> Result<PathBuf> {
        let path = resource_dir.join(TINC_BIN_FILENAME);
        if path.exists() {
            log::debug!("Using Tinc at {}", path.display());
            return Ok(path);
        }
        else {
            Err(Error::TincNotFound(path.display().to_string()))
        }
    }

    fn get_config_path(resource_dir: &Path) -> Option<PathBuf> {
        let path = resource_dir.join("tinc.conf");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

/// A handle to an `TincMonitor` for closing it.
#[derive(Debug, Clone)]
pub struct TincCloseHandle<H: ProcessHandle = TincProcHandle> {
    child: Arc<H>,
    closed: Arc<AtomicBool>,
}

impl<H: ProcessHandle> TincCloseHandle<H> {
    /// Kills the underlying Tinc process, making the `TincMonitor::wait` method return.
    pub fn close(self) -> io::Result<()> {
        log::debug!("TincCloseHandle - close");
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.child.kill()
        } else {
            Ok(())
        }
    }
}

/// Internal enum to differentiate between if the child process or the event dispatcher died first.
#[derive(Debug)]
enum WaitResult {
    Child(io::Result<ExitStatus>, bool),
    EventDispatcher,
}

/// Trait for types acting as Tinc process starters for `TincMonitor`.
pub trait TincBuilder {
    /// The type of handles to subprocesses this builder produces.
    type ProcessHandle: ProcessHandle;

    fn start(&self) -> io::Result<Self::ProcessHandle>;
}

/// Trait for types acting as handles to subprocesses for `TincMonitor`
pub trait ProcessHandle: Send + Sync + 'static {
    /// Block until the subprocess exits or there is an error in the wait syscall.
    fn wait(&self) -> io::Result<ExitStatus>;

    /// Kill the subprocess.
    fn kill(&self) -> io::Result<()>;
}

impl TincBuilder for TincCommand {
    type ProcessHandle = TincProcHandle;

    fn start(&self) -> io::Result<TincProcHandle> {
        TincProcHandle::new(self.build())
    }
}

impl ProcessHandle for TincProcHandle {
    fn wait(&self) -> io::Result<ExitStatus> {
        self.inner.wait().map(|output| output.status)
    }

    fn kill(&self) -> io::Result<()> {
        self.nice_kill(Tinc_DIE_TIMEOUT)
    }
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//    use crate::mktemp::TempFile;
//    use parking_lot::Mutex;
//    use std::{
//        path::{Path, PathBuf},
//        sync::Arc,
//    };
//
//    #[derive(Debug, Default, Clone)]
//    struct TestTincBuilder {
//        pub log: Arc<Mutex<Option<PathBuf>>>,
//        pub process_handle: Option<TestProcessHandle>,
//    }
//
//    impl TincBuilder for TestTincBuilder {
//        type ProcessHandle = TestProcessHandle;
////        fn log(&mut self, log: Option<impl AsRef<Path>>) -> &mut Self {
////            *self.log.lock() = log.as_ref().map(|path| path.as_ref().to_path_buf());
////            self
////        }
//
//        fn start(&self) -> io::Result<Self::ProcessHandle> {
//            self.process_handle
//                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to start"))
//        }
//    }
//
//    #[derive(Debug, Copy, Clone)]
//    struct TestProcessHandle(i32);
//
//    impl ProcessHandle for TestProcessHandle {
//        #[cfg(unix)]
//        fn wait(&self) -> io::Result<ExitStatus> {
//            use std::os::unix::process::ExitStatusExt;
//            Ok(ExitStatus::from_raw(self.0))
//        }
//
//        #[cfg(windows)]
//        fn wait(&self) -> io::Result<ExitStatus> {
//            use std::os::windows::process::ExitStatusExt;
//            Ok(ExitStatus::from_raw(self.0 as u32))
//        }
//
//        fn kill(&self) -> io::Result<()> {
//            Ok(())
//        }
//    }
//
//    #[test]
//    fn sets_log() {
//        let builder = TestTincBuilder::default();
//        let _ = TincMonitor::new_internal(
//            builder.clone(),
//            |_, _| {},
//            &Path::new(""),
//            Some(PathBuf::from("./my_test_log_file")),
//        );
//        assert_eq!(
//            Some(PathBuf::from("./my_test_log_file")),
//            *builder.log.lock()
//        );
//    }
//
//    #[test]
//    fn exit_successfully() {
//        let mut builder = TestTincBuilder::default();
//        builder.process_handle = Some(TestProcessHandle(0));
//        let testee =
//            TincMonitor::new_internal(
//                builder.clone(),
//                |_, _| {},
//                &Path::new(""),
//                Some(PathBuf::from("./my_test_log_file")),
//            ).unwrap();
//        assert!(testee.wait().is_ok());
//    }
//
//    #[test]
//    fn exit_error() {
//        let mut builder = TestTincBuilder::default();
//        builder.process_handle = Some(TestProcessHandle(1));
//        let testee =
//            TincMonitor::new_internal(
//                builder.clone(),
//                |_, _| {},
//                &Path::new(""),
//                Some(PathBuf::from("./my_test_log_file")),
//            ).unwrap();
//        assert!(testee.wait().is_err());
//    }
//
//    #[test]
//    fn wait_closed() {
//        let mut builder = TestTincBuilder::default();
//        builder.process_handle = Some(TestProcessHandle(1));
//        let testee =
//            TincMonitor::new_internal(
//                builder.clone(),
//                |_, _| {},
//                &Path::new(""),
//                Some(PathBuf::from("./my_test_log_file")),
//            ).unwrap();
//        testee.close_handle().close().unwrap();
//        assert!(testee.wait().is_ok());
//    }
//
//    #[test]
//    fn failed_process_start() {
//        let builder = TestTincBuilder::default();
//        let error =
//            TincMonitor::new_internal(
//                builder.clone(),
//                |_, _| {},
//                &Path::new(""),
//                Some(PathBuf::from("./my_test_log_file")),
//            ).unwrap_err();
//        match error {
//            Error::ChildProcessError(..) => (),
//            _ => panic!("Wrong error"),
//        }
//    }
//}
