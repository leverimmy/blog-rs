---
title: VSCode 连接 SSH 时 getcwd 报错的解决方法
tags:
  - VSCode
  - SSH
  - 服务器
categories:
  - 技术
mathjax: true
toc: true
date: 2026-07-14 17:11:23
password:
id: How-to-Fix-getcwd-Error-in-VSCode-Remote-SSH
---

TL;DR：服务器的 File System（FS）可能出现了异常，导致 SSH 会话继承的 Current Working Directory（CWD）失效，VSCode Server 因为无法执行 `getcwd()` 而退出。在本机的 SSH 配置中使用 `RemoteCommand cd /tmp && exec /bin/bash`，让 VSCode Server 从有效目录启动，即可绕过这一问题。

<!--more-->

## 问题描述

我在使用本机的 VSCode 通过 [Remote - SSH](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-ssh) 连接服务器时，始终无法建立远程连接。但是，直接在终端中使用 `ssh` 可以正常登录服务器，密码认证也已经成功。即便 SSH 登陆成功，在执行各种命令的时候终端仍会报错：

```bash
sudo apt install zsh
```

![`apt` 指令报错](/gallery/How-to-Fix-getcwd-Error-in-VSCode-Remote-SSH/bash-error.png)

从日志来看，VSCode Server 已经完成安装、启动并监听端口：

```
Authenticated to a.b.c.d ([a.b.c.d]:6988) using "password".
Remote server is listening on port 34311
Server started
```

真正的问题出现在 VSCode Server 读取当前工作目录时：

```
shell-init: error retrieving current directory: getcwd: cannot access parent directories: No such file or directory
sh: 0: getcwd() failed: No such file or directory
Error: ENOENT: process.cwd failed with error no such file or directory, uv_cwd
```

{% note info no-icon VSCode Remote - SSH 完整日志（已脱敏，节选） %}
```
[14:45:19.952] stderr> Authenticated to <HOST> ([<HOST>]:<PORT>) using "password".
[14:45:20.047] stderr> client_global_hostkeys_prove_confirm: server gave bad signature for RSA key 0: incorrect signature

...

[14:45:20.050] stderr> shell-init: error retrieving current directory: getcwd: cannot access parent directories: No such file or directory
[14:45:20.130] stderr> job-working-directory: error retrieving current directory: getcwd: cannot access parent directories: No such file or directory
[14:45:20.171] > /bin/bash
[14:45:20.171] Parent Shell: bash

...

[14:45:20.261] > Found existing installation at <REMOTE_HOME>/.vscode-server...
> Starting VS Code CLI...
[14:45:20.269] > Spawned remote CLI
> Waiting for server log...

...

[14:45:20.306] Remote server is listening on port <REMOTE_PORT>

...

[14:45:20.478] [server] Checking <REMOTE_HOME>/.vscode-server/... for a running server...
[14:45:20.478] [server] Installing and setting up Visual Studio Code Server...
[14:45:20.479] [server] Server setup complete
[14:45:20.482] [server] Starting server...

...

[14:45:20.489] [server] sh: 0: getcwd() failed: No such file or directory
[14:45:20.568] [server] Error: ENOENT: process.cwd failed with error no such file or directory, the current working directory was likely removed without changing the working directory, uv_cwd

...

[14:45:20.573] [server] {
[14:45:20.573] [server]   errno: -2,
[14:45:20.573] [server]   code: 'ENOENT',
[14:45:20.574] [server]   syscall: 'uv_cwd'
[14:45:20.574] [server] }

...

[14:45:20.616] SSH Resolver called for "ssh-remote+<HOST>", attempt 3, (Reconnection)

...

[14:45:20.619] Running server is stale. Ignoring
```
{% endnote %}

另外，观察到在服务器的 VNC 界面中，

```bash
cd ~
code .
```

会启动失败，报错信息如下：

