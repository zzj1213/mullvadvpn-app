use std::path;
use std::fs;
use std::io::{self, Write, Read};
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "duct can not start tinc")]
    StartTincError,

    #[error(display = "duct can not stop tinc")]
    StopTincError,

    #[error(display = "tinc process not exist")]
    TincNotExist,

    #[error(display = "tinc host file not exist")]
    FileNotExist(String),

    #[error(display = "tinc can't create key pair")]
    CreatePubKeyError,
}
const TINC_AUTH_PATH: &str = "auth/";
const TINC_AUTH_FILENAME: &str = "auth.txt";

pub struct TincCommand {
    tinc_home:          String,
    pub_key_path:       String,
    tinc_handle:        Option<duct::Handle>,
}
impl TincCommand {
    pub fn new(tinc_home: String) -> Self {
        let pub_key_path = tinc_home.clone() + "/key/rsa_key.pub";
        TincCommand {
            tinc_home,
            pub_key_path,
            tinc_handle: None,
        }
    }

    pub fn start_tinc(&mut self) -> Result<()> {
        let conf_tinc_home = "--config=".to_string() + &self.tinc_home;
        let conf_pidfile = "--pidfile=".to_string() + &self.tinc_home + "/tinc.pid";
        let argument: Vec<&str> = vec![
            &conf_tinc_home,
            &conf_pidfile,
            "--no-detach",
        ];
        let tinc_handle: duct::Expression = duct::cmd(
            OsString::from(self.tinc_home.to_string() + "/tincd"),
            argument).unchecked();
        self.tinc_handle = Some(
            tinc_handle.stderr_null().stdout_null().start()
                .map_err(|e| Error::StartTincError(e))?
        );
        Ok(())
    }

    pub fn stop_tinc(&mut self) -> Result<()> {
        if let Some(child) = &self.tinc_handle {
            child.kill().map_err(Error::StopTincError)?
        }
        self.tinc_handle = None;
        Ok(())
    }

