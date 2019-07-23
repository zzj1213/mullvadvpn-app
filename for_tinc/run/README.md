## 文件说明
```
./api_root_ca.pem				mullvad 官方服务器https CA证书
./ca.crt						mullvad 官方服务器openvpn CA证书
./cert.pem						conductor https CA证书
./key.pem						conductor https 密钥
../relays.json.tinc_example		tinc 模式的relays.json, 由conductor读取, client通过https rpc 获取
../settings.json.tinc_example	mullvad client 以tinc模式运行的配置文件示例
```

##Unix
###编译流程
#### 下载依赖
```
# For building the daemon
sudo apt install gcc libdbus-1-dev
# For running the frontend app
sudo apt install gconf2
# For building the installer
sudo apt install rpm
```
#### 下载源码
```
git clone https://github.com/mullvad/mullvadvpn-app.git
    当前fork: https://github.com/zzj1213/mullvadvpn-app.git
```
####git下载 submodule (包括openvpn)
```
git submodule update --init
```
#### 设置环境变量(OPENSSL_DIR, LIBMNL_DIR..)
```
source env.sh
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
#### 下载源码
```
git clone git://github.com/mullvad/mullvadvpn-app.git
cd mullvadvpn-app
```
#### git下载 submodule (包括openvpn)
```
git submodule update --init
```
#### 编译依赖的vc项目
```    
vs 2017编译 mullvadvpn-app/windows下所有项目
OR:
添加msbuild.exe到环境变量Path
msbuild.exe 所在地址: Microsoft Visual Studio\2017\Community\MSBuild\15.0\Bin
bash ./build_windows_modules.sh --dev-build
```

#### 编译Rust代码
```
cargo build --release
```
## 运行:
### 设置mullvad运行文件env
```
MULLVAD_RESOURCE_DIR
```
### 设置mullvad设置文件env
```
MULLVAD_SETTINGS_DIR
```

将当前目录下
```
cert.pem
key.pem             (conductor https key)

api_root_ca.pem             mullvad rpc https ca证书
ca.crt
tincd(windows下tincd.exe)
以及上级目录下
relays.json.tinc_example   重命名为relays.json
```
拷贝到mullvad-daemon运行目录(如果设置MULLVAD_RESOURCE_DIR, 则为对应目录).

settings.json.tinc_example   重命名为settings.json
拷贝到MULLVAD_SETTINGS_DIR linux下为/etc/mullvad

运行mullvad-daemon, 通过mullvad 命令行控制mullvad-daemon