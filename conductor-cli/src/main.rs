use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

use chrono::{offset::Utc, Duration};
use clap::App;
use rand::{thread_rng, Rng};

use conductor::{self, AccountInfo};

pub const COMMIT_ID: &str = include_str!(concat!(env!("OUT_DIR"), "/git-commit-id.txt"));

pub const COMMIT_DATE: &str = include_str!(concat!(env!("OUT_DIR"), "/git-commit-date.txt"));

struct Account;

impl Account {
    fn name(&self) -> &'static str {
        "account"
    }

    fn clap_subcommand(&self) -> App<'static, 'static> {
        clap::SubCommand::with_name(self.name())
            .about("Control Mullvad account")
            .setting(clap::AppSettings::SubcommandRequiredElseHelp)
            .subcommand(
                clap::SubCommand::with_name("create")
                    .about("Create account")
                    .arg(
                        clap::Arg::with_name("days")
                            .help("The Mullvad account available days")
                            .required(true),
                    )
            )
            .subcommand(
                clap::SubCommand::with_name("update")
                    .about("Update account expiry")
                    .args(&vec!(
                            clap::Arg::with_name("account")
                                .help("The Mullvad account")
                                .required(true),
                            clap::Arg::with_name("days")
                                .help("The Mullvad account available days")
                                .required(true),
                        )
                    )
            )
            .subcommand(
                clap::SubCommand::with_name("remove")
                    .about("Remove account")
                    .args(&vec!(
                        clap::Arg::with_name("account")
                            .help("The Mullvad account")
                            .required(true),
                    ))
            )
    }

    fn run(&self, matches: &clap::ArgMatches<'_>) {
        if let Some(set_matches) = matches.subcommand_matches("create") {
            if let Some(days_str) = set_matches.value_of("days") {
                if let Ok(days) = days_str.parse() {
                    self.create_account(days);
                }
            }
        }

        if let Some(set_matches) = matches.subcommand_matches("update") {
            if let Some(account) = set_matches.value_of("account") {
                if let Some(days_str) = set_matches.value_of("days") {
                    if let Ok(days) = days_str.parse() {
                        self.update_account(account, days);
                    }
                }
            }
        }

        if let Some(set_matches) = matches.subcommand_matches("remove") {
            if let Some(account) = set_matches.value_of("account") {
                self.remove_account(account);
            }
        }
    }

    fn create_account(&self, days: i64) {
        let mut rng = thread_rng();
        let db = conductor::Database::new();
        let mut account;
        loop {
            let vip_8 = rng.gen_range(67_837_953, 84_549_373).to_string();
            let random_head = rng.gen_range(1_000, 9_999).to_string();
            account = random_head + &vip_8;
            match db.account_select(&account) {
                Ok(_) => (),
                Err(conductor::DbError::NoAccount) => break,
                Err(e) => {
                    println!("{:?}", e);
                    return;
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
        match db.account_insert(&account, &info) {
            Ok(_) => println!("{}", account),
            Err(e) => println!("{:?}", e),
        }
    }

    fn update_account(&self, account:&str, days: i64) {
        let db = conductor::Database::new();
        if let Ok(mut info) = db.account_select(account) {
            info.expiry = info.expiry + Duration::days(days);
            if let Ok(_) = db.account_insert(&account, &info) {
                println!("account:{}\naccount status:{:?}", account, info);
            };
        }
    }

    fn remove_account(&self, account:&str) {
        let db = conductor::Database::new();
        if let Ok(_) = db.account_delete(&account) {
            println!("remove account:{}", account);
        };
    }
}

fn main() {
    let mut commands = HashMap::new();
    commands.insert(Account{}.name(), Account{});
    let matches =  App::new("conductor")
        .version(&format!("\nCommit date: {}\nCommit id: {}", COMMIT_DATE, COMMIT_ID).to_string()[..])
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommands(commands.values().map(|cmd| cmd.clap_subcommand()))
        .get_matches();

    let (subcommand_name, subcommand_matches) = matches.subcommand();
    if let Some(cmd) = commands.get(subcommand_name) {
        cmd.run(subcommand_matches.expect("No command matched"))
    }
}
