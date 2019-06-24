#![cfg(target_os = "android")]

mod daemon_interface;
mod from_java;
mod into_java;
mod is_null;
mod jni_event_listener;
mod vpn_service_tun_provider;

use crate::{
    daemon_interface::DaemonInterface, from_java::FromJava, into_java::IntoJava,
    jni_event_listener::JniEventListener, vpn_service_tun_provider::VpnServiceTunProvider,
};
use jni::{
    objects::{GlobalRef, JObject, JString},
    sys::{jboolean, JNI_FALSE, JNI_TRUE},
    JNIEnv,
};
use lazy_static::lazy_static;
use mullvad_daemon::{logging, version, Daemon, DaemonCommandSender};
use parking_lot::RwLock;
use std::{collections::HashMap, path::PathBuf, sync::mpsc, thread};
use talpid_types::ErrorExt;

const LOG_FILENAME: &str = "daemon.log";

const CLASSES_TO_LOAD: &[&str] = &[
    "java/net/InetAddress",
    "java/util/ArrayList",
    "net/mullvad/mullvadvpn/model/AccountData",
    "net/mullvad/mullvadvpn/model/Constraint$Any",
    "net/mullvad/mullvadvpn/model/Constraint$Only",
    "net/mullvad/mullvadvpn/model/GeoIpLocation",
    "net/mullvad/mullvadvpn/model/InetNetwork",
    "net/mullvad/mullvadvpn/model/LocationConstraint$City",
    "net/mullvad/mullvadvpn/model/LocationConstraint$Country",
    "net/mullvad/mullvadvpn/model/LocationConstraint$Hostname",
    "net/mullvad/mullvadvpn/model/PublicKey",
    "net/mullvad/mullvadvpn/model/Relay",
    "net/mullvad/mullvadvpn/model/RelayList",
    "net/mullvad/mullvadvpn/model/RelayListCity",
    "net/mullvad/mullvadvpn/model/RelayListCountry",
    "net/mullvad/mullvadvpn/model/RelaySettings$CustomTunnelEndpoint",
    "net/mullvad/mullvadvpn/model/RelaySettings$RelayConstraints",
    "net/mullvad/mullvadvpn/model/RelaySettingsUpdate$CustomTunnelEndpoint",
    "net/mullvad/mullvadvpn/model/RelaySettingsUpdate$RelayConstraintsUpdate",
    "net/mullvad/mullvadvpn/model/Settings",
    "net/mullvad/mullvadvpn/model/TunConfig",
    "net/mullvad/mullvadvpn/model/TunnelStateTransition$Blocked",
    "net/mullvad/mullvadvpn/model/TunnelStateTransition$Connected",
    "net/mullvad/mullvadvpn/model/TunnelStateTransition$Connecting",
    "net/mullvad/mullvadvpn/model/TunnelStateTransition$Disconnected",
    "net/mullvad/mullvadvpn/model/TunnelStateTransition$Disconnecting",
    "net/mullvad/mullvadvpn/MullvadDaemon",
    "net/mullvad/mullvadvpn/MullvadVpnService",
];

lazy_static! {
    static ref DAEMON_INTERFACE: DaemonInterface = DaemonInterface::new();
    static ref CLASSES: RwLock<HashMap<&'static str, GlobalRef>> =
        RwLock::new(HashMap::with_capacity(CLASSES_TO_LOAD.len()));
}

