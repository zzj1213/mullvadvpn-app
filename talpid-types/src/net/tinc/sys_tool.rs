#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unknown_lints)]

use std::string;


//For cmd
use std::process::Command;
use std::str;
//extern crate encoding;
//use self::encoding::all::GB18030;
//use std::process::Command;

// This function only gets compiled if the target OS is linux
#[cfg(target_os = "linux")]
pub fn is_linux() ->bool {
    return true;
}

// And this function only gets compiled if the target OS is *not* linux
#[cfg(not(target_os = "linux"))]
pub fn is_linux() ->bool {
    return false;
}

#[cfg(target_os = "windows")]
pub fn is_win() ->bool {
    return true;
}

#[cfg(not(target_os = "windows"))]
pub fn is_win() ->bool {
    return false;
}

pub fn get_env_var(key:&str) -> String {
    let mut value:String = String::from("");
    value = match ::std::env::var(key) {
        Ok(value) => value,
        Err(_) => return value,
    };
    return value;
}

// windows下正常。 linux下非系统应用获取不到返回值，且无法获取外部程序执行错误。
// v2 Linux 应该可以了，windows下返回中文无法处理需要GBK库 暂时未处理
//如果成功返回code = 0, output=执行的stdout输出, 否则code = 错误码，output = 错误信息
#[cfg(target_os = "linux")]
pub fn cmd(command:String) -> (i32, String) {
    let res;
    let output: String;
    let code: i32;
    let command_clone= command.clone();

    res = Command::new("/bin/bash")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|err| err.to_string());

    match res {
        Ok(pres) => {
            code = pres.status.code().expect(&("can't exec ".to_string() + &command_clone));
            let err:String = str::from_utf8(&pres.stderr).ok().unwrap().to_owned();
            if err.len() > 0 {
                output = err.clone();
            } else {
                output = str::from_utf8(&pres.stdout).ok().unwrap().to_owned();
            };
        },

        Err(err) => {
            output = err;
            code = -1;
        },
    };

    return (code, output.replace("\n", ""));
}

#[cfg(target_os = "windows")]
pub fn cmd(command:String) -> (i32, String) {
    let res;
    let output: String;
    let code: i32;
    let command_clone= command.clone();

    let iter: Vec<_> = command.split_whitespace().collect();
    let list_command = iter.into_iter();

    res = Command::new("cmd")
        .arg("/C")
        .args(list_command)
        .output()
        .map_err(|err| err.to_string());

    match res {
        Ok(pres) => {
            code = pres.status.code().expect(&("can't exec ".to_string() + &command_clone));
            output = str::from_utf8(&pres.stdout).ok().unwrap().to_owned();
        },
        Err(err) => {
            output = err;
            code = -1;
        },
    };

    return (code, output.replace("\n", "").replace("\r", ""));
}

pub fn cmd_err_panic(command:String) -> String {
    let (code, output) = cmd(command);
    if code != 0 {
        panic!(output);
    };
    return output;
}