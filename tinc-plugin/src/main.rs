extern crate tinc_plugin;
use tinc_plugin::control::dump_connections;
use tinc_plugin::listener::spawn;


fn main() {
    let a = spawn();
    loop {
        match a.recv() {
            Ok(event) => println!("{:?}", event),
            Err(_e) => (),
        }
    }
}