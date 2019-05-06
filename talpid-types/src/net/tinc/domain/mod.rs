//! 这个模块用于记录Ovrouter运行信息
//! GeoInfo：本机地理地址信息
//! ProxyInfo：公网ip， uuid等
//! TincInfo： 本机tinc运行参数
//! new() 创建空的结构体
//! load_local() 根据本地信息创建结构体，将会读取tinc公钥，ip，vip等

use serde_json;

mod geo;
pub use self::geo::GeoInfo;
mod proxy;
pub use self::proxy::ProxyInfo;
pub use self::proxy::OnlineProxy;
mod auth;
pub use self::auth::AuthInfo;
mod tinc;
pub use self::tinc::TincInfo;

use std::io;

#[derive(Debug, Clone)]
pub struct Info {
    pub geo_info: GeoInfo,
    pub proxy_info: ProxyInfo,
    pub tinc_info: TincInfo,
}
impl Info {
    pub fn new() -> Self {
        let geo_info = GeoInfo::new();
        let proxy_info = ProxyInfo::new();
        let tinc_info = TincInfo::new();
        Info {
            geo_info,
            proxy_info,
            tinc_info,
        }
    }

    pub fn new_from_local(tinc_home_path: &str) -> io::Result<Self> {
        let mut geo_info = GeoInfo::new();
        if !geo_info.load_local() {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                                      "Load geo info error"));
        };
        let mut proxy_info = ProxyInfo::new();
        if !proxy_info.load_local() {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                                      "Load local proxy info error"));
        }
        // 使用geo ip 作为proxy ip, 而非使用本机路由default出口ip.
        proxy_info.proxy_ip = geo_info.ipaddr.clone();
        let mut tinc_info = TincInfo::new();

        log::debug!("geo_info: {:?}",geo_info);
        log::debug!("proxy_info: {:?}",proxy_info);
        log::debug!("tinc_info: {:?}",tinc_info);

        Ok(Info {
            geo_info,
            proxy_info,
            tinc_info,
        })
    }
}