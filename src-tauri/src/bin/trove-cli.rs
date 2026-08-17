use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};
use trove_lib::cli_protocol::{dry_run_outcome, CliDispatchRequest};
use trove_lib::domain::{
    parse_trove_url, ActionOutcome, CreateMemoryInput, CreateReminderInput, CreateTaskInput,
    WorkbenchAction,
};

#[derive(Parser)]
#[command(name = "trove-cli", about = "Trove 本地 CLI — 通过运行中的应用执行工作台动作")]
struct Cli {
    /// 输出 JSON（默认人类可读摘要）
    #[arg(long, global = true)]
    json: bool,

    /// 仅预览，不调用应用
    #[arg(long, global = true)]
    dry_run: bool,

    /// Trove 可执行文件路径（默认自动探测）
    #[arg(long, global = true)]
    app: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 打开「今日」页
    Today,
    /// 打开「收件箱」
    Inbox,
    /// 打开快速搜索
    Search {
        /// 搜索词
        query: String,
    },
    /// 创建任务 / 提醒 / 记忆
    Create {
        #[arg(value_enum)]
        kind: CreateKind,
        #[arg(long)]
        title: String,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        due_date: Option<String>,
        #[arg(long)]
        fire_at: Option<String>,
        /// 跳过确认对话框，直接写入（高风险）
        #[arg(long)]
        yes: bool,
    },
    /// 完成任务
    Complete {
        task_id: String,
        #[arg(long)]
        yes: bool,
    },
    /// 直接 dispatch JSON 动作（高级）
    Raw {
        #[arg(long)]
        json: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CreateKind {
    Task,
    Reminder,
    Memory,
}

impl CreateKind {
    fn url_type(self) -> &'static str {
        match self {
            CreateKind::Task => "task",
            CreateKind::Reminder => "reminder",
            CreateKind::Memory => "memory",
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Today => invoke_url(&cli, "trove://today"),
        Commands::Inbox => invoke_url(&cli, "trove://inbox"),
        Commands::Search { query } => {
            let encoded = urlencoding_encode(query);
            invoke_url(&cli, &format!("trove://search?q={encoded}"))
        }
        Commands::Create {
            kind,
            title,
            notes,
            due_date,
            fire_at,
            yes,
        } => {
            if *yes {
                let action = build_create_action(
                    *kind,
                    title.clone(),
                    notes.clone(),
                    due_date.clone(),
                    fire_at.clone(),
                )?;
                dispatch_action(&cli, action)
            } else {
                let mut url = format!(
                    "trove://create?type={}&title={}",
                    kind.url_type(),
                    urlencoding_encode(title)
                );
                if let Some(n) = notes {
                    url.push_str(&format!("&notes={}", urlencoding_encode(n)));
                }
                if let Some(d) = due_date {
                    url.push_str(&format!("&dueDate={}", urlencoding_encode(d)));
                }
                if let Some(f) = fire_at {
                    url.push_str(&format!("&fireAt={}", urlencoding_encode(f)));
                }
                invoke_url(&cli, &url)
            }
        }
        Commands::Complete { task_id, yes } => {
            if !*yes {
                return Err("完成任务须加 --yes（或使用应用内操作）".into());
            }
            let id = uuid::Uuid::parse_str(task_id)
                .map_err(|_| "task_id 不是有效 UUID".to_string())?;
            dispatch_action(
                &cli,
                WorkbenchAction::CompleteTask {
                    task_id: id,
                    confirmed: true,
                },
            )
        }
        Commands::Raw { json } => {
            let action: WorkbenchAction =
                serde_json::from_str(json).map_err(|e| format!("动作 JSON 无效: {e}"))?;
            dispatch_action(&cli, action)
        }
    }
}

fn build_create_action(
    kind: CreateKind,
    title: String,
    notes: Option<String>,
    due_date: Option<String>,
    fire_at: Option<String>,
) -> Result<WorkbenchAction, String> {
    match kind {
        CreateKind::Task => Ok(WorkbenchAction::CreateTask {
            input: CreateTaskInput {
                title,
                notes,
                priority: None,
                list_id: None,
                due_date,
                due_time: None,
                tag_names: None,
            },
            confirmed: true,
        }),
        CreateKind::Reminder => {
            let fire_at = fire_at.ok_or_else(|| "提醒须指定 --fire-at".to_string())?;
            Ok(WorkbenchAction::CreateReminder {
                input: CreateReminderInput {
                    title,
                    notes,
                    task_id: None,
                    fire_at,
                    recurrence: None,
                    timezone: None,
                    end_at: None,
                },
                confirmed: true,
            })
        }
        CreateKind::Memory => Ok(WorkbenchAction::CreateMemory {
            input: CreateMemoryInput {
                title,
                body: notes,
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            },
            confirmed: true,
        }),
    }
}

fn invoke_url(cli: &Cli, url: &str) -> Result<(), String> {
    if cli.dry_run {
        let action = parse_trove_url(url).map_err(|e| e.to_string())?;
        let wb: WorkbenchAction = action.into();
        return print_outcome(cli, dry_run_outcome(&wb));
    }
    let app = resolve_app_path(cli.app.as_deref())?;
    let status = Command::new(&app)
        .arg(url)
        .status()
        .map_err(|e| format!("无法启动 {}: {e}", app.display()))?;
    if !status.success() {
        return Err(format!("应用退出码 {}", status.code().unwrap_or(-1)));
    }
    if cli.json {
        println!("{}", serde_json::json!({ "ok": true, "url": url }));
    } else {
        println!("已发送 {url}");
    }
    Ok(())
}

fn dispatch_action(cli: &Cli, action: WorkbenchAction) -> Result<(), String> {
    if cli.dry_run {
        return print_outcome(cli, dry_run_outcome(&action));
    }

    let response_path = std::env::temp_dir().join(format!(
        "trove-cli-{}.json",
        uuid::Uuid::new_v4()
    ));
    let request = CliDispatchRequest {
        action,
        options: CliDispatchRequest::cli_options(false),
        response_path: Some(response_path.to_string_lossy().into_owned()),
    };
    let arg = request.encode().map_err(|e| e.to_string())?;
    let app = resolve_app_path(cli.app.as_deref())?;
    Command::new(&app)
        .arg(&arg)
        .spawn()
        .map_err(|e| format!("无法启动 {}: {e}", app.display()))?;

    match wait_for_response(&response_path, Duration::from_secs(15)) {
        Some(outcome) => {
            let _ = std::fs::remove_file(&response_path);
            print_outcome(cli, outcome)
        }
        None => Err(format!(
            "等待应用响应超时（{}）",
            response_path.display()
        )),
    }
}

fn wait_for_response(path: &Path, timeout: Duration) -> Option<ActionOutcome> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if path.is_file() {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(outcome) = serde_json::from_str(&text) {
                    return Some(outcome);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

fn print_outcome(cli: &Cli, outcome: ActionOutcome) -> Result<(), String> {
    if cli.json {
        let json = serde_json::to_string_pretty(&outcome).map_err(|e| e.to_string())?;
        println!("{json}");
    } else {
        println!("{}", format_outcome_human(&outcome));
    }
    if matches!(outcome, ActionOutcome::Rejected { .. }) {
        return Err("动作被拒绝".into());
    }
    Ok(())
}

fn format_outcome_human(outcome: &ActionOutcome) -> String {
    match outcome {
        ActionOutcome::Navigated { path } => format!("已导航到 {path}"),
        ActionOutcome::SearchOpened { query } => format!("已打开搜索：{query}"),
        ActionOutcome::CreatePreviewPending { title, .. } => format!("等待确认创建：{title}"),
        ActionOutcome::DryRun { description } => format!("[dry-run] {description}"),
        ActionOutcome::TaskCreated { task } => format!("已创建任务 {}", task.id),
        ActionOutcome::ReminderCreated { reminder } => format!("已创建提醒 {}", reminder.id),
        ActionOutcome::MemoryCreated { memory } => format!("已创建记忆 {}", memory.id),
        ActionOutcome::TaskCompleted { task } => format!("已完成任务 {}", task.id),
        ActionOutcome::Rejected { reason } => format!("拒绝：{reason}"),
    }
}

fn resolve_app_path(override_path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    if let Ok(env) = std::env::var("TROVE_APP") {
        return Ok(PathBuf::from(env));
    }
    #[cfg(target_os = "macos")]
    {
        let mac = PathBuf::from("/Applications/Trove.app/Contents/MacOS/trove");
        if mac.is_file() {
            return Ok(mac);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(name) = exe.file_name().and_then(|n| n.to_str()) {
            if name == "trove-cli" {
                let sibling = exe.parent().map(|p| p.join("trove"));
                if let Some(path) = sibling {
                    if path.is_file() {
                        return Ok(path);
                    }
                }
            }
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/trove");
    if dev.is_file() {
        return Ok(dev);
    }
    Err("找不到 Trove 可执行文件；请设置 TROVE_APP 或 --app".into())
}

fn urlencoding_encode(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
