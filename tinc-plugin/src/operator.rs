#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write, Read};
use std::sync::Mutex;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use duct;
use openssl::rsa::Rsa;

use crate::{TincInfo, TincRunMode};

/// Results from fallible operations on the Tinc tunnel.
pub type Result<T> = std::result::Result<T, Error>;

static mut EL: *mut TincOperator = 0 as *mut _;

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

const PID_FILENAME: &str = "tinc.pid";

const TINC_AUTH_PATH: &str = "auth/";

const TINC_AUTH_FILENAME: &str = "auth.txt";

/// Errors that can happen when using the Tinc tunnel.
#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Unable to start
    #[error(display = "duct can not start tinc")]
    NeverInitOperator,

    /// Unable to start
    #[error(display = "duct can not start tinc")]
    StartTincError,

    #[error(display = "duct can not start tinc")]
    AnotherTincRunning,

    /// Unable to stop
    #[error(display = "duct can not stop tinc")]
    StopTincError,

    /// tinc process not exist
    #[error(display = "tinc pidfile not exist")]
    PidfileNotExist,

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
    IoError(String),

    /// No wan dev
    #[error(display = "No wan dev")]
    NoWanDev,


    /// Address loaded from file is invalid
    #[error(display = "Address loaded from file is invalid")]
    ParseLocalVipError(#[error(cause)] std::net::AddrParseError),
}

/// Tinc operator
pub struct TincOperator {
    tinc_home:              String,
    tinc_handle:            Option<duct::Handle>,
    mutex:                  Mutex<i32>,
    mode:                   TincRunMode,
}

impl TincOperator {
    /// 获取tinc home dir 创建tinc操作。
    pub fn new(tinc_home: &str, mode: TincRunMode) {
        let operator = TincOperator {
            tinc_home:      tinc_home.to_string() + "/tinc/",
            tinc_handle:    None,
            mutex:          Mutex::new(0),
            mode,
        };

        unsafe {
            EL = Box::into_raw(Box::new(operator));
        }
    }

