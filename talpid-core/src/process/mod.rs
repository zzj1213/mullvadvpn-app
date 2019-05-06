/// A module for all OpenVPN related process management.
pub mod openvpn;

// add by YanBowen
///A module for all Tinc related process management.
pub mod tinc;

/// A trait for stopping subprocesses gracefully.
pub mod stoppable_process;