# WorkBuddyBar

基于 Tauri 2、Vue 3 和 TypeScript 的 WorkBuddy 管理端状态托盘应用，支持 macOS 和 Windows。

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
