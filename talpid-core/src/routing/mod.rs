#![cfg_attr(target_os = "android", allow(dead_code))]
// TODO: remove the allow(dead_code) for android once it's up to scratch.
use futures::{sync::oneshot, Future};
use ipnetwork::IpNetwork;
use std::{collections::HashMap, net::IpAddr};
use tokio_executor::Executor;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod imp;

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod imp;

pub use imp::Error as PlatformError;

/// Errors that can be encountered whilst initializing RouteManager
#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Platform sepcific error occured
    #[error(display = "Failed to create route manager")]
    FailedToInitializeManager(#[error(cause)] imp::Error),
    /// Failed to spawn route manager future
    #[error(display = "Failed to spawn route manager on the provided executor")]
    FailedToSpawnManager,
}

/// RouteManager applies a set of routes to the route table.
/// If a destination has to be routed through the default node,
/// the route will be adjusted dynamically when the default route changes.
pub struct RouteManager {
    tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
}

impl RouteManager {
    /// Constructs a RouteManager and applies the required routes.
    /// Takes a map of network destinations and network nodes as an argument, and applies said
    /// routes.
    pub fn new(
        required_routes: HashMap<IpNetwork, NetNode>,
        exec: &mut impl Executor,
    ) -> Result<Self, Error> {
        let (tx, rx) = oneshot::channel();


        let route_manager = imp::RouteManagerImpl::new(required_routes, rx)
            .map_err(Error::FailedToInitializeManager)?;
        exec.spawn(Box::new(
            route_manager.map_err(|e| log::error!("Routing manager failed - {}", e)),
        ))
        .map_err(|_| Error::FailedToSpawnManager)?;

        Ok(Self { tx: Some(tx) })
    }

    /// Stops RouteManager and removes all of the applied routes.
    pub fn stop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let (wait_tx, wait_rx) = oneshot::channel();
            if let Err(_) = tx.send(wait_tx) {
                log::error!("RouteManager already down!");
                return;
            }

            if let Err(_) = wait_rx.wait() {
                log::error!("RouteManager paniced while shutting down");
            }
        }
    }
}

impl Drop for RouteManager {
    fn drop(&mut self) {
        self.stop();
    }
}


/// A netowrk route with a specific network node, destinaiton and an optional metric.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Route {
    node: Node,
    prefix: IpNetwork,
    metric: Option<u32>,
}

impl Route {
    fn new(node: Node, prefix: IpNetwork) -> Self {
        Self {
            node,
            prefix,
            metric: None,
        }
    }
}

/// A network route that should be applied by the RouteManager.
/// It can either be routed through a specific network node or it can be routed through the current
/// default route.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct RequiredRoute {
    prefix: IpNetwork,
    node: NetNode,
}

impl RequiredRoute {
    /// Constructs a new required route.
    pub fn new(prefix: IpNetwork, node: impl Into<NetNode>) -> Self {
        Self {
            node: node.into(),
            prefix,
        }
    }
}

/// A NetNode represents a network node - either a real one or a symbolic default one.
/// A route with a symbolic default node will be changed whenever a new default route is created.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum NetNode {
    /// A real node will be used to set a regular route that will remain unchanged for the lifetime
    /// of the RouteManager
    RealNode(Node),
    /// A default node is a symbolic node that will resolve to the network node used in the current
    /// most preferable default route
    DefaultNode,
}

impl From<Node> for NetNode {
    fn from(node: Node) -> NetNode {
        NetNode::RealNode(node)
    }
}

/// Node represents a real network node - it can be identified by a network interface name, an IP
/// address or both.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Node {
    ip: Option<IpAddr>,
    device: Option<String>,
}

impl Node {
    /// Construct an Node with both an IP address and an interface name.
    pub fn new(address: IpAddr, iface_name: String) -> Self {
        Self {
            ip: Some(address),
            device: Some(iface_name),
        }
    }

    /// Construct an Node from an IP address.
    pub fn address(address: IpAddr) -> Node {
        Self {
            ip: Some(address),
            device: None,
        }
    }

    /// Construct a Node from a network interface name.
    pub fn device(iface_name: String) -> Node {
        Self {
            ip: None,
            device: Some(iface_name),
        }
    }

    /// Retrieve a node's IP address
    pub fn get_address(&self) -> Option<IpAddr> {
        self.ip
    }

    /// Retrieve a node's network interface name
    pub fn get_device(&self) -> Option<&str> {
        self.device.as_ref().map(|s| s.as_ref())
    }
}
