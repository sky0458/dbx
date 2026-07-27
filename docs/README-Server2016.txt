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
4. 多位同事可以从各自的 Windows/RDP 登录会话运行同一份 DBX.exe。
5. 每位用户的数据相互隔离：
   DBX 数据：%APPDATA%\com.dbx.app
   WebView2 数据：%LOCALAPPDATA%\com.dbx.app\WebView2\FixedRuntime

运行机制
--------
- portable.dbx 标记该程序为免安装便携包。
- multi-user.dbx 将 DBX 数据改为每个 Windows 用户独立保存，避免共享 SQLite 数据库。
- fixed-webview2.dbx 启用随包携带的 WebView2Runtime。
- DBX.exe 会在创建 Tauri 窗口前自动配置：
  WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=<本包>\WebView2Runtime
  WEBVIEW2_USER_DATA_FOLDER=%LOCALAPPDATA%\com.dbx.app\WebView2\FixedRuntime
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--no-sandbox
- 单实例限制只在当前 Windows 登录会话内生效；不同 RDP 会话可以同时运行。
- 不要让多位同事共用同一个 Windows 账号并发运行。应给每位同事使用独立账号。
- 如需在已验证兼容的服务器上恢复 WebView2 沙箱，可在启动 DBX 前设置：
  DBX_WEBVIEW2_FORCE_SANDBOX=1
- --no-sandbox 会降低 WebView2 渲染进程隔离强度，只应在受控服务器和可信内容下
  使用。Microsoft 要求 Windows 10 系列系统上的 120+ Fixed Version Runtime 在
  启用 App Container 时配置运行时目录 ACL；若恢复沙箱，请让管理员按 Microsoft
  WebView2 分发文档配置该目录权限。

更新
----
该测试包禁用了应用内“只替换 DBX.exe”的自动更新，因为那会破坏固定运行时
兼容逻辑。升级时请下载新的完整便携包并覆盖整个程序目录。每位用户的数据位于
其 Windows 用户配置目录中，不会因覆盖程序目录而丢失。

从旧的单用户便携包迁移
----------------------
旧包将所有人的配置写入 <旧包>\data，不适合并发共享。新版不会自动把这份共享数据
复制给每个用户。确需迁移时，请先退出所有 DBX 进程，再由管理员将旧 data 中需要的
文件复制到对应用户的 %APPDATA%\com.dbx.app；涉及已保存密码时应重新录入并测试。

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
- 若只有第一位用户能启动，检查 multi-user.dbx 是否仍与 DBX.exe 同目录，并确认
  WEBVIEW2_USER_DATA_FOLDER 未被系统级环境变量覆盖成共享目录。
- 如启动失败，请保留程序目录及当前用户 %APPDATA%\com.dbx.app 中的日志后再反馈。

许可与来源
----------
- DBX：Apache License 2.0，见本目录 LICENSE。
- WebView2 Runtime：Microsoft 官方 Fixed Version Runtime；运行时原始文件、签名及
  第三方许可查看脚本均保持原样。下载与分发须遵守 Microsoft WebView2 条款：
  https://developer.microsoft.com/microsoft-edge/webview2/
