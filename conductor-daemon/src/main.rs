use std::error;
use std::sync::Arc;
use std::sync::RwLock;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::fs;
use std::path;

use chrono::{offset::Utc, TimeZone, Duration};
use clap::App as ClapApp;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::{future, Future, Stream};
use futures_timer::Delay;
use serde_json;
use serde_json::Value;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use ipnetwork;
use rand::{thread_rng, Rng};

use mullvad_types::relay_list::RelayList;
use mullvad_types::version::AppVersionInfo;
use mullvad_types::wireguard::AssociatedAddresses;
use tinc_plugin::{TincOperator, TincRunMode};

extern crate conductor;
#[allow(dead_code)]
use conductor::convention;
use conductor::Database;
use conductor::AccountInfo;

pub const COMMIT_ID: &str = include_str!(concat!(env!("OUT_DIR"), "/git-commit-id.txt"));

pub const COMMIT_DATE: &str = include_str!(concat!(env!("OUT_DIR"), "/git-commit-date.txt"));

/// The main handler for JSONRPC server.
fn rpc_handler(
    req: HttpRequest,
    payload: web::Payload,
) -> impl Future<Item = HttpResponse, Error = Error> {
    payload.concat2().from_err().and_then(move |body| {
        let reqjson: convention::Request = match serde_json::from_slice(body.as_ref()) {
            Ok(ok) => ok,
            Err(_) => {
                let r = convention::Response {
                    jsonrpc: String::from(convention::JSONRPC_VERSION),
                    result: Value::Null,
                    error: Some(convention::ErrorData::std(-32700)),
                    id: Value::Null,
                };
                return Ok(HttpResponse::Ok()
                    .content_type("application/json")
                    .body(r.dump()));
            }
        };
        let app_state = req.app_data().unwrap();
        let mut result = convention::Response::default();
        result.id = reqjson.id.clone();

        match rpc_select(&app_state, reqjson.method.as_str(), reqjson.params) {
            Ok(ok) => result.result = ok,
            Err(e) => result.error = Some(e),
        }

        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(result.dump()))
    })
}

fn rpc_select(
    _app_state: &AppState,
    method: &str,
    params: Vec<Value>,
) -> Result<Value, convention::ErrorData> {
    match method {
        "account_create" => {
            let days_str = match params[0].as_str() {
                Some(x) => x,
                None => return Err(convention::ErrorData::new(500, "Error params")),
            };
            let days: i64 = match days_str.parse() {
                Ok(x) => x,
                Err(_) => return Err(convention::ErrorData::new(500, "Error params")),
            };
            let res = AccountOperator::create_account(days);
            let r = serde_json::to_value(res).unwrap();
            return Ok(r);
        }
        "account_update" => {
            let acc = match params[0].as_str() {
                Some(x) => x,
                None => return Err(convention::ErrorData::new(400, "Error params")),
            };

            let days_str = match params[1].as_str() {
                Some(x) => x,
                None => return Err(convention::ErrorData::new(400, "Error params")),
            };
            let days: i64 = match days_str.parse() {
                Ok(x) => x,
                Err(_) => return Err(convention::ErrorData::new(500, "Error params")),
            };
            AccountOperator::update_account(acc, days);
            let r = serde_json::to_value(()).unwrap();
            return Ok(r);
        }
        "account_remove" => {
            let acc = match params[0].as_str() {
                Some(x) => x,
                None => return Err(convention::ErrorData::new(400, "Error params")),
            };

            AccountOperator::remove_account(acc);
            let r = serde_json::to_value(()).unwrap();
            return Ok(r);
        }
        "push_tinc_key" => {
            let acc = match params[0].as_str() {
                Some(x) => x,
                None => return Err(convention::ErrorData::new(400, "Error account param")),
            };
            if acc.len() < 6 {
                return Err(convention::ErrorData::new(401, "Account len error."))
            }

            let db = Database::instance();
            if let Ok(info) = db.account_select(acc) {
                let acc_time = info.expiry.timestamp();
                let now_time = Utc::now().timestamp();
                if acc_time - now_time < 0 {
                    return Err(convention::ErrorData::new(401, "Account expiry."));
                }
            }

            let pubkey = match params[1].as_str() {
                Some(x) => x,
                None => return Err(convention::ErrorData::new(400, "Error pubkey param")),
            };

            let vip_num: u32 = match ("1".to_string() + &acc[4..]).parse() {
                Ok(x) => x,
                Err(_) => return Err(convention::ErrorData::new(400, "Error params")),
            };
            let vip = IpAddr::from(Ipv4Addr::from(vip_num)).to_string();

            let host_name = TincOperator::get_filename_by_ip(false, &vip);
            if let Err(_) = TincOperator::instance().add_hosts(&host_name, pubkey) {
                return Err(convention::ErrorData::new(500, "Set host file failed."));
            };
            let local_pubkey = TincOperator::instance().get_local_pub_key().unwrap();
            let r = serde_json::to_value(&local_pubkey).unwrap();
            return Ok(r);
        }
        "get_expiry" => {
            if let Some(acc) = params[0].as_str() {
                let db = Database::instance();
                match db.account_select(acc) {
                    Ok(r) => {
                        let expiry = r.expiry;
                        let r = serde_json::to_value(&expiry).unwrap();
                        return Ok(r);
                    }
                    Err(e) => {
                        return Err(convention::ErrorData::new(404, &e.to_string()));
                    }
                }
            }

            let r = Utc.ymd(1970, 1, 1)
                .and_hms_milli(0, 0, 0, 0);
            let r = serde_json::to_value(&r).unwrap();
            return Ok(r);

        }
        "problem_report" => {
            Ok(serde_json::to_value(()).unwrap())
        }
        "relay_list_v2" => {
            let path = path::Path::new("./relay.json");
            let buf = fs::read(path).unwrap();
            let res = String::from_utf8(buf).unwrap();

            let relay_list: RelayList = serde_json::from_str(&res).unwrap();
            let r = serde_json::to_value(&relay_list).unwrap();
            Ok(r)
        }

        "app_version_check" => {
            let version = AppVersionInfo {
                current_is_supported: true,
                latest_stable:        "test_latest_stable".to_string(),
                latest:               "test_latest".to_string(),
            };
            let res = serde_json::to_value(&version).unwrap();
            Ok(res)
        }

        "push_wg_key" => {
            let ipv4 = Ipv4Addr::from_str("10.0.0.1").unwrap();
            let ipv6 = Ipv6Addr::from_str("::0").unwrap();
            let ip = AssociatedAddresses {
                ipv4_address: ipnetwork::Ipv4Network::new(ipv4, 0).unwrap(),
                ipv6_address: ipnetwork::Ipv6Network::new(ipv6, 0).unwrap(),
            };
            let r = serde_json::to_value(&ip).unwrap();
            Ok(r)
        }
        _ => Err(convention::ErrorData::std(-32601)),
    }
}

