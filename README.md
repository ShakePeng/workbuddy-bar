# WorkBuddyBar

基于 Tauri 2、Vue 3 和 TypeScript 的 WorkBuddy 管理端状态托盘应用，支持 macOS 和 Windows。

## 登录

首次启动需要填写管理端地址，并选择其中一种登录方式：

- **访问令牌**：粘贴管理员在管理端创建的访问令牌（`wbt_…`），以 `Authorization: Bearer` 透传，运行中令牌失效会回到登录页重填。
- **账户密码**：填写管理端账号与密码，登录后使用管理端下发的 `wb_session` 会话；账号密码保存在本机配置文件中，会话过期时自动重新登录，适合长期挂机。

两种方式的凭据都只写入本机配置文件（权限 0600），不会写入日志。使用明文 HTTP 地址时界面会给出提示。

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

同时在 `CHANGELOG.md` 里补上该版本的段落（格式 `## [x.y.z] - YYYY-MM-DD`）。CI 会校验该段落存在，
并把它自动截取为 GitHub Release 的更新内容，后面再附上固定的安装说明。

提交版本变更后推送标签：

```bash
git tag v1.0.0
git push origin v1.0.0
```

推送 `v*` 标签后，GitHub Actions 会自动构建 macOS ARM64 DMG 和 Windows x64 NSIS 安装包，并创建 GitHub Release。

## 自动更新

应用内置检查更新：在弹层底部或「设置 → 软件更新」点击「检查更新」，确认后自动下载、安装并重启。
更新源是 GitHub Release 上的 `latest.json` 与签名安装包，仓库需保持公开以便匿名下载。

一次性配置（已完成的可跳过）：

1. 生成更新签名密钥（私钥保留在本机，不要提交或外发）：

   ```bash
   yarn tauri signer generate -w ~/.tauri/workbuddy-updater.key --password "" --ci
   ```

2. 把公钥写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。
3. 在仓库 `Settings → Secrets and variables → Actions` 添加 `TAURI_SIGNING_PRIVATE_KEY`，值为私钥文件内容。

私钥未设密码时无需添加密码 Secret：workflow 会把 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 置为空，
避免签名时出现交互式密码提示卡住构建。

发布时 CI 会用私钥给更新包签名，并生成包含两平台签名的 `latest.json` 随 Release 上传。
注意：1.0.5 及更早版本没有更新器，需要手动下载安装 1.0.7 一次，之后的版本才能在应用内自动更新。
