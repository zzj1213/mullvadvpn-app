### TODO
####首先完成linux下tinc, 其他滞后 
- [x] linux下添加tinc作为底层tunnel
- [ ] i2p
  - [ ] 启动i2p, daemon处理i2p事件
  - [ ] cli i2p相关命令
  - [ ] 从i2p获取和修改,用户信息,组信息
- [ ] 通过i2p信息,选择tunnel参数
  - [ ] 连接到proxy(公有proxy 或 用户proxy)
  - [ ] netfilter tinc-up 修改本机firewall, routing, 
- [ ] 公有proxy计费(虚拟币或其他)
- [ ] macos, windows下运行
### 与mullvad源码相比修改的文件
```
mullvadvpn-app
├── for_tinc                        // mullvadvpn   tinc相关修改信息记录
├── mullvad-cli
│   └── src
│       └──cmds
│           ├── relay.rs            // tinc中继选择设置， 中继节点用户名，密码设置（暂时未启用）
│           └── tunnel.rs           // 启动tinc相关命令（不直接开启连接， 仅启动tinc-deamon）
│   
├── mullvad-daemon
│   └── src
│       ├── lib.rs                  // create_tunnel_parameters 通过settings.json中的tunnel配置，
│       │                           // 返回相应tunnel所需参数
│       └── relays.rs               // 中继方式选择添加tinc
│                                   // RelayListUpdater 临时屏蔽 relay list的更新，
│                                   // 避免relay.json丢失tinc相关配置
│   
├── mullvad-types
│   └── src
│       ├── custom_tunnel.rs        // 添加解析TunnelParameters，并返回tinc所需参数的部分。ConnectionConfig tinc config
│       ├── relay_constraints.rs    // 添加TincConstraints， 和相应的TunnelConstraints中tinc的部分
│       ├── relays.rs               // 添加TincEndpointData， 和相应的RelayTunnels中tinc的部分
│       └── settings.rs             // 添加解析settings.json时相应的talpid_types::net::tinc::TunnelOptions
│   
├── talpid-core
│   ├── process
│   │   └── src
│   │       ├── mod.rs              // 添加解析pub mod tinc;
│   │       └── tinc.rs             // 已经清空(old log：新增文件，tinc相关的duct启动参数等) 
│   ├── tunnel
│   │   └── src
│   │       ├── mod.rs              // 添加解析pub mod tinc;
│   │       └── tinc.rs             // 新增文件，TincMonitor tinc tunnel 状态监听
│   ├── tunnel_state_machine        // 添加 TunnelParameters::Tinc(_) => vec![]  
│   └── Cargo.toml                  // 添加 tinc-plugin = { path = "../tinc-plugin" }
│                                   // 添加 openssl = "0.10" 用于tinc key pair generat，tokio-opensll用到了openssl库
│    
├── talpid-types
│   ├── src
│   │   └── net
│   │       ├── mod.rs              // 添加解析pub mod tinc; 添加TunnelParameters::Tinc
│   │       └── tinc                // 新增文件夹，tinc相关的信息结构体
│   └── Cargo.toml                  // 添加 tinc信息需要的 uuid = { version = "0.6", features = ["v4"] }
│   
├── tinc-plugin                     // daemon 与tinc tunnel 信息交互中介
│   ├── src
│   │   ├── bin
│   │   │   └── repote.rs           // tinc-up tinc-down host-up host-down 调用这个执行程序向daemon上报tinc节点上下线信息
│   │   ├── control.rs
│   │   ├── lib.rs
│   │   ├── listener.rs
│   │   ├── main.rs
│   │   └── tinc_tcp_stream.rs
│   └── Cargo.toml    
└── Cargo.toml                      // 添加 members  tinc-plugin 

```

### tinc作为mullvad tunnel 需要的相关配置文件
#### relay.json.tinc_example
中继节点列表中需要包含可用的tinc中继
```
"tunnels": {
    "tinc": [
        {
            "port": 50069,
            "protocol": "tcp"
        }
    ]
},
```
#### settings.json.tinc_example
配置文件中需要设置使用tinc作为通道
```
  "relay_settings": {
    "normal": {
      "location": "any",
      "tunnel": {
        "only": {
          "tinc": {
            "port": "any",
            "protocol": {
              "only": "tcp"
            }
          }
        }
      }
    }
  },
```