    pub fn instance() ->  &'static mut Self {
        unsafe {
            if EL == 0 as *mut _ {
                panic!("Get tinc Operator instance, before init");
            }
            &mut *EL
        }
    }

    pub fn is_inited() -> bool {
        unsafe {
            if EL == 0 as *mut _ {
                return false;
            }
        }
        return true;
    }

    /// 启动tinc 返回duct::handle
    pub fn start_tinc(&mut self) -> Result<()> {
        let conf_tinc_home = "--config=".to_string() + &self.tinc_home;
        let conf_pidfile = "--pidfile=".to_string() + &self.tinc_home + "/tinc.pid";
        let argument: Vec<&str> = vec![
            &conf_tinc_home,
            &conf_pidfile,
            "--no-detach",
        ];
        let duct_handle: duct::Expression = duct::cmd(
            OsString::from(self.tinc_home.to_string() + "/" + TINC_BIN_FILENAME),
            argument).unchecked();
        self.tinc_handle = Some(duct_handle.stderr_null().stdout_null().start()
            .map_err(|e| {
                log::error!("StartTincError {:?}", e.to_string());
                Error::StartTincError
            })?
        );
        Ok(())
    }

    pub fn get_tinc_handle(&mut self) -> Option<duct::Handle> {
        self.tinc_handle.take()
    }

    pub fn stop_tinc(&mut self) -> Result<()> {
        if let Some(child) = &self.tinc_handle {
            child.kill().map_err(|_|Error::StopTincError)?
        }
        self.tinc_handle = None;
        Ok(())
    }

    pub fn check_tinc_status(&mut self) -> Result<()> {
        if let Some(child) = &self.tinc_handle {
            let out = child.try_wait()
                .map_err(|_|Error::TincNotExist)?;

            if let None = out {
                return Ok(());
            }
        }
        Err(Error::TincNotExist)
    }

    pub fn restart_tinc(&mut self) -> Result<()> {
        if let Ok(_) = self.check_tinc_status() {
            self.stop_tinc()?;
        }
        self.start_tinc()
    }

    /// 根据IP地址获取文件名
    pub fn get_filename_by_ip(is_proxy: bool, ip: &str) -> String {
        let splits = ip.split(".").collect::<Vec<&str>>();
        let mut filename = String::new();
        if is_proxy {
            filename = "proxy".to_string() + "_";
            filename.push_str(splits[0]);
            filename.push_str("_");
        }
        filename.push_str(splits[1]);
        filename.push_str("_");
        filename.push_str(splits[2]);
        filename.push_str("_");
        filename.push_str(splits[3]);
        filename
    }

    /// 添加子设备
    pub fn add_hosts(&self, host_name: &str, pub_key: &str) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
        let mut file = fs::File::create(
            format!("{}/{}/{}", self.tinc_home.clone() , "hosts", host_name))
            .map_err(|e|Error::FileCreateError(e.to_string()))?;
        file.write_all(pub_key.as_bytes())
            .map_err(|e|Error::FileCreateError(e.to_string()))?;
        drop(file);
        Ok(())
    }

    /// openssl Rsa 创建2048位密钥对, 并存放到tinc配置文件中
    pub fn create_pub_key(&self) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
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
                        self.tinc_home.to_string() + PUB_KEY_FILENAME)
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
    pub fn get_local_pub_key(&self) -> Result<String> {
        let _guard = self.mutex.lock().unwrap();
        let path = self.tinc_home.clone() + PUB_KEY_FILENAME;
        let mut file =  fs::File::open(path.clone())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        Ok(buf)
    }

    /// 修改本地公钥
    pub fn set_local_pub_key(&mut self, pub_key: &str) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
        let path = self.tinc_home.clone() + PUB_KEY_FILENAME;
        let mut file =  fs::File::create(path.clone())
            .map_err(|_|Error::CreatePubKeyError)?;
        file.write(pub_key.as_bytes())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        return Ok(());
    }

    /// 获取本地tinc虚拟ip
    pub fn get_local_vip(&self) -> Result<IpAddr> {
        let _guard = self.mutex.lock().unwrap();
        let mut out = String::new();

        let path = self.tinc_home.clone() + TINC_UP_FILENAME;
        let mut file = fs::File::open(path.clone())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;

        let mut res = String::new();
        file.read_to_string(&mut res)
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        #[cfg(unix)]
            let res: Vec<&str> = res.split("vpngw=").collect();
        #[cfg(windows)]
            let res: Vec<&str> = res.split("addr=").collect();
        if res.len() > 1 {
            let res = res[1].to_string();
            #[cfg(unix)]
            let res: Vec<&str> = res.split("\n").collect();
            #[cfg(windows)]
            let res: Vec<&str> = res.split(" mask").collect();
            if res.len() > 1 {
                out = res[0].to_string();
            }
        }
        Ok(IpAddr::from(Ipv4Addr::from_str(&out).map_err(Error::ParseLocalVipError)?))
    }

    /// 通过Info修改tinc.conf
    fn set_tinc_conf_file(&self, tinc_info: &TincInfo) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();

        let (is_proxy, name_ip) = match self.mode {
            TincRunMode::Proxy => (true, tinc_info.ip.clone()),
            TincRunMode::Client => (false, tinc_info.vip.clone()),
        };

        let name = Self::get_filename_by_ip(is_proxy,
                                            &name_ip.to_string());

        let mut connect_to: Vec<String> = vec![];
        for online_proxy in tinc_info.connect_to.clone() {
            let online_proxy_name = Self::get_filename_by_ip(true,
                                                             &online_proxy.ip.to_string());
            connect_to.push(online_proxy_name);
        }


        let mut buf_connect_to = String::new();
        for other in connect_to {
            let buf = "ConnectTo = ".to_string() + &other + "\n";
            buf_connect_to += &buf;
        }
        let mut buf :String = "Name = ".to_string() + &name + "\n"
        + &buf_connect_to
        + "DeviceType=tap\n\
            Mode=switch\n\
            Interface=dnet\n\
            BindToAddress = * 50069\n\
            ProcessPriority = high\n\
            PingTimeout=10";
        #[cfg(unix)]
        {
             buf = buf + "Device = /dev/net/tun\n";
        }

        let path = self.tinc_home.clone() + "/tinc.conf";
        let mut file = fs::File::create(path.clone())
            .map_err(|e|Error::FileCreateError(e.to_string()))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        return Ok(());
    }

    /// 检查info中的配置, 并与实际运行的tinc配置对比, 如果不同修改tinc配置,
    /// 如果自己的vip修改,重启tinc