    pub fn check_tinc_status(&mut self) -> Result<()> {
        if let Some(child) = &self.tinc_handle {
            let out = child.try_wait()
                .map_err(Error::TincNotExist)?;

            if let None = x {
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
    pub fn add_hosts(&self, host_name: &str, pub_key: &str) -> bool {
        let mut file = fs::File::create(format!("{}/{}/{}", self.tinc_home.clone() , "hosts", host_name)).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        drop(file);
        true
    }

    /// 获取子设备公钥
    pub fn get_host_pub_key(&self, host_name:&str) -> Result<String> {
        let file_path =  &self.tinc_home.to_string() + "/hosts/" + host_name;
        let mut file = fs::File::open(file_path)
            .map_err(Error::FileNotExist(file_path))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(Error::FileNotExist(file_path))?;
        Ok(contents)
    }

    #[cfg(unix)]
    pub fn create_pub_key(&self) -> Result<()> {
        let mut write_priv_key_ok = false;
        if let Ok(key) = Rsa::generate(2048) {
            if let Ok(priv_key) = key.private_key_to_pem() {
                if let Ok(priv_key) = String::from_utf8(priv_key) {
                    if let Ok(mut file) = fs::File::create(
                        self.tinc_home.to_string() + "priv_key.pem") {
                        file.write_all(priv_key.as_bytes())?;
                        drop(file);

                        write_priv_key_ok = true;
                    }
                }
            }
            if let Ok(pub_key) = key.public_key_to_pem() {
                if let Ok(pub_key) = String::from_utf8(pub_key) {
                    if let Ok(mut file) = fs::File::create(
                        self.tinc_home.to_string() + "pub_key.pem") {
                        file.write_all(pub_key.as_bytes())?;
                        drop(file);
                        if write_priv_key_ok {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err(Error::CreatePubKeyError)
    }

    /// 从pub_key文件读取pub_key
    pub fn get_pub_key(&self) -> Result<String> {
        let file =  fs::File::open(self.tinc_home.clone() + &self.pub_key_path)
            ;
        file.read()
    }

    pub fn set_pub_key(&mut self, pub_key: &str) -> bool {
        let file = File::new(self.tinc_home.clone() + &self.pub_key_path);
        file.write(pub_key.to_string())
    }

    pub fn get_vip(&self) -> String {
        let mut out = String::new();

        let file = File::new(self.tinc_home.clone() + "tinc-up");
        let res = file.read();
        let res: Vec<&str> = res.split("vpngw=").collect();
        if res.len() > 1 {
            let res = res[1].to_string();
            let res: Vec<&str> = res.split("\n").collect();
            if res.len() > 1 {
                out = res[0].to_string();
            }
        }
        return out;
    }

    fn set_tinc_conf_file(&self, info: &Info) -> bool {
        let name = "proxy".to_string() + "_"
            + &self.get_filename_by_ip(&info.proxy_info.proxy_ip);

        let mut connect_to: Vec<String> = vec![];
        for online_proxy in info.proxy_info.online_porxy.clone() {
            let online_proxy_name = "proxy".to_string() + "_"
                + &self.get_filename_by_ip(&online_proxy.ip.to_string());
            connect_to.push(online_proxy_name);
        }


        let mut buf_connect_to = String::new();
        for other in connect_to {
            let buf = "ConnectTo = ".to_string() + &other + "\n\
            ";
            buf_connect_to += &buf;
        }
        let buf = "Name = ".to_string() + &name + "\n\
        " + &buf_connect_to
            + "DeviceType=tap\n\
        Mode=switch\n\
        Interface=tun0\n\
        Device = /dev/net/tun\n\
        BindToAddress = * 50069\n\
        ProcessPriority = high\n\
        PingTimeout=10";
        let file = File::new(self.tinc_home.clone() + "/tinc.conf");
        file.write(buf.to_string())
    }

    /// 检查info中的配置, 并与实际运行的tinc配置对比, 如果不同修改tinc配置,
    /// 如果自己的vip修改,重启tinc
    pub fn check_info(&mut self, info: &Info) -> bool {
        let mut need_restart = false;
        {
            let file_vip = self.get_vip();
            if file_vip != info.tinc_info.vip.to_string() {
                debug!("tinc operator check_info local {}, remote {}",
                       file_vip,
                       info.tinc_info.vip.to_string());

                if !self.change_vip(info.tinc_info.vip.to_string()) {
                    return false;
                }

                if !self.set_hosts(true,
                                   &info.proxy_info.proxy_ip.to_string(),
                                   &info.tinc_info.pub_key,
                ) {
                    return false;
                }

                need_restart = true;
            }
        }
        {
            for online_proxy in info.proxy_info.online_porxy.clone() {
                if !self.set_hosts(true,
                                   &online_proxy.ip.to_string(),
                                   &online_proxy.pubkey,
                ) {
                    return false;
                }
            }
        }

        if self.check_self_hosts_file(self.tinc_home.borrow(), &info) {
            self.set_hosts(
                true,
                &info.proxy_info.proxy_ip,
                &info.tinc_info.pub_key);
        }

        if need_restart {
            self.set_tinc_conf_file(&info);
            self.restart_tinc();
        }
        return true;
    }

    fn set_hosts(&self,
                 is_proxy: bool,
                 ip: &str,
                 pubkey: &str) -> bool {
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
            let file = File::new(self.tinc_home.clone() + "/hosts/" + &file_name);
            if !file.write(buf.to_string()) {
                return false;
            }
        }
        true
    }

    /// 修改tinc虚拟ip
    fn change_vip(&self, vip: String) -> bool {
        let wan_name = match get_wan_name() {
            Some(x) => x,
            None => {
                warn!("change_vip get dev wan failed, use defualt.");
                "eth0".to_string()
            }
        };
        {
            let buf = "#! /bin/sh\n\
            dev=tun0\n\
            vpngw=".to_string() + &vip + "\n\
            echo 1 > /proc/sys/net/ipv4/ip_forward\n\
            ifconfig ${dev} ${vpngw} netmask 255.0.0.0\n\
            iptables -t nat -F\n\
            iptables -t nat -A POSTROUTING -s ${vpngw}/8 -o "
                + &wan_name
                + " -j MASQUERADE\n\
            exit 0";
            let file = File::new(self.tinc_home.clone() + "/tinc-up");
            if !file.write(buf.to_string()) {
                return false;
            }
        }
        true
    }

    pub fn check_self_hosts_file(&self, tinc_home: &str, info: &Info) -> bool {
        let ip = info.proxy_info.proxy_ip.clone();
        let filename = self.get_filename_by_ip(&ip);
        let file = File::new(
            tinc_home.to_string()
                + "/hosts/"
                + "proxy_"
                + &filename
        );
        file.file_exists()
    }

    pub fn write_auth_file(&self,
                           server_url:  &str,
                           info:        &Info,
    ) -> bool {
        let auth_dir = path::PathBuf::from(&(self.tinc_home.to_string() + TINC_AUTH_PATH));
        if !path::Path::new(&auth_dir).is_dir() {
            if let Err(_) = fs::create_dir_all(&auth_dir) {
                return false;
            }
        }

        let file_path_buf = auth_dir.join(TINC_AUTH_FILENAME);
        let file_path = path::Path::new(&file_path_buf);

        let permissions = PermissionsExt::from_mode(0o755);
        if file_path.is_file() {
            if let Ok(file) = fs::File::open(&file_path) {
                if let Err(_) = file.set_permissions(permissions) {
                    return false;
                }
            }
            else {
                return false;
            }
        }
        else {
            if let Ok(file) = fs::File::create(&file_path) {
                if let Err(_) = file.set_permissions(permissions) {
                    return false;
                }
            }
            else {
                return false;
            }
        }
        if let Some(file_str) = file_path.to_str() {
            let file = File::new(file_str.to_string());
            let auth_info = AuthInfo::load(server_url, info);
            file.write(auth_info.to_json_str());
        }
        else {
            return false;
        }
        return true;
    }
}