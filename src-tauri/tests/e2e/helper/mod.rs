//! e2e 基础设施：spawn `tauri-driver`、连接被测 app、数据隔离、失败截图。
//!
//! 运行方式（先 `cargo build`，且 `tauri-driver` 在 PATH 中）：
//!
//! ```bash
//! E2E=1 cargo test --test e2e -- --ignored
//! ```
//!
//! 数据隔离（硬约束）：每个用例把 `TROVE_DATA_DIR` 指向自己的 tempdir，
//! 被测 app 的 DB / 备份 / 资产全部落在该目录内，绝不触碰真实用户库。
//!
//! 平台约束：`tauri-driver` 直接驱动时仅支持 Linux / Windows（macOS 没有
//! WKWebView WebDriver 实现，官方文档明示）。macOS 上用例会跳过并打印原因，
//! 真实验证走 CI 的 `e2e` job（ubuntu）。

use std::future::Future;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use fantoccini::elements::Element;
use fantoccini::{Client, ClientBuilder, Locator};

/// 单个用例的总超时（CI 冷启动偏慢，给足余量）。
const CASE_TIMEOUT: Duration = Duration::from_secs(180);
/// 连接 tauri-driver / 启动 app 的等待上限。CI 首启要冷建 GTK/字体缓存，
/// 30s 不够（run 40703046029 实测 app 已起但握手在 30s 处被掐）。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(90);
/// trove-cli 单实例转发上限（app 卡死时不无限等）。
const CLI_TIMEOUT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(250);

/// 所有用例串行执行：`TROVE_DATA_DIR` 是进程级 env、`tauri-driver` 不宜多开、
/// 被测 app 受 single-instance 插件约束。串行是正确性要求，不是优化。
fn global_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn artifact_root() -> PathBuf {
    if let Ok(dir) = std::env::var("E2E_ARTIFACT_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("CARGO_TARGET_TMPDIR") {
        return PathBuf::from(dir).join("e2e-artifacts");
    }
    std::env::temp_dir().join("trove-e2e-artifacts")
}

/// 被测二进制：`E2E_BINARY` 优先，缺省 `target/debug/trove`。
fn resolve_trove_bin() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("E2E_BINARY") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("E2E_BINARY 指向的二进制不存在：{}", p.display()))
        };
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/trove");
    if p.is_file() {
        return Ok(p);
    }
    Err(format!(
        "被测二进制不存在：{}。先运行 `cargo build`，或用 E2E_BINARY 指定路径",
        p.display()
    ))
}

fn resolve_cli_bin() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("E2E_CLI") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("E2E_CLI 指向的二进制不存在：{}", p.display()))
        };
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/trove-cli");
    if p.is_file() {
        return Ok(p);
    }
    Err(format!(
        "trove-cli 不存在：{}。先运行 `cargo build`",
        p.display()
    ))
}

fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("bind :0")
        .local_addr()
        .expect("local_addr")
        .port()
}

fn wait_port(port: u16, what: &str) -> Result<(), String> {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(format!("等待 {what} 端口 {port} 超时"));
        }
        std::thread::sleep(POLL);
    }
}

/// 测试串行门 + 环境门。返回 `None` 表示用例应跳过（原因已打印）；
/// `Some(guard)` 期间持有全局串行锁。
pub fn acquire_lock(case: &str) -> Option<MutexGuard<'static, ()>> {
    match std::env::var_os("E2E") {
        Some(v) if !v.is_empty() => {}
        _ => {
            // `#[ignore]` 已挡住默认 `cargo test`；这里挡的是 `--include-ignored`。
            println!("[e2e:{case}] skip：未设置 E2E=1");
            return None;
        }
    }
    if !cfg!(any(target_os = "linux", target_os = "windows")) {
        println!(
            "[e2e:{case}] skip：tauri-driver 仅支持 Linux/Windows（macOS 无 WKWebView WebDriver 实现，\
             见 v2.tauri.app/develop/tests/webdriver）；真实验证走 CI e2e job"
        );
        return None;
    }
    let guard = global_lock().lock().unwrap_or_else(|e| e.into_inner());
    // fantoccini(rustls-tls) 不带默认 CryptoProvider：首个 TLS 连接会 panic。
    // 幂等安装 ring provider（已设置时返回 Err，忽略即可）。
    let _ = rustls::crypto::ring::default_provider().install_default();
    Some(guard)
}

pub struct Ctx {
    pub client: Client,
    /// 本用例的隔离数据目录（tempdir 内），断言 app 行为时可读取。
    pub data_dir: PathBuf,
    pub trove_bin: PathBuf,
    pub cli_bin: PathBuf,
    case: &'static str,
}

impl Ctx {
    /// 用例输出带步骤名（PRD R3）。
    pub fn step(&self, name: &str) {
        println!("[e2e:{}] 步骤：{name}", self.case);
    }