//    pub fn check_info(&mut self, tinc_info: &TincInfo) -> Result<()> {
//        let mut need_restart = false;
//        {
//            let file_vip = self.get_local_vip()?;
//            if file_vip != tinc_info.vip {
//                log::debug!("tinc operator check_info local {}, remote {}",
//                       file_vip,
//                       tinc_info.vip.to_string());
//
//                self.set_tinc_up(&tinc_info)?;
//
//                self.set_hosts(true,
//                                   &tinc_info.ip.to_string(),
//                                   &tinc_info.pub_key)?;
//
//                need_restart = true;
//            }
//        }
//        {
//            for online_proxy in tinc_info.connect_to.clone() {
//                self.set_hosts(true,
//                                   &online_proxy.ip.to_string(),
//                                   &online_proxy.pubkey)?;
//            }
//        }
//
//        self.check_self_hosts_file(&self.tinc_home, &tinc_info)?;
//        self.set_hosts(
//            true,
//            &tinc_info.ip.to_string(),
//            &tinc_info.pub_key)?;
//
//        if need_restart {
//            self.set_tinc_conf_file(&tinc_info)?;
//            self.stop_tinc()?;
//        }
//        return Ok(());
//    }

    /// 添加hosts文件
    /// if is_proxy{ 文件名=proxy_10_253_x_x }
    /// else { 文件名=虚拟ip后三位b_c_d }
    pub fn set_hosts(&self,
                     is_proxy: bool,
                     ip: &str,
                     pubkey: &str)
        -> Result<()>
    {
        let _guard = self.mutex.lock().unwrap();
        {
            let mut buf;
            if is_proxy {
                buf = "Address=".to_string() + ip + "\n"
                    + "Port=50069\n"
                    + pubkey;
            }
            else {
                buf = pubkey.to_string();
            }
            let file_name = Self::get_filename_by_ip(is_proxy, ip);

            let path = self.tinc_home.clone() + "/hosts/" + &file_name;
            let mut file = fs::File::create(path.clone())
                .map_err(|e|Error::FileCreateError(e.to_string()))?;
            file.write(buf.as_bytes())
                .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        }
        Ok(())
    }

    /// 检测自身hosts文件,是否正确
    pub fn check_self_hosts_file(&self, tinc_home: &str, tinc_info: &TincInfo) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
        let ip = tinc_info.ip.to_string();

        let is_proxy = match self.mode {
            TincRunMode::Proxy => true,
            TincRunMode::Client => false,
        };
        let filename = Self::get_filename_by_ip(is_proxy, &ip);

        let path = tinc_home.to_string()
            + "/hosts/"
            + "proxy_"
            + &filename;
        fs::File::create(path.clone())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        Ok(())
    }

    /// Load local tinc config file vpnserver for tinc vip and pub_key.
    /// Success return true.
