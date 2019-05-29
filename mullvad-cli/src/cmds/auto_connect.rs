use crate::{new_rpc_client, Command, Result};
use clap::value_t_or_exit;

pub struct AutoConnect;

impl Command for AutoConnect {
    fn name(&self) -> &'static str {
        "auto-connect"
    }

    fn clap_subcommand(&self) -> clap::App<'static, 'static> {
        clap::SubCommand::with_name(self.name())
            .about("Control the daemon auto-connect setting")
            .setting(clap::AppSettings::SubcommandRequiredElseHelp)
            .subcommand(
                clap::SubCommand::with_name("set")
                    .about("Change auto-connect setting")
                    .arg(
                        clap::Arg::with_name("policy")
                            .required(true)
                            .possible_values(&["on", "off"]),
                    ),
            )
            .subcommand(
                clap::SubCommand::with_name("get")
                    .about("Display the current auto-connect setting"),
            )
    }

    fn run(&self, matches: &clap::ArgMatches<'_>) -> Result<()> {
        if let Some(set_matches) = matches.subcommand_matches("set") {
            let auto_connect = value_t_or_exit!(set_matches.value_of("policy"), String);
            self.set(auto_connect == "on")
        } else if let Some(_matches) = matches.subcommand_matches("get") {
            self.get()
        } else {
            unreachable!("No auto-connect command given");
        }
    }
}

impl AutoConnect {
    fn set(&self, auto_connect: bool) -> Result<()> {
        let mut rpc = new_rpc_client()?;
        rpc.set_auto_connect(auto_connect)?;
        println!("Changed auto-connect sharing setting");
        Ok(())
    }

    fn get(&self) -> Result<()> {
        let mut rpc = new_rpc_client()?;
        let auto_connect = rpc.get_settings()?.get_auto_connect();
        println!("Autoconnect: {}", if auto_connect { "on" } else { "off" });
        Ok(())
    }
}