    /// 各 locator 方法返回 `String`（XPath 表达式），避免借用临时 format! 值；
    /// `find/wait_for/click` 统一接 `&str`。
    pub fn nav_link(&self, label: &str) -> String {
        format!("//nav//a[.//span[normalize-space()]='{label}']")
    }

    pub fn button_with_text(&self, text: &str) -> String {
        format!("//button[normalize-space()='{text}']")
    }

    /// 图标按钮无文本，只有 aria-label（如「新建分组或清单」）。
    pub fn button_aria(&self, aria: &str) -> String {
        format!("//button[@aria-label='{aria}']")
    }

    /// 详情面板标题输入：aside 内首个无 type 的 input（日期/时间均有 type，
    /// 标签输入有 placeholder；标题是文档序第一个）。
    pub fn title_input(&self) -> String {
        "(//aside//input[not(@type)])[1]".into()
    }

    /// 任务行（TaskRow 外层是 `div[role=button]`），按标题包含匹配。
    pub fn task_row(&self, title: &str) -> String {
        format!(
            "//div[@role='button'][contains(normalize-space(.),'{title}')]"
        )
    }

    /// 任务行内指定 aria-label 的按钮（如 完成 / 取消完成）。
    pub fn row_button(&self, title: &str, aria: &str) -> String {
        format!(
            "//div[@role='button'][contains(normalize-space(.),'{title}')]\
             //button[@aria-label='{aria}']"
        )
    }

    pub async fn wait_for(&self, xpath: &str, what: &str) -> Result<Element, String> {
        self.client
            .wait()
            .at_most(Duration::from_secs(20))
            .every(POLL)
            .for_element(Locator::XPath(xpath))
            .await
            .map_err(|e| format!("等待 {what} 超时: {e}"))
    }

    pub async fn find(&self, xpath: &str, what: &str) -> Result<Element, String> {
        self.client
            .find(Locator::XPath(xpath))
            .await
            .map_err(|e| format!("找不到 {what}: {e}"))
    }

    /// 点击 = 先 wait_for（20s 重试窗口，覆盖视图切换渲染）再点；
    /// WebKitGTK 下 React 重渲染会让已获节点失效（stale），重试重新等待再点。
    pub async fn click(&self, xpath: &str, what: &str) -> Result<(), String> {
        let mut last = String::new();
        for _ in 0..3 {
            let el = self.wait_for(xpath, what).await?;
            match el.click().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let msg = e.to_string();
                    last = msg.clone();
                    if msg.contains("not found") || msg.contains("stale") {
                        tokio::time::sleep(POLL).await;
                        continue;
                    }
                    return Err(format!("点击 {what} 失败: {msg}"));
                }
            }
        }
        Err(format!("点击 {what} 失败（重试耗尽）: {last}"))
    }

    /// 等待主窗口侧栏渲染（app 冷启动 + React 挂载）。
    pub async fn wait_main_ready(&self) -> Result<(), String> {
        self.wait_for(&self.nav_link("今日"), "主窗口侧栏（今日）")
            .await?;
        Ok(())
    }

    /// 新库首启会出现 onboarding 全屏遮罩；点击「开始使用」完成引导。
    /// 已完成引导的库里该按钮不存在，直接跳过。
    pub async fn dismiss_onboarding(&self) -> Result<(), String> {
        let xpath = self.button_with_text("开始使用");
        if let Ok(btn) = self.client.find(Locator::XPath(&xpath)).await {
            self.step("完成 onboarding 引导（开始使用）");
            btn.click().await.map_err(|e| format!("点击「开始使用」失败: {e}"))?;
        }
        Ok(())
    }

    /// 切换到包含指定元素的 WebDriver 窗口（Tauri 多窗口：main / quick）。
    pub async fn switch_to_window(&self, marker_xpath: &str, label: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let handles = self
                .client
                .windows()
                .await
                .map_err(|e| format!("枚举窗口失败: {e}"))?;
            for handle in handles {
                if self.client.switch_to_window(handle).await.is_err() {
                    continue;
                }
                if self.client.find(Locator::XPath(marker_xpath)).await.is_ok() {
                    self.step(&format!("切换到窗口 {label}"));
                    return Ok(());
                }
            }
            if Instant::now() > deadline {
                return Err(format!("未找到窗口 {label}（marker 未命中）"));
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// 切回主窗口（以侧栏为标志）。
    pub async fn switch_to_main_window(&self) -> Result<(), String> {
        self.switch_to_window(&self.nav_link("今日"), "main").await
    }

    /// 用 JS `dblclick` 事件触发双击（React onDoubleClick）。
    /// 说明：dnd-kit PointerSensor 不响应 WebDriver 合成拖拽（已实测），
    /// 但 dblclick 重命名不经过 dnd-kit，合成事件可触发。
    pub async fn double_click(&self, el: &Element) -> Result<(), String> {
        let args = vec![serde_json::to_value(el).map_err(|e| e.to_string())?];
        self.client
            .execute(
                "arguments[0].dispatchEvent(new MouseEvent('dblclick', {\
                 bubbles: true, cancelable: true, detail: 2}))",
                args,
            )
            .await
            .map(|_| ())
            .map_err(|e| format!("双击失败: {e}"))
    }

    /// 回车键（WebDriver Return 码点）。
    pub const ENTER: &'static str = "\u{E007}";

    /// 运行 `trove-cli`（继承 TROVE_DATA_DIR；--app 指向被测二进制）。
    /// CLI 不直接读写 DB，经运行中的 app（单实例转发）执行动作；
    /// 转发可能因 app 卡死而无限阻塞，必须带超时并强杀。
    pub fn run_cli(&self, args: &[&str]) -> Result<String, String> {
        let mut child = Command::new(&self.cli_bin)
            .arg("--app")
            .arg(&self.trove_bin)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("运行 trove-cli {args:?} 失败: {e}"))?;
        let deadline = Instant::now() + CLI_TIMEOUT;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if Instant::now() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!(
                            "trove-cli {args:?} 超时（{}s）：单实例转发未响应，app 可能已卡死",
                            CLI_TIMEOUT.as_secs()
                        ));
                    }
                    std::thread::sleep(POLL);
                }
                Err(e) => return Err(format!("trove-cli {args:?} 等待失败: {e}")),
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|e| format!("trove-cli {args:?} 读取输出失败: {e}"))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            return Err(format!("trove-cli {args:?} 退出失败：{combined}"));
        }
        Ok(combined)
    }
}

