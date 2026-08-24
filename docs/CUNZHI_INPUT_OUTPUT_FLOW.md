# Cunzhi input.md / output.md 流程说明

## 核心问题

1. **AI 能否阅读 input.md？** 能。AI 在调用 cunzhi 脚本后，若返回 `KeepGoing=true`，会输出 `input_file: ~/.cunzhi/{port}/input.md`，AI 需读取该文件获取用户指令。
2. **用户能否通过 output.md 与 AI 对话？** 能。AI 将消息写入 output.md，调用 cunzhi 脚本后，iterate GUI 会显示 output.md 内容，用户输入后写入 input.md。

## 文件路径

| 文件 | 路径 | 谁写入 | 谁读取 |
|------|------|--------|--------|
| output.md | `~/.cunzhi/{port}/output.md` | AI | cunzhi 脚本（发送到 GUI） |
| input.md | `~/.cunzhi/{port}/input.md` | cunzhi 脚本（用户输入） | AI |

- Windows: `%USERPROFILE%\.cunzhi\{port}\`
- 示例: `C:\Users\example\.cunzhi\5311\output.md`

## 完整流程

```
1. AI 写入 ~/.cunzhi/{port}/output.md（任务摘要/问题）
2. AI 调用 cunzhi 脚本: python bin/cunzhi.py {port}
3. 脚本读取 output.md，POST 到 iterate --serve 的 /api/dialog
4. iterate 弹出 GUI 显示 output.md 内容
5. 用户在 GUI 中输入并提交
6. 脚本将用户输入写入 ~/.cunzhi/{port}/input.md
7. 脚本输出: KeepGoing=true 和 input_file: .../input.md
8. AI 读取 input.md 获取用户指令，继续对话
```

## 端口选择

- **5330**：推荐用于 cunzhi 脚本对话，避开 MCP 占用的 5311
- **5311**：常用默认端口，可能被 MCP/网关占用
- **5310**：备选，避免与 MCP 冲突

**启动 iterate 服务**（5330 避开 MCP 的 5311）：
```powershell
# 桌面版
Start-Process "$env:USERPROFILE\Desktop\iterate.exe" -ArgumentList "--serve", "--port", "5330"
```

## 六种语言调用命令汇总

| 语言 | 调用命令 | 备注 |
|------|----------|------|
| Python | `python bin/cunzhi.py 5330` | 从 output.md 读取；或 `--message "xxx"` 直接传消息 |
| Go | `go run ./bin 5330` | 项目根目录执行 |
| Node.js | `node bin/cunzhi.cjs 5330` | |
| Java | `cd bin && javac -encoding UTF-8 Cunzhi.java && java Cunzhi 5330` | 需先编译 |
| PHP | `php bin/cunzhi.php 5330` | |
| C++ | `.\cunzhi.exe 5330` | 需先编译，DLL 需在 PATH |

**带参数调用**（不读 output.md，直接传消息）：
```powershell
python bin/cunzhi.py 5330 --message "测试消息" --workspace "c:\Users\example\iterate"
go run ./bin 5330 --message "测试消息"
node bin/cunzhi.cjs 5330 --message "测试消息"
java -cp bin Cunzhi 5330 --message "测试消息"
php bin/cunzhi.php 5330 --message "测试消息"
.\cunzhi.exe 5330 --message "测试消息"
```

## Python 流程验证

```powershell
# 1. 确保 iterate 已启动
iterate --serve --port 5310

# 2. 在另一终端，AI 写入 output.md
$dir = "$env:USERPROFILE\.cunzhi\5310"
New-Item -ItemType Directory -Path $dir -Force
Set-Content -Path "$dir\output.md" -Value "## 测试`n请回复「收到」" -Encoding UTF8

# 3. 调用 cunzhi（会阻塞等待 GUI 回复）
python bin/cunzhi.py 5310

# 4. 用户在 GUI 回复后，脚本输出 input_file 路径
# 5. AI 读取该路径的 input.md
```

## 底层逻辑

所有语言实现相同协议：
- 读取 output.md（或 --message 参数）→ POST /api/dialog
- 接收 GUI 返回 → 写入 input.md
- 输出 input_file 路径供 AI 读取

Python 走通后，其他语言在相同 iterate 服务下行为一致。