```
Error: ENOENT: process.cwd failed with error no such file or directory, the current working directory was likely removed without changing the working directory, uv_cwd
    at process.wrappedCwd [as cwd] (node:internal/bootstrap/switches/does_own_process_state:142:28)
    at Gs (file:///usr/share/code/resources/app/out/cli.js:433:75892)
    at file:///usr/share/code/resources/app/out/cli.js:433:75997
    at ModuleJob.run (node:internal/modules/esm/module_job:439:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:633:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:116:5) {
  errno: -2,
  code: 'ENOENT',
  syscall: 'uv_cwd'
}
```

![打开 VSCode 失败](/gallery/How-to-Fix-getcwd-Error-in-VSCode-Remote-SSH/code-error.png)

但

```bash
cd ~/Desktop
code ~
```

或者：

```bash
cd /tmp
code ~
```

就没有问题。

![成功打开 VSCode](/gallery/How-to-Fix-getcwd-Error-in-VSCode-Remote-SSH/success.png)

## 原因分析

`getcwd()` 用来获取进程的当前工作目录。正常情况下，即使目录为空，它也不应该报错。结合这台服务器上的其他现象，我更倾向于认为根本原因是服务器的 FS 出现了异常，CWD 失效只是它直接表现出来的结果。

```bash
cd ~/Desktop
code ~
```

或者：

```bash
cd /tmp
code ~
```

这里的 `~` 是传给 `code` 的参数，表示需要打开的目录；`~/Desktop` 或 `/tmp` 才是 VSCode 进程启动时继承的 CWD。打开的目录仍然是 `~`，但从有效的 CWD 启动后，VSCode 就能正常运行。因此，VSCode 的安装和用户主目录本身都没有问题。

一种可能的情况是：用户目录或它上层的挂载点被重新创建、替换或重新挂载后，SSH 网关、远程容器或者 VNC 服务的父进程仍然持有旧目录。子进程继续继承这个已经无法在当前 FS 中解析的目录，于是 `getcwd()` 返回 `ENOENT`。

这也解释了为什么普通 SSH 看起来仍然可以使用。Shell 中的 `$PWD` 只是一个环境变量，终端提示符即使仍然显示 `~`，也不代表内核中的 CWD 可以被正确解析。一些不依赖 CWD 的命令可以继续执行（比如 `ls`、`cd`、`pwd`），但 Node.js 启动 VSCode Server 时会调用 `process.cwd()`，因此直接退出。

## 解决方案

### 修改 SSH 配置

在本机的 `~/.ssh/config` 中增加一个专门供 VSCode 使用的 Host：

```sshconfig
Host alias-of-the-server
  HostName a.b.c.d
  User xxx
  Port yyyy
  RemoteCommand cd /tmp && exec /bin/bash
```

重点在于：

```sshconfig
RemoteCommand cd /tmp && exec /bin/bash
```

SSH 连接建立后，它会先切换到一定存在的 `/tmp`，然后启动 Bash。VSCode 随后发送到远端的安装和启动脚本都会由这个 Bash 执行，因此 VSCode Server 会继承 `/tmp` 作为有效的 CWD。

之所以使用一个新的 Host，而不是直接修改原来的配置，是因为 `RemoteCommand` 会影响该 Host 下的所有 SSH 连接。创建一个专用别名，可以避免影响普通的交互式 SSH。

使用下面的命令验证配置：

```bash
printf 'pwd\nexit\n' | ssh -T alias-of-the-server
```

如果最后输出 `/tmp`，就说明后续命令已经从有效目录启动。

### 修改 VSCode 设置

在 VSCode 中打开 `Preferences: Open User Settings (JSON)`，加入：

```json
{
  "remote.SSH.useLocalServer": true,
  "remote.SSH.enableRemoteCommand": true,
  "remote.SSH.useExecServer": false
}
```

## 总结

这个问题的直接原因是 VSCode Server 继承了失效的 CWD，深层原因则更可能是服务器 FS 或挂载状态异常。`RemoteCommand` 并没有修复 FS，而是在启动 VSCode Server 前先切换到 `/tmp`，绕开了失效目录。
