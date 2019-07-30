netsh interface ipv4 set address name="dnet" source=static addr=10.145.195.115 mask=255.255.255.255
route add 3.112.67.122 mask 255.255.255.255 192.168.1.11
route add 10.255.255.254 mask 255.255.255.255 10.255.255.254 if 33
route add 0.0.0.0 mask 0.0.0.0 10.255.255.254 if 33
E:/Rust/mullvad/7_16/mullvadvpn-app/resource_dir/tinc//tinc-report.exe -u