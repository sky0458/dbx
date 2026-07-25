DBX Windows Server 2016 x64 免安装测试包
=========================================

适用环境
--------
- Windows Server 2016 LTSC x64，Desktop Experience（桌面体验）。
- 建议安装当前可用的 Windows 累积更新。
- 本包未签名，仅用于内部测试。

使用方法
--------
1. 将 DBX-portable 整个目录解压到本机 NTFS 磁盘，例如 D:\Tools\DBX-portable。
2. 不要从 ZIP 内、UNC 路径或网络共享直接运行。
3. 双击 DBX.exe，不需要安装 WebView2，也不需要管理员权限。
4. DBX 数据写入同目录的 data；WebView2 用户数据写入 data\webview2。

运行机制
--------
- portable.dbx 启用 DBX 便携数据目录。
- fixed-webview2.dbx 启用随包携带的 WebView2Runtime。
- DBX.exe 会在创建 Tauri 窗口前自动配置：
  WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=<本包>\WebView2Runtime
  WEBVIEW2_USER_DATA_FOLDER=<本包>\data\webview2
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--no-sandbox
- 如需在已验证兼容的服务器上恢复 WebView2 沙箱，可在启动 DBX 前设置：
  DBX_WEBVIEW2_FORCE_SANDBOX=1
- --no-sandbox 会降低 WebView2 渲染进程隔离强度，只应在受控服务器和可信内容下
  使用。Microsoft 要求 Windows 10 系列系统上的 120+ Fixed Version Runtime 在
  启用 App Container 时配置运行时目录 ACL；若恢复沙箱，请让管理员按 Microsoft
  WebView2 分发文档配置该目录权限。

更新
----
该测试包禁用了应用内“只替换 DBX.exe”的自动更新，因为那会破坏固定运行时
兼容逻辑。升级时请下载新的完整便携包，并保留原 data 目录。也可以直接解压覆盖，
但不要删除 data。

数据库驱动
----------
本包只包含 DBX.exe 和 Microsoft WebView2 Fixed Version Runtime，不包含 Oracle、
OceanBase Oracle 或其他额外数据库驱动/JRE。请通过 DBX 驱动管理器另行离线导入。
MySQL、PostgreSQL 和 OceanBase MySQL 模式使用 DBX 内置原生驱动。

排查
----
- 若 Windows 阻止未签名程序：在 DBX.exe 属性中检查“解除锁定”，并让管理员确认
  组织的应用控制策略。
- 任务管理器中 msedgewebview2.exe 的路径应位于本包 WebView2Runtime 目录。
- msedgewebview2.exe 命令行默认应包含 --no-sandbox。
- 如启动失败，请保留整个目录及 data 中的日志后再反馈。

许可与来源
----------
- DBX：Apache License 2.0，见本目录 LICENSE。
- WebView2 Runtime：Microsoft 官方 Fixed Version Runtime；运行时原始文件、签名及
  第三方许可查看脚本均保持原样。下载与分发须遵守 Microsoft WebView2 条款：
  https://developer.microsoft.com/microsoft-edge/webview2/
