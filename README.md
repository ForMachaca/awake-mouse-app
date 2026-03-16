# 屏幕常醒

[![CI](https://github.com/ForMachaca/awake-mouse-app/actions/workflows/build.yml/badge.svg)](https://github.com/ForMachaca/awake-mouse-app/actions/workflows/build.yml)
[![Release Packages](https://github.com/ForMachaca/awake-mouse-app/actions/workflows/release.yml/badge.svg)](https://github.com/ForMachaca/awake-mouse-app/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/ForMachaca/awake-mouse-app)](https://github.com/ForMachaca/awake-mouse-app/releases/latest)

一个跨端 GUI 桌面应用，用于防止屏幕休眠，并按固定间隔通过系统原生 API 发送带随机性的真实鼠标移动事件。

## 功能

- 图形界面启动、停止与更新参数
- 周期性防休眠
- 按随机微扰曲线路径逐帧移动鼠标，而不是一次性跳点
- 默认不回到原始位置，也可按需开启“回到起点”
- 平台分层，按系统调用原生 API

## 平台实现

- macOS
  - 防休眠：`IOPMAssertionCreateWithName`
  - 鼠标事件：`CoreGraphics CGEventCreateMouseEvent`
- Windows
  - 防休眠：`SetThreadExecutionState`
  - 鼠标事件：`SendInput`
- Linux
  - 防休眠：基于 `XResetScreenSaver` 的 X11 活动刷新
  - 鼠标事件：`XTestFakeMotionEvent`
  - 说明：当前仅支持 X11，Wayland 默认不允许全局模拟鼠标

## 运行

```bash
cargo run
```

## 下载

- 稳定打包产物：从 [Releases](https://github.com/ForMachaca/awake-mouse-app/releases) 下载
- 开发构建产物：从 [Actions](https://github.com/ForMachaca/awake-mouse-app/actions) 的最新成功工作流下载
- Tag 发布时会自动生成这些平台包：
  - macOS：`.app.tar.gz`
  - Windows：`.msi / .exe`
  - Linux：`.deb`
- Windows 下载说明：
  - `.msi`：标准安装包，适合常规安装与卸载
  - `.exe`：免安装的裸可执行文件，可直接运行

## Tag 发布流程

```bash
git checkout main
git pull
git tag v0.1.0
git push origin v0.1.0
```

- 推送 `v*` 标签后，`Release Packages` 工作流会自动构建 `.app / .msi / .exe / .deb`
- 所有打包产物会自动上传到对应的 GitHub Release
- 日常 `push` 和 `pull request` 只走 `CI` 工作流，用于编译检查和上传普通构建产物

## 使用提示

- macOS 首次运行需要在“系统设置 > 隐私与安全性 > 辅助功能”里允许应用控制电脑，否则鼠标事件可能被系统拦截。
- Linux 需要存在 X11 和 `Xtst` 运行库。
- 如果你希望“开始后立刻执行一次移动”，可以把 `worker.rs` 中的首次调度改成 `Instant::now()`。