/// 一次性连接（用例 #4 重开校验会连第二次，数据目录不变）。
/// 必须带超时：app 起不来（如 CI 无显示服务器）时 WebKitWebDriver 的
/// 新会话请求永不返回，裸 await 会把整条流水线挂到 job 上限。
async fn connect(trove_bin: &PathBuf, port: u16) -> Result<Client, String> {
    let mut caps = serde_json::Map::new();
    caps.insert(
        "tauri:options".to_string(),
        serde_json::json!({ "application": trove_bin.display().to_string() }),
    );
    let url = format!("http://127.0.0.1:{port}");
    let mut builder =
        ClientBuilder::rustls().map_err(|e| format!("构建 WebDriver 客户端失败: {e}"))?;
    builder.capabilities(caps);
    // connect 借用 builder（&self），builder 必须先落 let，future 才能过 timeout await。
    let fut = builder.connect(&url);
    match tokio::time::timeout(CONNECT_TIMEOUT, fut).await {
        Ok(r) => r.map_err(|e| format!("连接 tauri-driver 失败: {e}")),
        Err(_) => Err(format!(
            "连接 tauri-driver 超时（{}s）：app 未能在期限内被拉起；\
             常见原因是无显示服务器（需 xvfb-run）或 app 启动即崩溃",
            CONNECT_TIMEOUT.as_secs()
        )),
    }
}

/// 持有：串行锁、tempdir、TROVE_DATA_DIR 注入、tauri-driver 子进程。
/// `finish` 保证逆序清理；tempdir 存活到用例结束（隔离硬约束）。
pub struct Harness {
    ctx: Ctx,
    extra_clients: Vec<Client>,
    driver: Child,
    driver_log: PathBuf,
    artifact_dir: PathBuf,
    port: u16,
    _tempdir: tempfile::TempDir,
    _guard: Option<MutexGuard<'static, ()>>,
}

impl Harness {
    /// 完整准备：锁 + env 门 + tempdir + TROVE_DATA_DIR + tauri-driver + 首连。
    /// `Ok(None)` = 用例被跳过（已打印原因）。
    pub async fn acquire(case: &'static str) -> Result<Option<Harness>, String> {
        let Some(guard) = acquire_lock(case) else {
            return Ok(None);
        };
        let trove_bin = resolve_trove_bin()?;
        let cli_bin = resolve_cli_bin()?;
        let tempdir = tempfile::tempdir().map_err(|e| format!("创建 tempdir 失败: {e}"))?;
        let data_dir = tempdir.path().to_path_buf();
        // env 注入：tauri-driver → WebKitWebDriver → app 子进程逐级继承；
        // CLI 子进程同样继承。锁保证串行期间无其他用例改写该 env。
        std::env::set_var("TROVE_DATA_DIR", &data_dir);

        let artifact_dir = artifact_root().join(case);
        std::fs::create_dir_all(&artifact_dir).map_err(|e| format!("创建 artifact 目录失败: {e}"))?;

        let port = free_port();
        let native_port = free_port();
        let driver_bin = std::env::var("E2E_TAURI_DRIVER").unwrap_or_else(|_| "tauri-driver".into());
        let driver_log = artifact_dir.join("tauri-driver.log");
        let log = std::fs::File::create(&driver_log)
            .map_err(|e| format!("创建 tauri-driver 日志失败: {e}"))?;
        let driver = Command::new(&driver_bin)
            .arg("--port")
            .arg(port.to_string())
            .arg("--native-port")
            .arg(native_port.to_string())
            .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|e| {
                format!(
                    "spawn {driver_bin} 失败: {e}（请 `cargo install tauri-driver --locked`，\
                     或用 E2E_TAURI_DRIVER 指定路径）"
                )
            })?;
        wait_port(port, "tauri-driver")?;

