## 运行说明
### 安装taptun
#### Linux
系统默认自带taptun驱动无需额外安装
#### windows
resource_dir/taptun/windows 对应的系统位数下 执行 addtap.bat\
执行resource_dir/taptun/mtap.bat\
查看windows网络适配器, 是否有名为dnet的虚拟网卡
#### macos
需添加taptun驱动\
说明: resource_dir/libs 文件夹下为macos tincd依赖动态库
### 设置运行环境
#### Linux
```
cp ./resource_dir/tinc/linux/* ./resourcec_dir/tinc
source ./resource_dir/runtime_env.sh
```
#### macos
```
cp ./resource_dir/tinc/macos/* ./resourcec_dir/tinc
source ./resource_dir/runtime_env.sh
```
#### windows
##### CMD or PowerShell
```
cd resource_dir
runtime_env.bat
```
##### bash
```
source ./resource_dir/runtime_env.sh
```

### 运行
1. 运行mullva-daemon
2. 创建新的dnet账号, 并设置可使用天数为30.这个操作将返回一个dnet账号
```
dnet account create 30
```
3. 设置账号, 将创建的账号, 或已有的账号设置到dnet.
```
dnet account set <账号>
```
4. 代理通道连接
```
dnet connect
```
5. 代理通道关闭连接
```
dnet disconnect
```