#[derive(Debug, err_derive::Error)]
pub enum Error {
    #[error(display = "Failed to create VpnService tunnel provider")]
    CreateVpnServiceTunProvider(#[error(cause)] vpn_service_tun_provider::Error),

    #[error(display = "Failed to get cache directory path")]
    GetCacheDir(#[error(cause)] mullvad_paths::Error),

    #[error(display = "Failed to initialize the mullvad daemon")]
    InitializeDaemon(#[error(cause)] mullvad_daemon::Error),

    #[error(display = "Failed to spawn the JNI event listener")]
    SpawnJniEventListener(#[error(cause)] jni_event_listener::Error),
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_initialize(
    env: JNIEnv,
    this: JObject,
    vpnService: JObject,
) {
    let log_dir = start_logging();

    load_classes(&env);

    if let Err(error) = initialize(&env, &this, &vpnService, log_dir) {
        log::error!("{}", error.display_chain());
    }
}

fn start_logging() -> PathBuf {
    let log_dir = mullvad_paths::log_dir().unwrap();
    let log_file = log_dir.join(LOG_FILENAME);

    logging::init_logger(log::LevelFilter::Debug, Some(&log_file), true).unwrap();
    log_panics::init();
    version::log_version();

    log_dir
}

fn load_classes(env: &JNIEnv) {
    let mut classes = CLASSES.write();

    for class in CLASSES_TO_LOAD {
        classes.insert(class, load_class_reference(env, class));
    }
}

fn load_class_reference(env: &JNIEnv, name: &str) -> GlobalRef {
    let class = match env.find_class(name) {
        Ok(class) => class,
        Err(_) => panic!("Failed to find {} Java class", name),
    };

    env.new_global_ref(JObject::from(class))
        .expect("Failed to convert local reference to Java class into a global reference")
}

fn initialize(
    env: &JNIEnv,
    this: &JObject,
    vpn_service: &JObject,
    log_dir: PathBuf,
) -> Result<(), Error> {
    let tun_provider =
        VpnServiceTunProvider::new(env, vpn_service).map_err(Error::CreateVpnServiceTunProvider)?;
    let daemon_command_sender = spawn_daemon(env, this, tun_provider, log_dir)?;

    DAEMON_INTERFACE.set_command_sender(daemon_command_sender);

    Ok(())
}

fn spawn_daemon(
    env: &JNIEnv,
    this: &JObject,
    tun_provider: VpnServiceTunProvider,
    log_dir: PathBuf,
) -> Result<DaemonCommandSender, Error> {
    let listener = JniEventListener::spawn(env, this).map_err(Error::SpawnJniEventListener)?;
    let (tx, rx) = mpsc::channel();

    thread::spawn(
        move || match create_daemon(listener, tun_provider, log_dir) {
            Ok(daemon) => {
                let _ = tx.send(Ok(daemon.command_sender()));
                match daemon.run() {
                    Ok(()) => log::info!("Mullvad daemon has stopped"),
                    Err(error) => log::error!("{}", error.display_chain()),
                }
            }
            Err(error) => {
                let _ = tx.send(Err(error));
            }
        },
    );

    rx.recv().unwrap()
}

fn create_daemon(
    listener: JniEventListener,
    tun_provider: VpnServiceTunProvider,
    log_dir: PathBuf,
) -> Result<Daemon<JniEventListener>, Error> {
    let resource_dir = mullvad_paths::get_resource_dir();
    let cache_dir = mullvad_paths::cache_dir().map_err(Error::GetCacheDir)?;

    let daemon = Daemon::start_with_event_listener_and_tun_provider(
        listener,
        tun_provider,
        Some(log_dir),
        resource_dir,
        cache_dir,
        version::PRODUCT_VERSION.to_owned(),
    )
    .map_err(Error::InitializeDaemon)?;

    Ok(daemon)
}

fn get_class(name: &str) -> GlobalRef {
    match CLASSES.read().get(name) {
        Some(class) => class.clone(),
        None => panic!("Class not loaded: {}", name),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_connect(_: JNIEnv, _: JObject) {
    if let Err(error) = DAEMON_INTERFACE.connect() {
        log::error!(
            "{}",
            error.display_chain_with_msg("Failed to request daemon to connect")
        );
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_disconnect(_: JNIEnv, _: JObject) {
    if let Err(error) = DAEMON_INTERFACE.disconnect() {
        log::error!(
            "{}",
            error.display_chain_with_msg("Failed to request daemon to disconnect")
        );
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_generateWireguardKey(
    _: JNIEnv,
    _: JObject,
) -> jboolean {
    match DAEMON_INTERFACE.generate_wireguard_key() {
        Ok(()) => JNI_TRUE,
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to generate wireguard key")
            );
            JNI_FALSE
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_getAccountData<'env, 'this>(
    env: JNIEnv<'env>,
    _: JObject<'this>,
    accountToken: JString,
) -> JObject<'env> {
    let account = String::from_java(&env, accountToken);

    match DAEMON_INTERFACE.get_account_data(account) {
        Ok(data) => data.into_java(&env),
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to get account data")
            );
            JObject::null()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_getCurrentLocation<'env, 'this>(
    env: JNIEnv<'env>,
    _: JObject<'this>,
) -> JObject<'env> {
    match DAEMON_INTERFACE.get_current_location() {
        Ok(location) => location.into_java(&env),
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to get current location")
            );
            JObject::null()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_getRelayLocations<'env, 'this>(
    env: JNIEnv<'env>,
    _: JObject<'this>,
) -> JObject<'env> {
    match DAEMON_INTERFACE.get_relay_locations() {
        Ok(relay_list) => relay_list.into_java(&env),
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to get relay locations")
            );
            JObject::null()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_getSettings<'env, 'this>(
    env: JNIEnv<'env>,
    _: JObject<'this>,
) -> JObject<'env> {
    match DAEMON_INTERFACE.get_settings() {
        Ok(settings) => settings.into_java(&env),
        Err(error) => {
            log::error!("{}", error.display_chain_with_msg("Failed to get settings"));
            JObject::null()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_getState<'env, 'this>(
    env: JNIEnv<'env>,
    _: JObject<'this>,
) -> JObject<'env> {
    match DAEMON_INTERFACE.get_state() {
        Ok(state) => state.into_java(&env),
        Err(error) => {
            log::error!("{}", error.display_chain_with_msg("Failed to get state"));
            JObject::null()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_getWireguardKey<'env, 'this>(
    env: JNIEnv<'env>,
    _: JObject<'this>,
) -> JObject<'env> {
    match DAEMON_INTERFACE.get_wireguard_key() {
        Ok(public_key) => public_key.into_java(&env),
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to get wireguard key")
            );
            JObject::null()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_setAccount(
    env: JNIEnv,
    _: JObject,
    accountToken: JString,
) {
    let account = <Option<String> as FromJava>::from_java(&env, accountToken);

    if let Err(error) = DAEMON_INTERFACE.set_account(account) {
        log::error!("{}", error.display_chain_with_msg("Failed to set account"));
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_MullvadDaemon_updateRelaySettings(
    env: JNIEnv,
    _: JObject,
    relaySettingsUpdate: JObject,
) {
    let update = FromJava::from_java(&env, relaySettingsUpdate);

    if let Err(error) = DAEMON_INTERFACE.update_relay_settings(update) {
        log::error!(
            "{}",
            error.display_chain_with_msg("Failed to update relay settings")
        );
    }
}
