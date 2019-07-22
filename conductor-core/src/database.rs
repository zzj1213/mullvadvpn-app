use std::path::PathBuf;
use std::fs;
use std::io::{self, Write, BufReader, BufRead};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use chrono::{offset::Utc, DateTime};
use serde_json;

use crate::types::AccountInfo;

static mut EL: *mut Database = 0 as *mut _;
const ACCOUNT_PATH: &str = "./account.json";
const RELAY_PATH: &str = "./relay.json";

pub type Result<T> = std::result::Result<T, Error>;

#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "IO ERROR")]
    IoError(#[error(cause)] io::Error),
    #[error(display = "JSON ERROR")]
    JsonError(#[error(cause)] serde_json::Error),
    #[error(display = "No such user.")]
    NoAccount,
}

pub struct Database {
    mutex:          Mutex<i32>,
    account_path:   PathBuf,
    relay_path:     PathBuf,
}

impl Database {
    pub fn new() -> Self {
        let mutex = Mutex::new(0);
        let account_path = PathBuf::from_str(ACCOUNT_PATH).unwrap();
        if !account_path.is_file() {
            let mut f = fs::OpenOptions::new().create(true).write(true).open(&account_path)
                .expect("Can't create account.json");
            f.write(b"{}").expect("Can't create account.json");
        }
        let relay_path =  PathBuf::from_str(RELAY_PATH).unwrap();
        if !relay_path.is_file() {
            let mut f = fs::OpenOptions::new().create(true).write(true).open(&relay_path)
                .expect("Can't create relay.json");
            f.write(b"{}").expect("Can't create relay.json");
        }
        Database {
            mutex,
            account_path,
            relay_path,
        }
    }

    pub fn instance() -> &'static mut Self {
        unsafe {
            if EL == 0 as *mut _ {
                EL = Box::into_raw(Box::new(Database::new()));
            }
            &mut *EL
        }
    }

    fn account_load(&self) -> Result<HashMap<String, AccountInfo>> {
        let _ = self.mutex.lock().unwrap();
        let f = fs::OpenOptions::new().read(true).open(&self.account_path)
            .map_err(Error::IoError)?;
        let reader = BufReader::new(f);
        let mut res = String::new();
        for line in reader.lines() {
            let line = line.map_err(Error::IoError)?;
            res += &line;
        }
        return serde_json::from_str(&res).map_err(Error::JsonError);
    }

    fn account_save(&self, data: HashMap<String, AccountInfo>) -> Result<()> {
        let _ = self.mutex.lock().unwrap();
        let json_data = serde_json::to_string(&data)
            .map_err(Error::JsonError)?;
        return fs::write(&self.account_path, json_data.as_bytes()).map(|_|()).map_err(Error::IoError);
    }

    pub fn account_insert(&self, account: &str, info: &AccountInfo) -> Result<()> {
        let mut data = self.account_load()?;
        data.insert(account.to_string(), info.clone());
        self.account_save(data)
    }

    pub fn account_delete(&self, account: &str) -> Result<()> {
        let mut data = self.account_load()?;
        data.remove(account);
        self.account_save(data)
    }

    pub fn account_select(&self, account: &str) -> Result<AccountInfo> {
        let data = self.account_load()
            .map_err(|_|Error::NoAccount)?;
        if let Some(info) = data.get(account) {
            return Ok(info.clone());
        }
        else {
            return Err(Error::NoAccount);
        }
    }

    pub fn vip_check_exits(&self, account: &str) -> Result<()> {
        let data = self.account_load()
            .map_err(|_|Error::NoAccount)?;
        let keys = data.keys();
        for key in keys {
            if account[4..].to_string() == key[4..].to_string() {
                return Ok(());
            }
        }
        return Err(Error::NoAccount);

    }
}