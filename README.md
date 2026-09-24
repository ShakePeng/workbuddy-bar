# WorkBuddyBar

基于 Tauri 2、Vue 3 和 TypeScript 的 WorkBuddy 管理端状态托盘应用，支持 macOS 和 Windows。

## 下载安装

前往 GitHub Releases 下载当前版本：

- macOS Apple Silicon：下载 `WorkBuddyBar-vX.Y.Z-macos-aarch64.dmg`，安装后从“应用程序”启动。
- Windows x64：下载 `WorkBuddyBar-vX.Y.Z-windows-x64-setup.exe`，按安装向导完成安装。

当前安装包未进行 Apple 签名/公证和 Windows 代码签名，首次启动可能出现系统安全提示：

- macOS 若被 Gatekeeper 拦截，请右键应用选择“打开”；也可以在终端执行：
  ```bash
  xattr -dr com.apple.quarantine /Applications/WorkBuddyBar.app
  ```
- Windows 若出现 SmartScreen 提示，请选择“更多信息”，然后点击“仍要运行”。

## 开发

```bash
yarn install
yarn tauri:dev
```

## 构建

构建当前平台的安装包：

```bash
yarn tauri:build
```

构建 Windows NSIS 安装包：

```bash
yarn tauri:build:windows
```

构建 macOS Apple Silicon DMG：

```bash
yarn tauri:build:macos
```

Windows 构建需要 Rust MSVC 工具链和 WebView2。安装包会输出到 `src-tauri/target/release/bundle/`。

## 发布

发布版本以 Git 标签为准，格式为 `vMAJOR.MINOR.PATCH`。发布前需要保持以下文件中的版本一致：

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`

提交版本变更后推送标签：

```bash
git tag v1.0.0
git push origin v1.0.0
```

推送 `v*` 标签后，GitHub Actions 会自动构建 macOS ARM64 DMG 和 Windows x64 NSIS 安装包，并创建 GitHub Release。
