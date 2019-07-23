#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]

use std::fs;
use std::io::{self, Write, Read};
use std::ffi::OsString;
#[cfg(unix)]
use std::net::IpAddr;
use std::str::FromStr;

use openssl::rsa::Rsa;

use tinc_plugin::TincInfo;

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

/// Errors that can happen when using the Tinc tunnel.
#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Unable to start
    #[error(display = "duct can not start tinc")]
    StartTincError,

    /// Unable to stop
    #[error(display = "duct can not stop tinc")]
    StopTincError,

    /// tinc process not exist
    #[error(display = "tinc process not exist")]
    TincNotExist,

    /// tinc host file not exist
    #[error(display = "tinc host file not exist")]
    FileNotExist(String),

    /// Failed create file
    #[error(display = "Failed create file")]
    FileCreateError(String),

    /// Tinc can't create key pair
    #[error(display = "Tinc can't create key pair")]
    CreatePubKeyError,

    /// Invalid tinc info
    #[error(display = "Invalid tinc info")]
    TincInfoError,

    /// Error while running "ip route".
    #[error(display = "Error while running \"ip route\"")]
    FailedToRunIp(#[error(cause)] io::Error),

    /// Io error
    #[error(display = "Io error")]
    IoError(#[error(cause)] io::Error),

    /// No wan dev
    #[error(display = "No wan dev")]
    NoWanDev,
}
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
        let conf_tinc_home = "--config=".to_string() + &self.tinc_home;
        let conf_pidfile = "--pidfile=".to_string() + &self.tinc_home + "/tinc.pid";
        let argument: Vec<&str> = vec![
            &conf_tinc_home,
            &conf_pidfile,
            "--no-detach",
        ];
        let tinc_handle: duct::Expression = duct::cmd(
            OsString::from(self.tinc_home.to_string() + "/" + TINC_BIN_FILENAME),
            argument).unchecked();
        tinc_handle.stderr_null().stdout_null().start()
            .map_err(|e| {
                log::error!("StartTincError {:?}", e.to_string());
                Error::StartTincError
            })
    }

