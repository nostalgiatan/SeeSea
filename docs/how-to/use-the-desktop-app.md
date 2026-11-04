# 使用桌面版

桌面版是 `desktop/` 里的一个 Tauri 程序：一个窗口、一个托盘图标和一个全局快捷键，里面显示[工作台](use-the-workbench.md)。它启动的是同一个 `seesea` 程序，命令行和桌面版用的是同一份数据和设置。

## 运行

先构建 `seesea`，再构建并运行桌面版：

```bash
cargo build --release --manifest-path seesea-core/Cargo.toml -p seesea-cli
cd desktop
SEESEA_BIN=../seesea-core/target/release/seesea cargo run --release
```

Linux 上需要 `webkit2gtk-4.1` 和 `gtk3` 的开发包，其他系统见 [Tauri 的前置条件](https://v2.tauri.app/start/prerequisites/)。

桌面版按这个顺序找 `seesea`：环境变量 `SEESEA_BIN`，和桌面版程序放在同一个目录的 `seesea`，`PATH`。找不到时窗口显示原因。

## 日常使用

- 关闭窗口只是收到托盘，工作台里启动的 HTTP 接口和 MCP 服务继续运行。
- 托盘图标左键显示或收起窗口，右键菜单有“显示”“在浏览器中打开”和“退出”。
- `Ctrl Shift Space`（macOS 上是 `Cmd Shift Space`）唤起窗口并聚焦搜索框，再按一次收起。系统不允许注册这个快捷键时，只有这一项不可用。
- 同时只运行一个桌面版；再次启动会把已有的窗口带到前面。
- 点击结果里的链接在默认浏览器中打开。

退出时桌面版先让 `seesea` 关闭它启动的服务，再结束。桌面版被强行杀掉时，Linux 上 `seesea` 也会收到停止信号；macOS 和 Windows 上还没有这层保护。

## 打包

```bash
cargo install tauri-cli --locked   # 只需一次
desktop/scripts/bundle.sh
```

脚本构建发行版的 `seesea`，把它作为随附程序放进安装包，再生成当前系统的安装包。

## 安全

桌面版在 `127.0.0.1` 的一个空闲端口上启动 `seesea desk`，端口每次不同。窗口里的页面对 Tauri 来说是外部页面，没有被授予任何权限，不能调用桌面版的功能。

页面以外的地址不会在窗口里打开：它们交给默认浏览器。