//    pub fn load_local(&mut self, tinc_home: &str) -> io::Result<TincInfo> {
//        let _guard = self.mutex.lock().unwrap();
//        let mut tinc_info = TincInfo::new();
//        {
//            let mut res = String::new();
//            let mut _file = fs::File::open(tinc_home.to_string() + PUB_KEY_FILENAME)?;
//            _file.read_to_string(&mut res).map_err(Error::IoError)?;
//            tinc_info.pub_key = res.clone();
//        }
//        {
//            tinc_info.vip = self.get_local_vip()?
//
//        }
//        return Err(Error::L);
//    }

    pub fn set_info_to_local(&mut self, info: &TincInfo) -> Result<()> {
        self.set_tinc_conf_file(info)?;
        let is_proxy = match self.mode {
            TincRunMode::Proxy => true,
            TincRunMode::Client => false,
        };

        self.set_tinc_up(&info)?;
        self.set_tinc_down()?;
        self.set_host_up()?;
        self.set_host_down()?;

        for online_proxy in info.connect_to.clone() {
            self.set_hosts(true,
                           &online_proxy.ip.to_string(),
                           &online_proxy.pubkey)?;
        };
        self.set_hosts(is_proxy, &info.vip.to_string(), &info.pub_key)
    }

    fn set_tinc_up(&self, tinc_info: &TincInfo) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();

        let mut buf;

        let netmask = match self.mode {
            TincRunMode::Proxy => "255.0.0.0",
            TincRunMode::Client => "255.255.255.255",
        };

        #[cfg(unix)]
        {
            buf = "#! /bin/sh\n\
            dev=dnet\n\
            vpngw=".to_string() + &tinc_info.vip.to_string() + "\n" +
            "ifconfig ${dev} ${vpngw} netmask " + netmask;

            buf = buf + "\n" + &self.tinc_home + "/tinc-report -u";

            if TincRunMode::Client == self.mode {
                buf = buf + "\n"
                    + "route add -host " + &tinc_info.connect_to[0].ip.to_string() + " gw _gateway";
                buf = buf + "\n"
                    + "route add -host 10.255.255.254 dev dnet";
                buf = buf + "\n"
                    + "route add default gw " + &tinc_info.connect_to[0].vip.to_string();
            }
        }

        #[cfg(windows)]
        {
            buf = "netsh interface ipv4 set address name=\"dnet\" source=static addr=".to_string() +
                &tinc_info.vip.to_string() + " mask=" + netmask;

            buf = buf + "\r\n" + &self.tinc_home + "/tinc-report.exe -u";
        }

        let path = self.tinc_home.clone() + "/" + TINC_UP_FILENAME;
        let mut file = fs::File::create(path.clone())
            .map_err(|e|Error::FileCreateError(e.to_string()))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        Ok(())
    }

    fn set_tinc_down(&self) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -d");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -d";

        let path = self.tinc_home.clone() + "/" + TINC_DOWN_FILENAME;
        let mut file = fs::File::create(path.clone())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        Ok(())
    }

    fn set_host_up(&self) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -hu ${NODE}");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -hu ${NODE}";

        let path = self.tinc_home.clone() + "/" + HOST_UP_FILENAME;
        let mut file = fs::File::create(path.clone())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        Ok(())
    }

    fn set_host_down(&self) -> Result<()> {
        let _guard = self.mutex.lock().unwrap();
        #[cfg(windows)]
            let buf = &(self.tinc_home.to_string() + "/tinc-report.exe -hd ${NODE}");
        #[cfg(unix)]
            let buf = "#!/bin/sh\n".to_string() + &self.tinc_home + "/tinc-report -hd ${NODE}";

        let path = self.tinc_home.clone() + "/" + HOST_UP_FILENAME;
        let mut file = fs::File::create(path.clone())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        file.write(buf.as_bytes())
            .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
        Ok(())
    }

    /// 获取子设备公钥
    pub fn get_host_pub_key(&self, host_name: &str) -> Result<String> {
        let _guard = self.mutex.lock().unwrap();
        let file_path = &(self.tinc_home.to_string() + "/hosts/" + host_name);
        let mut file = fs::File::open(file_path)
            .map_err(|_| Error::FileNotExist(file_path.to_string()))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|_| Error::FileNotExist(file_path.to_string()))?;
        Ok(contents)
    }

    // 写TINC_AUTH_PATH/TINC_AUTH_FILENAME(auth/auth.txt),用于tinc reporter C程序
    // TODO 去除C上报tinc上线信息流程,以及去掉auth/auth.txt.
//    fn write_auth_file(&self,
//                           server_url:  &str,
//                           info:        &TincInfo,
//    ) -> Result<()> {
//        let path = self.tinc_home.to_string() + TINC_AUTH_PATH;
//        let auth_dir = path::PathBuf::from(&(path));
//        if !path::Path::new(&auth_dir).is_dir() {
//            fs::create_dir_all(&auth_dir)
//                .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
//        }
//
//        let file_path_buf = auth_dir.join(TINC_AUTH_FILENAME);
//        let file_path = path::Path::new(&file_path_buf);
//
//        if let Some(file_str) = file_path.to_str() {
//            let path = file_str.to_string();
//            let mut file = fs::File::create(path.clone())
//                .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
//            let auth_info = AuthInfo::load(server_url, info);
//            file.write(auth_info.to_json_str().as_bytes())
//                .map_err(|e|Error::IoError(path.clone() + " " + &e.to_string()))?;
//        }
//
//        return Ok(());
//    }

}

#[cfg(windows)]
fn get_vnic_index() -> Option<String> {
    let res = duct::cmd!(
    "wmic",
    "nic",
    "where",
    "netconnectionid = 'dnet'",
    "get",
    "index");
    if let Ok(mut out) = res.read() {
        out = out.replace("Index  \r\r\n", "").replace(" ", "");
        return Some(out);
    }
    None
}
