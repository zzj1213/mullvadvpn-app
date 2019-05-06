use std::str::FromStr;
use std::net::IpAddr;
use std::fs::File;
use std::io::Read;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
}
