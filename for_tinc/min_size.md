## 压缩编译结果大小

### 1. strip
直接剥离
strip mullvad-daemon
strip mullvad
添加编译命令
vim~/.cargo/config
```
[target.x86_64-unknown-linux-gnu]
rustflags="-C link-arg=-s"
```

### 2. Removing jemalloc
未使用, jemalloc用于优化动态内存分配, 可通过使用系统默认的malloc避免在二进制文件中添加jemalloc,
未使用原因:需要在源码中大量添加
```
#[cfg(not(feature = "system-alloc"))]
```

### 3. Panic Abort
已使用, 影响: panic时无相应错误信息, 因为有log信息影响不大
使用方法: 在Cargo.toml中添加release编译配置
```$xslt
[profile.release]
panic = "abort"
```

### 4. Use LLVM's full LTO
已使用, 全程使用LLVM编译, 不使用增量编译
影响: 编译时更好的优化二进制文件,延长编译所需时间. 
使用方法: 在Cargo.toml中添加release编译配置
```
[profile.release]
lto = true
incremental = false
```

### 5. Reduce Parallel Code Generation Units
By default, Cargo specifies 16 parallel codegen units for release builds. This improves compile times, but prevents some optimizations.
Set thisto 1 in Cargo.toml to allow for maximum size reduction optimizations:
已使用
使用方法: 在Cargo.toml中添加release编译配置
```
[profile.release]
codegen-units = 1
```

### 6. 设置cargo优化等级
使用方法: 在Cargo.toml中添加release编译配置
```
[profile.release]
opt-level = "z"
```

### 7. upx
压缩可执行程序, 启动程序时再解压, 影响: 延长程序启动时间

### 8. #![no_std] #![no_main]
未使用, 原因: mullvad需要使用标准库

### 9. xargo编译
未使用, 需要Nightly Rust