        let client = connect(&trove_bin, port).await?;
        println!("[e2e:{case}] 已连接（TROVE_DATA_DIR={}）", data_dir.display());
        Ok(Some(Harness {
            ctx: Ctx {
                client,
                data_dir,
                trove_bin,
                cli_bin,
                case,
            },
            extra_clients: Vec::new(),
            driver,
            driver_log,
            artifact_dir,
            port,
            _tempdir: tempdir,
            _guard: Some(guard),
        }))
    }

    /// 第二次连接同一数据目录（用例 #4「重开保持」）。
    pub async fn reconnect(&mut self) -> Result<(), String> {
        // 旧会话随旧 Client drop 关闭（fantoccini drop 即关会话），
        // app 随会话退出；再起新会话复用同一 tauri-driver。
        wait_app_exit(&self.ctx.trove_bin);
        let port = self.driver_port()?;
        let client = connect(&self.ctx.trove_bin, port).await?;
        self.ctx.client = client;
        Ok(())
    }

    fn driver_port(&self) -> Result<u16, String> {
        Ok(self.port)
    }

    /// 运行用例主体（带步骤名输出与总超时）。
    pub async fn run<'a, F, Fut>(&'a self, f: F) -> Result<(), String>
    where
        F: FnOnce(&'a Ctx) -> Fut,
        Fut: Future<Output = Result<(), String>> + 'a,
    {
        let case = self.ctx.case;
        match tokio::time::timeout(CASE_TIMEOUT, f(&self.ctx)).await {
            Ok(r) => r,
            Err(_) => Err(format!("用例总超时（{}s）", CASE_TIMEOUT.as_secs())),
        }
    }

    /// 失败留证（截图 + 页面源码 + tauri-driver 日志已在 artifact 目录），
    /// 然后逆序清理；失败时 panic 使该用例置红。
    pub async fn finish(mut self, result: Result<(), String>) {
        let case = self.ctx.case;
        if let Err(err) = &result {
            self.save_failure_artifacts(err).await;
        }
        for c in self.extra_clients.drain(..) {
            let _ = tokio::time::timeout(Duration::from_secs(5), c.close()).await;
        }
        let _ = tokio::time::timeout(Duration::from_secs(10), self.ctx.client.close()).await;
        let _ = self.driver.kill();
        let _ = self.driver.wait();
        wait_app_exit(&self.ctx.trove_bin);
        // 兜底：WebKitWebDriver 偶发不随会话结束而退出 app；
        // e2e 环境本就不允许有其他 Trove 实例（single-instance 约束）。
        let _ = Command::new("pkill")
            .args(["-f", self.ctx.trove_bin.to_string_lossy().as_ref()])
            .output();
        std::env::remove_var("TROVE_DATA_DIR");
        drop(self._tempdir);
        drop(self._guard.take());

        if let Err(err) = result {
            panic!("[e2e:{case}] 失败：{err}\nartifact 目录：{}", self.artifact_dir.display());
        }
        println!("[e2e:{case}] 通过");
    }

    async fn save_failure_artifacts(&self, err: &str) {
        let dir = self.artifact_dir.clone();
        let _ = std::fs::write(dir.join("error.txt"), err);
        // app 可能已死：截图/取源码也要限时，别让留证变成二次挂起。
        let shot =
            tokio::time::timeout(Duration::from_secs(5), self.ctx.client.screenshot()).await;
        if let Ok(Ok(png)) = shot {
            let _ = std::fs::write(dir.join("failure.png"), png);
        }
        let html = tokio::time::timeout(Duration::from_secs(5), self.ctx.client.source()).await;
        if let Ok(Ok(page)) = html {
            let _ = std::fs::write(dir.join("page.html"), page);
        }
        eprintln!(
            "[e2e:{}] 失败，artifact：{}",
            self.ctx.case,
            dir.display()
        );
    }
}

fn wait_app_exit(trove_bin: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let alive = Command::new("pgrep")
            .arg("-f")
            .arg(trove_bin.to_string_lossy().as_ref())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !alive {
            return;
        }
        std::thread::sleep(POLL);
    }
}
