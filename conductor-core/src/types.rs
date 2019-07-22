use chrono::{offset::Utc, TimeZone, DateTime};
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AccountInfo {
    pub expiry:     DateTime<Utc>,
    pub status:     String,
    pub vip:        IpAddr,
}

impl AccountInfo {
    pub fn new() -> Self {
        let expiry = Utc.ymd(1970, 1, 1)
            .and_hms_milli(0, 0, 0, 0);

        AccountInfo {
            expiry,
            status: String::new(),
            vip:    IpAddr::from_str("255.255.255.255").unwrap(),
        }
    }
}