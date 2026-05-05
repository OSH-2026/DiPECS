//! Daemon 进程管理
//!
//! 提供 daemonize (后台化) 和信号处理。

use std::process;

/// 将当前进程 daemon 化 (fork + setsid)
///
/// 仅在 Linux 上有效。在非 Linux 平台 (如 macOS) 上直接返回。
pub fn daemonize() {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: 调用 POSIX fork() 和 setsid() 完成 daemon 化。
        // fork 后父进程退出, 子进程成为新的会话 leader。
        unsafe {
            let pid = libc::fork();
            if pid < 0 {
                tracing::error!("fork failed: {}", std::io::Error::last_os_error());
                process::exit(1);
            }
            if pid > 0 {
                // 父进程: 退出
                process::exit(0);
            }

            // 子进程: 成为新的会话 leader
            if libc::setsid() < 0 {
                tracing::error!("setsid failed: {}", std::io::Error::last_os_error());
                process::exit(1);
            }

            // 将工作目录切换到根目录
            let _ = libc::chdir(c"/".as_ptr());

            // 关闭标准文件描述符
            let _ = libc::close(0);
            let _ = libc::close(1);
            let _ = libc::close(2);

            // 重定向到 /dev/null
            let fd = libc::open(c"/dev/null".as_ptr(), libc::O_RDWR);
            assert_eq!(fd, 0);
            let fd = libc::dup(0);
            assert_eq!(fd, 1);
            let fd = libc::dup(0);
            assert_eq!(fd, 2);
        }
    }

    tracing::info!("dipecsd daemon started (pid={})", process::id());
}

/// 安装信号处理器 (SIGTERM, SIGINT)
///
/// 使用 tokio::signal 实现, 零 unsafe, 零平台特定代码。
/// 收到信号后通过 broadcast channel 通知主循环优雅退出。
pub fn install_signal_handlers() -> tokio::sync::broadcast::Receiver<()> {
    let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

    // SIGINT (Ctrl+C) — 跨平台
    let tx_ctrlc = tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("received SIGINT, initiating graceful shutdown");
            let _ = tx_ctrlc.send(());
        }
    });

    // SIGTERM — Unix only
    #[cfg(unix)]
    {
        let tx_term = tx.clone();
        tokio::spawn(async move {
            if let Ok(mut sigterm) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            {
                sigterm.recv().await;
                tracing::info!("received SIGTERM, initiating graceful shutdown");
                let _ = tx_term.send(());
            }
        });
    }

    rx
}
