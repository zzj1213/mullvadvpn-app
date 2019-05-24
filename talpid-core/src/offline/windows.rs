//! # License
//!
//! Copyright (C) 2018  Amagicom AB
//!
//! This program is free software: you can redistribute it and/or modify it under the terms of the
//! GNU General Public License as published by the Free Software Foundation, either version 3 of
//! the License, or (at your option) any later version.

use crate::{tunnel_state_machine::TunnelCommand, winnet};
use futures::sync::mpsc::UnboundedSender;
use parking_lot::Mutex;
use std::{
    ffi::c_void,
    io,
    mem::zeroed,
    os::windows::io::{IntoRawHandle, RawHandle},
    ptr,
    sync::Arc,
    thread,
    time::Duration,
};
use talpid_types::ErrorExt;
use winapi::{
    shared::{
        basetsd::LONG_PTR,
        minwindef::{DWORD, LPARAM, LRESULT, UINT, WPARAM},
        windef::HWND,
    },
    um::{
        handleapi::CloseHandle,
        libloaderapi::GetModuleHandleW,
        processthreadsapi::GetThreadId,
        synchapi::WaitForSingleObject,
        winbase::INFINITE,
        winuser::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            GetWindowLongPtrW, PostQuitMessage, PostThreadMessageW, SetWindowLongPtrW,
            GWLP_USERDATA, GWLP_WNDPROC, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, WM_DESTROY,
            WM_POWERBROADCAST, WM_USER,
        },
    },
};

const CLASS_NAME: &[u8] = b"S\0T\0A\0T\0I\0C\0\0\0";
const REQUEST_THREAD_SHUTDOWN: UINT = WM_USER + 1;


#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "Unable to create listener thread")]
    ThreadCreationError(#[error(cause)] io::Error),
    #[error(display = "Failed to start connectivity monitor")]
    ConnectivityMonitorError,
}


pub struct BroadcastListener {
    thread_handle: RawHandle,
    thread_id: DWORD,
    _system_state: Arc<Mutex<SystemState>>,
}

unsafe impl Send for BroadcastListener {}

impl BroadcastListener {
    pub fn start(sender: UnboundedSender<TunnelCommand>) -> Result<Self, Error> {
        let mut system_state = Arc::new(Mutex::new(SystemState {
            network_connectivity: false,
            suspended: false,
            daemon_channel: sender,
        }));

        let power_broadcast_state_ref = system_state.clone();

        let power_broadcast_callback = move |message: UINT, wparam: WPARAM, _lparam: LPARAM| {
            let state = power_broadcast_state_ref.clone();
            if message == WM_POWERBROADCAST {
                if wparam == PBT_APMSUSPEND {
                    log::debug!("Machine is preparing to enter sleep mode");
                    apply_system_state_change(state, StateChange::Suspended(true));
                } else if wparam == PBT_APMRESUMEAUTOMATIC {
                    log::debug!("Machine is returning from sleep mode");
                    thread::spawn(move || {
                        // TAP will be unavailable for approximately 2 seconds on a healthy machine.
                        thread::sleep(Duration::from_secs(5));
                        log::debug!("TAP is presumed to have been re-initialized");
                        apply_system_state_change(state, StateChange::Suspended(false));
                    });
                }
            }
        };

        let join_handle = thread::Builder::new()
            .spawn(move || unsafe {
                Self::message_pump(power_broadcast_callback);
            })
            .map_err(Error::ThreadCreationError)?;

        let real_handle = join_handle.into_raw_handle();

        unsafe { Self::setup_network_connectivity_listener(&mut system_state)? };

        Ok(BroadcastListener {
            thread_handle: real_handle,
            thread_id: unsafe { GetThreadId(real_handle) },
            _system_state: system_state,
        })
    }

