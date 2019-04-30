use std::str::FromStr;
use std::net::IpAddr;
use std::fs::File;
use std::io::Read;

use serde::{Deserialize, Serialize};

use super::net_tool::get_local_ip;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct TincInfo {
    pub vip: IpAddr,
    pub pub_key: String,
}
impl TincInfo {
    pub fn new() -> Self {
        let vip = IpAddr::from_str("0.0.0.0").unwrap();
        let pub_key = "".to_string();
        TincInfo {
            vip,
            pub_key,
        }
    }

    // Load local tinc config file vpnserver for tinc vip and pub_key.
    // Success return true.
    pub fn load_local(&mut self, tinc_home: &str, pub_key_path: &str) -> bool {
        let mut _file = File::open(tinc_home.to_string() + "/hosts/vpnserver").unwrap();
        let mut res = String::new();
        _file.read_to_string(&mut res).unwrap();
        let tmp: Vec<&str> = res.split("\n").collect();
        let tmp: Vec<&str> = tmp[0].split(" ").collect();
        let vip = tmp[2];
        self.vip = IpAddr::from_str(vip).unwrap();

        let mut res = String::new();
        let mut _file = File::open(tinc_home.to_string() + pub_key_path).unwrap();
        _file.read_to_string(&mut res).unwrap();
        self.pub_key = res.clone().replace("\n", "");
        return true;
    }
}

#[derive(Debug, Clone)]
pub struct ProxyInfo {
    pub uid: String,
    pub proxy_pub_key: String,
    pub isregister: bool,
    pub cookie: String,
    pub auth_type: String,
    pub os: String,
    pub server_type: String,
    pub proxy_ip: String,
    pub ssh_port: String,
}
impl ProxyInfo {
    pub fn new() -> Self {
        ProxyInfo {
            uid: String::new(),
            proxy_pub_key: String::new(),
            isregister: false,
            cookie: String::new(),
            auth_type: String::new(),
            os: String::new(),
            server_type: String::new(),
            proxy_ip: "0.0.0.0".to_string(),
            ssh_port: String::new(),
        }
    }

    pub fn create_uid(&mut self) -> bool {
        self.uid = uuid::Uuid::new_v4().to_string();
        true
    }

    pub fn load_local(&mut self) -> bool {
        self.auth_type = "0".to_string();
        self.server_type = "vppn1".to_string();
        self.os = "ubuntu".to_string();
        if let Ok(local_ip) = get_local_ip() {
            self.proxy_ip = local_ip.to_string();
        } else {
            return false;
        };
        true
    }
}
