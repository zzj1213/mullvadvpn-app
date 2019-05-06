#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unknown_lints)]

use std::str::FromStr;
use std::net::{Ipv4Addr, SocketAddr, TcpStream, IpAddr};

use std::time::Duration;
use std::io::{stdout, Write, Read, Error, ErrorKind};

use curl::easy::{Easy, List};
use super::sys_tool::cmd;

pub fn get_wan_name() -> Option<String> {
    let local_ip = get_local_ip().unwrap().to_string();

    let (code, output) = cmd(
        "ip address|grep ".to_string() + &local_ip + " | awk '{print $(7)}'");

    if code != 0 {
        return None;
    }

    Some(output)
}

// 连接8.8.8.8 或8.8.4.4 获取信号输出网卡ip，多网卡取路由表默认外网连接ip
// get_localip().unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))
pub fn get_local_ip() -> std::io::Result<IpAddr> {
    let timeout = Duration::new(3 as u64, 0 as u32);
    let addr0 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8,8,8,8)), 53);
    // 如果可以连接到8.8.8.8 || 8.8.4.4 获取出口ip，如果失败获取网卡ip
    let socket = match TcpStream::connect_timeout(&addr0, timeout) {
        Ok(x) => x,
        Err(_) => {
            let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8,8,4,4)), 53);
            let socket1 = match TcpStream::connect_timeout(&addr1, timeout) {
                Ok(x) => x,
                Err(e) => {
                    let (code, output) = cmd(
                        "ip address|grep -w inet | awk 'NR == 2' | awk '{print $(2)}'".to_string());
                    if !code == 0 {
                        return Err(e);
                    }
                    let ip: Vec<&str> = output.split("/").collect();
                    match IpAddr::from_str(ip[0]){
                        Ok(ip) => return Ok(ip),
                        Err(_) => return Err(e),
                    };
                }
            };
            socket1
        }
    };
    let ip = socket.local_addr()?.ip();
    Ok(ip)
}

/// http 请求返回结果
pub struct  HttpResult {
    pub code: u32,
    pub data: String,
    pub header: Vec<String>,
}

/// https get请求
pub fn url_get(url:&str) -> Result<HttpResult, Error> {
    let mut res_data = Vec::new();
    let mut headers = Vec::new();

    let mut handle = get_handle(url)?;
    {
        let mut transfer = handle.transfer();

        transfer.header_function(|header| {
            headers.push(String::from_utf8_lossy(header).to_string());
            true
        })?;

        let _ = transfer.write_function(|buf| {
            res_data.extend_from_slice(buf);
            Ok(buf.len())
        });
        transfer.perform()?;
    }

    let data = String::from_utf8_lossy(&res_data).into_owned();
    let header = headers;

    let code = handle.response_code().unwrap();
    let res = HttpResult {
        code,
        data,
        header,
    };
    return Ok(res);
}

/// https post请求
pub fn url_post(url: &str, data: String, cookie: &str) -> Result<HttpResult, Error> {
    let mut send_data = data.as_bytes();
    let cookie = cookie.clone().replace("\r\n", "");
    let mut res_data = Vec::new();
    let mut headers = Vec::new();

    let mut handle = post_handle(url, send_data.len())?;

    let mut list = List::new();
    list.append("Content-Type: application/json;charset=UTF-8")?;

    if cookie.len() > 0 {
        list.append(&("Cookie: ".to_string() + &cookie))?;
        handle.http_headers(list)?;

        handle.post_field_size( (send_data.len()) as u64)?;
        handle.http_content_decoding(true)?;
    } else {
        handle.post_field_size(send_data.len() as u64)?;
    }

    {
        let mut transfer = handle.transfer();
        transfer.header_function(|header| {
            headers.push(String::from_utf8_lossy(header).to_string());
            true
        })?;

        transfer.read_function(move |into| {
            Ok(send_data.read(into).unwrap_or(0))
        })?;

        let _ = transfer.write_function(|buf| {
            res_data.extend_from_slice(buf);
            Ok(buf.len())
        });

        transfer.perform().map_err(|e|Error::new(ErrorKind::InvalidData, "curl_perform"))?;
    }

    let data = String::from_utf8_lossy(&res_data).into_owned();
    let header = headers;

    let code = handle.response_code().unwrap();
    let res = HttpResult {
        code,
        data,
        header,
    };
    return Ok(res);
}

/// 将json 解析成 a=1&b=2 格式
pub fn http_json(json_str: String) -> String {
    let json_str = json_str.clone();
    json_str.replace("\\\"", "")
        .replace("\"", "")
        .replace(":", "=")
        .replace(",", "&")
        .replace("{", "")
        .replace("}", "")
}
/// 创建post请求handle
fn post_handle(url: &str, post_size: usize) -> Result<Easy, Error> {
    let mut handle = get_handle(url)?;
    handle.post(true)?;
    handle.post_field_size(post_size as u64)?;
    Ok(handle)
}

/// 创建get请求handle
fn get_handle(url: &str) -> Result<Easy, Error> {
    let mut handle = Easy::new();
    handle.timeout(Duration::new(5, 0))?;
    handle.show_header(false)?;
    handle.url(url)?;
    handle.ssl_verify_host(false)?;
    handle.ssl_verify_peer(false)?;
    Ok(handle)
}