pub trait ImplNetwork {
    fn ping(&self) -> String;
    fn wait(&self, d: u64) -> Box<Future<Item = String, Error = Box<error::Error>>>;

    fn get(&self) -> u32;
    fn inc(&mut self);
}

pub struct ObjNetwork {
    c: u32,
}

impl ObjNetwork {
    fn new() -> Self {
        Self { c: 0 }
    }
}

impl ImplNetwork for ObjNetwork {
    fn ping(&self) -> String {
        String::from("pong")
    }

    fn wait(&self, d: u64) -> Box<Future<Item = String, Error = Box<error::Error>>> {
        if let Err(e) = Delay::new(::std::time::Duration::from_secs(d)).wait() {
            let e: Box<error::Error> = Box::new(e);
            return Box::new(future::err(e));
        };
        Box::new(future::ok(String::from("pong")))
    }

    fn get(&self) -> u32 {
        self.c
    }

    fn inc(&mut self) {
        self.c += 1;
    }
}

#[derive(Clone)]
pub struct AppState {
    network: Arc<RwLock<ImplNetwork>>,
}

impl AppState {
    pub fn new(network: Arc<RwLock<ImplNetwork>>) -> Self {
        Self { network }
    }
}

fn web_server() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let network = Arc::new(RwLock::new(ObjNetwork::new()));

    // load ssl keys
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("key.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("cert.pem").unwrap();


    let sys = actix::System::new("actix_jrpc");
    HttpServer::new(move || {
        let app_state = AppState::new(network.clone());
        App::new()
            .data(app_state)
            .wrap(middleware::Logger::default())
            .service(web::resource("/rpc/").route(web::post().to_async(rpc_handler)))
    })
        .bind_ssl("0.0.0.0:50071", builder)
        .unwrap()
        .workers(1)
        .start();

    let _ = sys.run();
}

struct AccountOperator{}
impl AccountOperator {
    fn create_account(days: i64) -> String {
        let mut rng = thread_rng();
        let db = conductor::Database::instance();
        let mut account;
        loop {
            let vip_8 = rng.gen_range(67_837_953, 84_549_373).to_string();
            let random_head = rng.gen_range(1_000, 9_999).to_string();
            account = random_head + &vip_8;
            match db.account_select(&account) {
                Ok(_) => (),
                Err(conductor::DbError::NoAccount) => break,
                Err(e) => {
                    return e.to_string();
                },
            }
        }
        let expiry = Utc::now() + Duration::days(days);
        let mut num: u64 = account.parse().unwrap();
        num = num % 10_000_0000 + 100_000_000;
        let vip_v4 = Ipv4Addr::from(num as u32);
        let vip = IpAddr::from(vip_v4);
        let info = AccountInfo {
            expiry,
            status:     "unused".to_string(),
            vip,
        };
        let res = match db.account_insert(&account, &info) {
            Ok(_) => account,
            Err(e) => e.to_string(),
        };
        return res;
    }

    fn update_account(account:&str, days: i64) -> bool {
        let db = conductor::Database::instance();
        if let Ok(mut info) = db.account_select(account) {
            info.expiry = info.expiry + Duration::days(days);
            if let Ok(_) = db.account_insert(&account, &info) {
                return true;
            };
        }
        return false;
    }

    fn remove_account(account:&str) -> bool {
        let db = conductor::Database::instance();
        if let Ok(_) = db.account_delete(&account) {
            return true;
        };
        return false;
    }
}

fn main() {
    ClapApp::new("conductor")
        .version(&format!("\nCommit date: {}\nCommit id: {}", COMMIT_DATE, COMMIT_ID).to_string()[..])
        .setting(clap::AppSettings::ColorAuto)
        .get_matches();

    // TODO mullvad path
    TincOperator::new("/root/mullvadvpn-app/", TincRunMode::Proxy);

    web_server();
}