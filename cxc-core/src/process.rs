use std::ffi::OsStr;
use std::process::Command;

/// 创建子进程命令。
///
/// Windows 的 GUI 进程启动控制台程序（`wsl.exe`、`cmd.exe` 等）时，
/// 系统默认会新建一个控制台窗口。这里统一加上 `CREATE_NO_WINDOW`，
/// 避免窗口在启动探测、刷新托盘、写入 WSL 配置时闪现。
pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = Command::new(program);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(program)
    }
}
