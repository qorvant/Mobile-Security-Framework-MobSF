# MobSF-Win

将 [MobSF](https://github.com/MobSF/Mobile-Security-Framework-MobSF) 的**静态分析**能力移植到 Windows 桌面端的实验性项目。

架构选择（与用户确认）：

- **Tauri (WebView2)** 作为桌面 GUI 壳 —— 复用 Web 技术栈，体积小。
- **单 exe 内嵌引擎** —— GUI 与分析引擎在同一个可执行文件内，通过 Tauri 的
  `invoke` 通道分层，部署最简单。
- **聚焦 StaticAnalyzer** —— 用 Rust **调用（而非重写）** 外部工具
  （apktool / jadx）完成反编译；纯 Rust 部分负责无需外部工具的解析。

## 组件

```
mobsf-win/
├── engine/        # 纯 Rust 分析引擎（不依赖 Tauri，可独立测试）
│   └── src/
│       ├── axml.rs   # 二进制 AndroidManifest.xml (AXML) 解析器（纯 Rust）
│       ├── manifest.rs
│       ├── apk.rs     # APK(zip) 读取
│       └── tools.rs   # apktool / jadx 子进程编排
├── app/           # Tauri 壳（编译为单 exe）
├── src/           # 前端（HTML/JS/CSS，WebView2 渲染）
└── scripts/       # 图标生成等辅助脚本
```

## 已落地能力

- ✅ **APK 清单解析**：优先调用官方 `aapt dump badging`（兼容 AAPT1/AAPT2
  所有二进制 XML 版本），解析出包名、版本、SDK 级别、权限列表、
  Activity/Service/Receiver/Provider 组件及其 `exported` 标志。
  纯 Rust 的 `AndroidManifest.xml`(AXML) 解析器作为 AAPT1 的尽力回退。
- ✅ 列出 APK 内所有条目。
- ✅ 通过子进程调用 apktool / jadx 进行反编译（参数以列表形式传递，避免命令注入）。

## 构建与运行

前置：Rust 工具链、Node.js（仅用于前端资源，Tauri v2 构建期需要）、
Windows 10+（自带 WebView2）。

```powershell
cd mobsf-win
node scripts/gen_icon.mjs          # 生成 app/src-tauri/icons/icon.ico
cargo install tauri-cli --version "^2"   # 若尚未安装 Tauri CLI
cargo tauri build                  # 产物为单个 .exe（NSIS 安装包）
```

不装 Tauri CLI 也可直接用 cargo 编译（首次会下载 WebView2 运行时等依赖）。
**必须显式带上 `custom-protocol` 特性**，否则 `tauri/build.rs` 会判定
`dev = true`，前端资源不内嵌，WebView 会去加载 `devUrl` 而报
`ERR_CONNECTION_REFUSED`：

```powershell
cargo build --release -p mobsf-win --features custom-protocol
```

也可以直接运行根目录的 `build.bat`（已包含上述特性与图标生成）。

## 放置外部工具（反编译功能需要）

反编译需要两个 Java 工具：`apktool`（资源 + smali）与 `jadx`（Java/Kotlin 源码）。
它们**不会**随仓库发布，需要另行准备（两者都需要系统 `PATH` 上有 `java`）：

```powershell
cd mobsf-win
powershell -ExecutionPolicy Bypass -File scripts/download_tools.ps1
```

脚本会把工具下载到 `mobsf-win/tools/`（已 git-ignore）：

- `tools/apktool.jar`
- `tools/jadx/bin/jadx.bat`

也可手动放置，支持以下任一位置（自动探测，相对路径基于启动目录，也支持放在 exe 同目录）：

- `jadx` 在 `PATH`，或 `tools/jadx/bin/jadx[.bat][.exe]`
- `apktool.jar` 在 `tools/apktool.jar`，或当前目录的 `apktool.jar`

> 探测逻辑见 `engine/src/tools.rs` 的 `discover_tools()`。注意：之前版本的
> 探测只搜 `PATH`，会把 `tools/` 下的 `jadx.bat` 误判为"not found on PATH"，
> 现已修复（同时会回退到 exe 所在目录）。

未配置工具时，纯 Rust 的清单解析与分析仍然可用。

## 测试

```powershell
cargo test -p mobsf-engine
```

该测试会用仓库内 `mobsf/DynamicAnalyzer/.../*.apk` 真实固件校验 AXML 解析器。

## 当前范围与后续

这只是迁移的第一步。后续可逐步接入：

- 证书 / 签名信息解析（META-INF）
- `libsast` 规则匹配的 Rust 化或子进程化
- 恶意代码检测模型的推理接入
- 动态分析（Frida）的 Windows 适配（工作量最大）
