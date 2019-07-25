#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]

use std::fs;
use std::io::{self, Read};
use std::net::IpAddr;
use std::str::FromStr;

use tinc_plugin::TincInfo;
use tinc_plugin::{TincOperator as PluginTincOperator, TincOperatorError};

/// Errors that can happen when using the Tinc tunnel
pub type Error = TincOperatorError;

/// Results from fallible operations on the Tinc tunnel.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(unix)]
const TINC_BIN_FILENAME: &str = "tincd";
#[cfg(windows)]
const TINC_BIN_FILENAME: &str = "tincd.exe";

const PRIV_KEY_FILENAME: &str = "rsa_key.priv";

const PUB_KEY_FILENAME: &str = "rsa_key.pub";

#[cfg(unix)]
const TINC_UP_FILENAME: &str = "tinc-up";
#[cfg(windows)]
const TINC_UP_FILENAME: &str = "tinc-up.bat";

#[cfg(unix)]
const TINC_DOWN_FILENAME: &str = "tinc-down";
#[cfg(windows)]
const TINC_DOWN_FILENAME: &str = "tinc-down.bat";

#[cfg(unix)]
const HOST_UP_FILENAME: &str = "host-up";
#[cfg(windows)]
const HOST_UP_FILENAME: &str = "host-up.bat";

#[cfg(unix)]
const HOST_DOWN_FILENAME: &str = "host-down";
#[cfg(windows)]
const HOST_DOWN_FILENAME: &str = "host-down.bat";

const TINC_AUTH_PATH: &str = "auth/";
const TINC_AUTH_FILENAME: &str = "auth.txt";

/// Tinc operator
pub struct TincOperator {
    tinc_home:              String,
}
impl TincOperator {
    /// 获取tinc home dir 创建tinc操作。
    pub fn new(tinc_home: String) -> Self {
        TincOperator {
            tinc_home,
        }
    }

    /// 启动tinc 返回duct::handle
    pub fn start_tinc(&mut self) -> Result<duct::Handle> {
        PluginTincOperator::instance().start_tinc()?;
        if let Some(handle) = PluginTincOperator::instance().get_tinc_handle() {
            return Ok(handle);
        }
        Err(TincOperatorError::StartTincError)
    }

    /// 添加子设备
    pub fn add_hosts(&self, host_name: &str, pub_key: &str) -> Result<()> {
        PluginTincOperator::instance().add_hosts(host_name, pub_key)
    }

    /// 获取子设备公钥
//    pub fn get_host_pub_key(&self, host_name:&str) -> Result<String> {
//        PluginTincOperator::instance()
//            .get_host_pub_key(host_name)
//            .map_err(Error::TincOperatorError)
//    }

    /// 从pub_key文件读取pub_key
    pub fn get_local_pub_key(&self) -> Result<String> {
        PluginTincOperator::instance().get_local_pub_key()
    }

    /// 修改本地公钥
    pub fn set_local_pub_key(&mut self, pub_key: &str) -> Result<()> {
        PluginTincOperator::instance().set_local_pub_key(pub_key)
    }

    /// 获取本地tinc虚拟ip
    pub fn get_local_vip(&self) -> Result<String> {
        PluginTincOperator::instance().get_local_vip()
    }

    /// 添加hosts文件
    /// if is_proxy{ 文件名=proxy_10_253_x_x }
    /// else { 文件名=虚拟ip后三位b_c_d }
    fn set_hosts(&self,
                 is_proxy: bool,
                 ip: &str,
                 pubkey: &str) -> Result<()> {
        PluginTincOperator::instance().set_hosts(is_proxy, ip, pubkey)
    }

    /// set_tinc_conf_file
    pub fn set_info_to_local(&self, tinc_info: &TincInfo) -> Result<()> {
        PluginTincOperator::instance().set_info_to_local(tinc_info)
    }

    /// Load local tinc config file vpnserver for tinc vip and pub_key.
    /// Success return true.
    pub fn load_local(&mut self, tinc_home: &str) -> io::Result<TincInfo> {
        let mut tinc_info = TincInfo::new();
        {
            let mut res = String::new();
            let mut _file = fs::File::open(tinc_home.to_string() + PUB_KEY_FILENAME)?;
            _file.read_to_string(&mut res)?;
            tinc_info.pub_key = res.clone();
        }
        {
            if let Ok(vip_str) = self.get_local_vip() {
                if let Ok(vip) = IpAddr::from_str(&vip_str) {
                    tinc_info.vip = vip;
                    return Ok(tinc_info);
                }

            }
        }
        return Err(io::Error::new(io::ErrorKind::InvalidData, "tinc config file error"));
    }

    /// check info and save change
    pub fn check_info(&self, tinc_info: &TincInfo) -> Result<()> {
        PluginTincOperator::instance().check_info(tinc_info)
    }
}