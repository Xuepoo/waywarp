# Waywarp (简体中文)

[English](./README.md) | 简体中文

Waywarp 是一款针对 Wayland 窗口合成器（基于 wlroots）的高性能、键盘驱动的鼠标光标控制工具。旨在为键盘流核心用户以及 AI 智能体（AI Agent）提供极致、无感且可编程的屏幕指针控制体验。

---

## 核心特性

- **网格标签定位 (Hint-based Positioning)**：在屏幕上渲染一层半透明的字符网格，键入对应字符即可瞬间将光标 Warp 到指定区域。
- **多级渐进精准度 (Multi-pass Refinement)**：支持粗调网格至微距网格的多次收敛（Coarse -> Fine），在大屏或高分辨率下实现像素级的精确定位（~4px）。
- **智能体 CLI 模式 (Agent CLI Mode)**：
  - 支持输出结构化 JSON 格式的全部屏幕网格坐标，供 LLM AI 智能体“看清”屏幕。
  - 支持免交互的绝对坐标控制、相对增量移动以及物理鼠标点击模拟（左键/右键/中键）。
- **灵活的指令回调 (Callback System)**：支持在 Warp 选定或手动退出后触发自定义的 Shell 指令（如触发系统原生的 warp、拖拽、点击或双击）。
- **多显示器原生感知 (Multi-monitor Support)**：自适应检测多屏物理拓扑，为不同分辨率的屏幕单独生成网格，支持跨屏无缝操控。

---

## 支持的窗口合成器 (Compositors)

- Hyprland
- Sway
- River
- Wayfire
- niri
- 任何基于 wlroots 的 Wayland 合成器

---

## 安装方式

### 1. 使用 Cargo 安装
```bash
cargo install waywarp
```

### 2. 从源码编译
```bash
git clone https://github.com/Xuepoo/waywarp.git
cd waywarp
cargo build --release
```

---

## 核心使用方法

```bash
# 1. 启动全屏交互式网格（键盘流用户模式）
waywarp

# 2. 以 JSON 格式输出屏幕上所有的网格点坐标（智能体感知模式）
waywarp --list-hints --format json

# 3. 指定特定的网格标签执行定位点击
waywarp --select "fa"

# 4. 绝对坐标移动 + 模拟点击
waywarp --move-to 800 460 --click left

# 5. 相对坐标微调移动 + 模拟点击 (v0.1.2+)
waywarp --move-by 50 -30 --click left

# 6. 启动键盘流持续驱动 Normal Mode (光标模式) (v0.1.3+)
waywarp --normal
```

### Normal Mode (光标模式) 默认按键绑定

使用 `waywarp --normal` 启动后，你将进入持续、平滑的键盘鼠标驱动模式：

| 按键 | 功能说明 |
| --- | --- |
| `h` / `左方向键` | 光标向左移动 |
| `j` / `下方向键` | 光标向下移动 |
| `k` / `上方向键` | 光标向上移动 |
| `l` / `右方向键` | 光标向右移动 |
| 按住 `Shift` | 3倍移动速度提升 (快速移动/加速度) |
| 按住 `Control` | 0.25倍移动速度降低 (高精细微调定位) |
| `f` / `回车键` | 物理鼠标左键点击 (若启用了 `exit_on_select` 将自动退出) |
| `d` | 物理鼠标右键点击 (若启用了 `exit_on_select` 将自动退出) |
| `s` | 物理鼠标中键点击 |
| `u` | 物理鼠标滚轮向上滚动 |
| `e` | 物理鼠标滚轮向下滚动 |
| `Escape` / `q` | 优雅退出 Normal Mode |

---

## 配置文件说明

Waywarp 在首次运行且检测到配置文件不存在时，会自动在 `~/.config/waywarp/config`（或 `$XDG_CONFIG_HOME/waywarp/config`）下生成默认的配置文件。

```ini
# ~/.config/waywarp/config
on_select_cmd=hyprctl dispatch movecursor {global_x} {global_y}
hint_font=monospace
hint_size=18
```

关于所有配置选项（如字体、半透明背景色、微调次数）、指令回调插值变量的完整详细说明与进阶实践，请参阅：[Waywarp 详细配置指南 (英文)](docs/configuration.md)。

---

## 开源协议

MIT License