    unsafe fn message_pump<F>(client_callback: F)
    where
        F: Fn(UINT, WPARAM, LPARAM),
    {
        let dummy_window = CreateWindowExW(
            0,
            CLASS_NAME.as_ptr() as *const u16,
            ptr::null_mut(),
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            GetModuleHandleW(ptr::null_mut()),
            ptr::null_mut(),
        );

        // Move callback information to the heap.
        // This enables us to reach the callback through a "thin pointer".
        let boxed_callback = Box::new(client_callback);

        // Detach callback from Box.
        let raw_callback = Box::into_raw(boxed_callback) as *mut c_void;

        SetWindowLongPtrW(dummy_window, GWLP_USERDATA, raw_callback as LONG_PTR);
        SetWindowLongPtrW(
            dummy_window,
            GWLP_WNDPROC,
            Self::window_procedure::<F> as LONG_PTR,
        );

        let mut msg = zeroed();

        loop {
            let status = GetMessageW(&mut msg, 0 as HWND, 0, 0);

            if status < 0 {
                continue;
            }
            if status == 0 {
                break;
            }

            if msg.hwnd.is_null() {
                if msg.message == REQUEST_THREAD_SHUTDOWN {
                    DestroyWindow(dummy_window);
                }
            } else {
                DispatchMessageW(&mut msg);
            }
        }

        // Reattach callback to Box for proper clean-up.
        let _ = Box::from_raw(raw_callback as *mut F);
    }

    unsafe extern "system" fn window_procedure<F>(
        window: HWND,
        message: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT
    where
        F: Fn(UINT, WPARAM, LPARAM),
    {
        let raw_callback = GetWindowLongPtrW(window, GWLP_USERDATA);

        if raw_callback != 0 {
            let typed_callback = &mut *(raw_callback as *mut F);
            typed_callback(message, wparam, lparam);
        }

        if message == WM_DESTROY {
            PostQuitMessage(0);
            return 0;
        }

        DefWindowProcW(window, message, wparam, lparam)
    }

    /// The caller must make sure the `system_state` reference is valid
    /// until after `WinNet_DeactivateConnectivityMonitor` has been called.
    unsafe fn setup_network_connectivity_listener(
        system_state: &Mutex<SystemState>,
    ) -> Result<(), Error> {
        let callback_context = system_state as *const _ as *mut libc::c_void;
        let mut state = system_state.lock();
        let mut current_connectivity = true;
        if !winnet::WinNet_ActivateConnectivityMonitor(
            Some(Self::connectivity_callback),
            callback_context,
            &mut current_connectivity as *mut _,
            Some(winnet::error_sink),
            ptr::null_mut(),
        ) {
            return Err(Error::ConnectivityMonitorError);
        }
        state.network_connectivity = current_connectivity;
        Ok(())
    }

    unsafe extern "system" fn connectivity_callback(connectivity: bool, context: *mut c_void) {
        let state_lock: &mut Mutex<SystemState> = &mut *(context as *mut _);
        let mut state = state_lock.lock();
        state.apply_change(StateChange::NetworkConnectivity(connectivity));
    }
}

impl Drop for BroadcastListener {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.thread_id, REQUEST_THREAD_SHUTDOWN, 0, 0);
            WaitForSingleObject(self.thread_handle, INFINITE);
            CloseHandle(self.thread_handle);
            if !winnet::WinNet_DeactivateConnectivityMonitor() {
                log::error!("Failed to deactivate connectivity monitor");
            }
        }
    }
}

#[derive(Debug)]
enum StateChange {
    NetworkConnectivity(bool),
    Suspended(bool),
}

struct SystemState {
    network_connectivity: bool,
    suspended: bool,
    daemon_channel: UnboundedSender<TunnelCommand>,
}

impl SystemState {
    fn apply_change(&mut self, change: StateChange) {
        let old_state = self.is_offline_currently();
        match change {
            StateChange::NetworkConnectivity(connectivity) => {
                self.network_connectivity = connectivity;
            }

            StateChange::Suspended(suspended) => {
                self.suspended = suspended;
            }
        };

        let new_state = self.is_offline_currently();
        if old_state != new_state {
            if let Err(e) = self
                .daemon_channel
                .unbounded_send(TunnelCommand::IsOffline(new_state))
            {
                log::error!("Failed to send new offline state to daemon: {}", e);
            }
        }
    }

    fn is_offline_currently(&self) -> bool {
        !self.network_connectivity || self.suspended
    }
}

pub type MonitorHandle = BroadcastListener;

pub fn spawn_monitor(sender: UnboundedSender<TunnelCommand>) -> Result<MonitorHandle, Error> {
    BroadcastListener::start(sender)
}

fn apply_system_state_change(state: Arc<Mutex<SystemState>>, change: StateChange) {
    let mut state = state.lock();
    state.apply_change(change);
}

pub fn is_offline() -> bool {
    match winnet::is_offline() {
        Ok(state) => state,
        Err(e) => {
            log::error!(
                "{}",
                e.display_chain_with_msg("Failed to get current connectivity")
            );
            false
        }
    }
}