//    pub fn stop_tinc(&mut self) -> Result<()> {
//        if let Some(child) = &self.tinc_handle {
//            child.kill().map_err(|_|Error::StopTincError)?
//        }
//        self.tinc_handle = None;
//        Ok(())
//    }
//
//    pub fn check_tinc_status(&mut self) -> Result<()> {
//        if let Some(child) = &self.tinc_handle {
//            let out = child.try_wait()
//                .map_err(|_|Error::TincNotExist)?;
//
//            if let None = out {
//                return Ok(());
//            }
//        }
//        Err(Error::TincNotExist)
//    }
//
//    pub fn restart_tinc(&mut self) -> Result<()> {
//        if let Ok(_) = self.check_tinc_status() {
//            self.stop_tinc()?;
//        }
//        self.start_tinc()
//    }

    /// 根据IP地址获取文件名
    pub fn get_filename_by_ip(&self, ip: &str) -> String {
        let splits = ip.split(".").collect::<Vec<&str>>();
        let mut filename = String::new();
        filename.push_str(splits[0]);
        filename.push_str("_");
        filename.push_str(splits[1]);
        filename.push_str("_");
        filename.push_str(splits[2]);
        filename.push_str("_");
        filename.push_str(splits[3]);
        filename
    }

    /// 根据IP地址获取文件名
    pub fn get_client_filename_by_virtual_ip(&self, virtual_ip: &str) -> String {
        let splits = virtual_ip.split(".").collect::<Vec<&str>>();
        let mut filename = String::new();
        filename.push_str(splits[1]);
        filename.push_str("_");
        filename.push_str(splits[2]);
        filename.push_str("_");
        filename.push_str(splits[3]);
        filename
    }

    /// 添加子设备
    pub fn add_hosts(&self, host_name: &str, pub_key: &str) -> Result<()> {
        let mut file = fs::File::create(
            format!("{}/{}/{}", self.tinc_home.clone() , "hosts", host_name))
            .map_err(|e|Error::FileCreateError(e.to_string()))?;
        file.write_all(pub_key.as_bytes())
            .map_err(|e|Error::FileCreateError(e.to_string()))?;
        drop(file);
        Ok(())
    }

    /// 获取子设备公钥
    pub fn get_host_pub_key(&self, host_name:&str) -> Result<String> {
        let file_path =  &(self.tinc_home.to_string() + "/hosts/" + host_name);
        let mut file = fs::File::open(file_path)
            .map_err(|_|Error::FileNotExist(file_path.to_string()))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|_|Error::FileNotExist(file_path.to_string()))?;
        Ok(contents)
    }

    /// openssl Rsa 创建2048位密钥对, 并存放到tinc配置文件中
    pub fn create_pub_key(&self) -> Result<()> {
        let mut write_priv_key_ok = false;
        if let Ok(key) = Rsa::generate(2048) {
            if let Ok(priv_key) = key.private_key_to_pem() {
                if let Ok(priv_key) = String::from_utf8(priv_key) {
                    let mut file = fs::File::create(
                        self.tinc_home.to_string() + PRIV_KEY_FILENAME)
                        .map_err(|e|Error::FileCreateError(e.to_string()))?;
                    file.write_all(priv_key.as_bytes())
                        .map_err(|_|Error::CreatePubKeyError)?;
                    drop(file);

                    write_priv_key_ok = true;
                }
            }
            if let Ok(pub_key) = key.public_key_to_pem() {
                if let Ok(pub_key) = String::from_utf8(pub_key) {
                    let mut file = fs::File::create(
                        self.tinc_home.to_string() + "pub_key.pem")
                        .map_err(|e|Error::FileCreateError(e.to_string()))?;
                    file.write_all(pub_key.as_bytes())
                        .map_err(|_|Error::CreatePubKeyError)?;
                    drop(file);
                    if write_priv_key_ok {
                        return Ok(());
                    }
                }
            }
        }
        Err(Error::CreatePubKeyError)
    }

    /// 从pub_key文件读取pub_key
    pub fn get_pub_key(&self) -> Result<String> {
        let mut file =  fs::File::open(self.tinc_home.clone() + PUB_KEY_FILENAME)
            .map_err(|e|Error::FileNotExist(e.to_string()))?;
        let buf = &mut [0; 2048];
        file.read(buf)
            .map_err(|e|Error::FileNotExist(e.to_string()))?;
        Ok(String::from_utf8_lossy(buf).to_string())
    }

    /// 修改本地公钥
    pub fn set_pub_key(&mut self, pub_key: &str) -> Result<()> {
        let mut file = fs::File::create(self.tinc_home.clone() + PUB_KEY_FILENAME)
            .map_err(|_|Error::CreatePubKeyError)?;
        file.write(pub_key.as_bytes())
            .map_err(|e|Error::FileNotExist(e.to_string()))?;
        return Ok(());
    }

    /// 获取本地tinc虚拟ip
    pub fn get_vip(&self) -> Result<String> {
        let mut out = String::new();
        let mut file = fs::File::open(self.tinc_home.clone() + TINC_UP_FILENAME)
            .map_err(|e|Error::FileNotExist(e.to_string()))?;
        let res = &mut [0; 2048];
        file.read(res).map_err(Error::IoError)?;
        let res = String::from_utf8_lossy(res);
        #[cfg(unix)]
            let res: Vec<&str> = res.split("vpngw=").collect();
        #[cfg(windows)]
            let res: Vec<&str> = res.split("addr=\"").collect();
        if res.len() > 1 {
            let res = res[1].to_string();
            #[cfg(unix)]
                let res: Vec<&str> = res.split("\n").collect();
            #[cfg(windows)]
                let res: Vec<&str> = res.split("\" ").collect();
            if res.len() > 1 {
                out = res[0].to_string();
            }
        }
        return Ok(out);
    }

    /// 添加hosts文件
    /// if is_proxy{ 文件名=proxy_10_253_x_x }
    /// else { 文件名=虚拟ip后三位b_c_d }
    fn set_hosts(&self,
                 is_proxy: bool,
                 ip: &str,
                 pubkey: &str) -> Result<()> {
        {
            let mut proxy_or_client = "proxy".to_string();
            if !is_proxy {
                proxy_or_client = "CLIENT".to_string();
            }
            let buf = "Address=".to_string()
                + ip
                + "\n\
                "
                + pubkey
                + "Port=50069\n\
                ";
            let file_name = proxy_or_client.to_string()
                + "_" + &self.get_filename_by_ip(ip);
            let mut file = fs::File::create(self.tinc_home.clone() + "/hosts/" + &file_name)
                .map_err(|e|Error::FileCreateError(e.to_string()))?;
            file.write(buf.as_bytes())
                .map_err(|e|Error::FileNotExist(e.to_string()))?;
        }
        Ok(())
    }

    /// 修改tinc虚拟ip
    fn change_vip(&self, vip: String) -> Result<()> {
        {
            let buf = "#! /bin/sh\n\
            dev=tun0\n\
            vpngw=".to_string() + &vip + "\n\
            echo 1 > /proc/sys/net/ipv4/ip_forward\n\
            ifconfig ${dev} ${vpngw} netmask 255.0.0.0\n\
            iptables -t nat -F\n\
            exit 0";
            let mut file = fs::File::create(self.tinc_home.clone() + "/tinc-up")
                .map_err(|e|Error::FileCreateError(e.to_string()))?;
            file.write(buf.as_bytes())
                .map_err(|e|Error::FileNotExist(e.to_string()))?;
        }
        Ok(())
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
            if let Ok(vip_str) = self.get_vip() {
                if let Ok(vip) = IpAddr::from_str(&vip_str) {
                    tinc_info.vip = vip;
                    return Ok(tinc_info);
                }

            }
        }
        return Err(io::Error::new(io::ErrorKind::InvalidData, "tinc config file error"));
    }

    fn set_tinc_up(&self) -> Result<()> {
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -u");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -u";

        let mut file = fs::File::create(self.tinc_home.clone() + "/" + TINC_UP_FILENAME)
            .map_err(|e|Error::IoError(e))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(e))?;
        Ok(())
    }

    fn set_tinc_down(&self) -> Result<()> {
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -d");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -d";

        let mut file = fs::File::create(self.tinc_home.clone() + "/" + TINC_DOWN_FILENAME)
            .map_err(|e|Error::IoError(e))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(e))?;
        Ok(())
    }

    fn set_host_up(&self) -> Result<()> {
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -hu ${NODE}");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -hu ${NODE}";

        let mut file = fs::File::create(self.tinc_home.clone() + "/" + HOST_UP_FILENAME)
            .map_err(|e|Error::IoError(e))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(e))?;
        Ok(())
    }

    fn set_host_down(&self) -> Result<()> {
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -hd ${NODE}");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -hd ${NODE}";

        let mut file = fs::File::create(self.tinc_home.clone() + "/" + HOST_UP_FILENAME)
            .map_err(|e|Error::IoError(e))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(e))?;
        Ok(())
    }
}