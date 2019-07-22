use std::str::FromStr;
use std::net::IpAddr;
use std::fs::File;
use std::io::Read;

use crate::net::{Endpoint, GenericTunnelOptions, TransportProtocol, TunnelEndpoint, TunnelType};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TincRunMode {
    Client,
    Proxy,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TincInfo {
    pub ip:         IpAddr,
    pub vip:        IpAddr,
    pub pub_key:    String,
    pub mode:       TincRunMode,
    pub connect_to: Vec<IpAddr>,
}

impl TincInfo {
    pub fn new() -> Self {
        let ip = IpAddr::from_str("0.0.0.0").unwrap();
        let vip = IpAddr::from_str("0.0.0.0").unwrap();
        let pub_key = "".to_string();
        TincInfo {
            ip,
            vip,
            pub_key,
            mode: TincRunMode::Client,
            connect_to: vec![],
        }
    }
}

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
            proxy: None,
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
