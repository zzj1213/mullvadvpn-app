##编译流程
###Unix
#### 下载源码
```
git clone https://github.com/mullvad/mullvadvpn-app.git
    当前fork: https://github.com/zzj1213/mullvadvpn-app.git
```
####git下载 submodule (包括openvpn)
```
git submodule update --init
```
####编译
```
cargo build --release
```
###windows

##运行
```
将当前目录下
api_root_ca.pem 
ca.crt
tincd
上级目录下
relays.json.tinc_example   重命名为relays.json
拷贝到mullvad-daemon运行目录.比如target/debug/

settings.json.tinc_example   重命名为settings.json
拷贝到/etc/mullvad-vpn

运行mullvad-daemon, 通过mullvad 命令行控制mullvad-daemon
```