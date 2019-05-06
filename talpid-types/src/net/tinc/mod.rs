use crate::net::{Endpoint, GenericTunnelOptions, TransportProtocol, TunnelEndpoint, TunnelType};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub mod domain;
pub use self::domain::{Info, TincInfo, ProxyInfo, GeoInfo};

pub mod net_tool;
pub mod sys_tool;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct TunnelParameters {
    pub config: ConnectionConfig,
    // Empty. Reserved to tunnel command.
    pub options: TunnelOptions,
    // pub enable_ipv6: bool,
    pub generic_options: GenericTunnelOptions,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ConnectionConfig {
    pub endpoint:       Endpoint,
    pub tinc_info:      TincInfo,
//    pub this_node:      TincNode,
//    pub connect_to:     Vec<TincNode>,
}

impl ConnectionConfig {
    pub fn new(endpoint: Endpoint) -> ConnectionConfig {
        Self {
            endpoint,
            tinc_info: TincInfo::new(),
//            this_node,
//            connect_to,
        }
    }
    pub fn get_tunnel_endpoint(&self) -> TunnelEndpoint {
        TunnelEndpoint {
            tunnel_type: TunnelType::Tinc,
            endpoint: self.endpoint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TunnelOptions {}

//#[derive(Clone, Eq, PartialEq, Deserialize, Serialize, Debug)]
//pub struct TincNode {
//    pub ip:         String,
//    pub vip:        String,
//    pub pub_key:    String,
//}