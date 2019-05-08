##Unix
###编译流程
#### 下载源码
```
git clone https://github.com/mullvad/mullvadvpn-app.git
    当前fork: https://github.com/zzj1213/mullvadvpn-app.git
```
####git下载 submodule (包括openvpn)
```
git submodule update --init
```
####编译:
```
cargo build --release
```
---
##windows
### 编译需求:
```
   vs 2017 v141tool
   windows sdk  10.0.16299.0 (必须要该版本sdk)
```
### 编译流程:
```
   下载源码
       git clone git://github.com/mullvad/mullvadvpn-app.git
       cd mullvadvpn-app
       git下载 submodule (包括openvpn)
       git submodule update --init
       
       vs 2017编译 mullvadvpn-app/windows下所有项目
   
   将以下编译结果
       winfw.dll, windns.dll, winnet.dll 拷贝到 C:\Windows\System32
   
   编译Rust代码
       cargo build --release
```

## 运行:
```
将当前目录下
api_root_ca.pem             mullvad rpc https ca证书
ca.crt
tincd(windows下tincd.exe)
上级目录下
relays.json.tinc_example   重命名为relays.json
拷贝到mullvad-daemon运行目录.比如target/debug/

settings.json.tinc_example   重命名为settings.json
拷贝到/etc/mullvad-vpn

运行mullvad-daemon, 通过mullvad 命令行控制mullvad-daemon
```