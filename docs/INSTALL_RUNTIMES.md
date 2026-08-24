# 多语言 cunzhi 运行时安装指南

当 winget 被安装锁阻塞时，可使用直接下载方式安装。

**快速安装（管理员）**：`.\scripts\install-runtimes.ps1`  
**无管理员时**：仅 PHP 会安装到用户目录。

## Go

**便携安装（无需管理员）**：下载 `go1.26.0.windows-amd64.zip`，解压到 `%LOCALAPPDATA%\Go`，将 `%LOCALAPPDATA%\Go\bin` 加入用户 PATH。

**MSI 安装（需管理员）**：

- **下载**: https://go.dev/dl/
- **Windows**: 选择 `go1.26.0.windows-amd64.msi`（或最新版）
- **安装**: 双击运行，安装时勾选「Add to PATH」
- **验证**: 新开终端执行 `go version`

## Java 17

**便携安装（无需管理员）**：下载 `microsoft-jdk-17-windows-x64.zip`（https://aka.ms/download-jdk/microsoft-jdk-17-windows-x64.zip），解压到 `%LOCALAPPDATA%\Java\jdk-17`，将 `%LOCALAPPDATA%\Java\jdk-17\bin` 加入用户 PATH。

**编译 Cunzhi.java**：`javac -encoding UTF-8 Cunzhi.java`（Windows 默认 GBK，需指定 UTF-8）

**MSI 安装**：https://learn.microsoft.com/zh-cn/java/openjdk/download#openjdk-17

## PHP

- **下载**: https://windows.php.net/download/
- **选择**: PHP 8.4 VS17 x64 Thread Safe，ZIP 包
- **安装**: 解压到 `C:\php`，将 `C:\php` 加入系统 PATH
- **验证**: 新开终端执行 `php -v`

## C++ (gcc + libcurl)

**方式一：curl.se 预编译包（配合 Strawberry/MinGW）**

1. 下载：https://curl.se/windows/ 选择 `curl-*-win64-mingw.zip`
2. 解压到 `%LOCALAPPDATA%\curl-dev\`
3. 编译（PowerShell）：
   ```powershell
   $c = "$env:LOCALAPPDATA\curl-dev\curl-8.18.0_4-win64-mingw"
   g++ -o cunzhi.exe bin/cpp/cunzhi.cpp -I"$c\include" -L"$c\lib" "$c\lib\libcurl.dll.a" -lssl -lcrypto -lz -lws2_32 -lwldap32 -std=c++17
   ```
4. 将 `%LOCALAPPDATA%\curl-dev\curl-*\bin` 加入 PATH，或复制 `libcurl-x64.dll` 到可执行文件同目录

**方式二：MSYS2**

1. 安装 MSYS2：https://www.msys2.org/
2. 在 **MSYS2 UCRT64** 终端执行：`pacman -S mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-curl`
3. 编译：`g++ -o cunzhi.exe bin/cpp/cunzhi.cpp -lcurl -std=c++17`
