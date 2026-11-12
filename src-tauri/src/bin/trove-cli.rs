use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};
use trove_lib::cli_protocol::{dry_run_outcome, CliDispatchRequest};
use trove_lib::domain::{
    parse_trove_url, ActionListHit, ActionOutcome, ActionUpdateTaskInput, CreateMemoryInput,
    CreateReminderInput, CreateTaskInput, EntityId, TaskDeleteDisposition, TaskPriority,
    TaskQuery, TaskStatus, WorkbenchAction,
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
        /// 任务清单（名称或 uuid，仅 task；需 --yes）
        #[arg(long)]
        list: Option<String>,
        /// 任务优先级 none|low|medium|high（仅 task；需 --yes）
        #[arg(long)]
        priority: Option<String>,
        /// 任务标签，可多次传入（仅 task；需 --yes）
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// 父任务 uuid（仅 task；需 --yes）
        #[arg(long)]
        parent: Option<String>,
        /// 任务截止时刻 HH:MM（仅 task；需 --yes）
        #[arg(long)]
        due_time: Option<String>,
        /// 跳过确认对话框，直接写入（高风险）
        #[arg(long)]
        yes: bool,
    },
    /// 从 NDJSON 批量创建任务（每行一条；上限 100）
    CreateBatch {
        /// NDJSON 文件路径；缺省读 stdin
        #[arg(long)]
        file: Option<PathBuf>,
        /// 全局清单（名称或 uuid），行内 list 字段优先
        #[arg(long)]
        list: Option<String>,
        /// 直接写入（必须）
        #[arg(long)]
        yes: bool,
    },
    /// 查看任务详情（完整 JSON）
    Show { task_id: String },
    /// 更新任务（只传需要修改的字段）
    Update {
        task_id: String,
        #[arg(long)]
        title: Option<String>,
        /// 新备注；传空字符串清空
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        due_date: Option<String>,
        #[arg(long)]
        due_time: Option<String>,
        /// 清空截止日期与时刻（优先于 due flags）
        #[arg(long)]
        clear_due: bool,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        list: Option<String>,
        /// 标签整体替换，可多次传入；不传则保持不变
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// 直接写入（必须）
        #[arg(long)]
        yes: bool,
    },
    /// 重开已完成任务
    Reopen {
        task_id: String,
        #[arg(long)]
        yes: bool,
    },
    /// 归档任务
    Archive {
        task_id: String,
        #[arg(long)]
        yes: bool,
    },
    /// 删除任务（必须显式指定子任务处置）
    Delete {
        task_id: String,
        /// 子任务处置：cascade=一并删除，promote=提升为顶层
        #[arg(long, value_enum)]
        children: DeleteChildren,
        #[arg(long)]
        yes: bool,
    },
    /// 延后 N 天
    Postpone {
        task_id: String,
        #[arg(long)]
        days: i64,
        #[arg(long)]
        yes: bool,
    },
    /// 设置 / 清除延期显示日期
    Defer {
        task_id: String,
        /// 延期显示日期 YYYY-MM-DD
        #[arg(long)]
        at: Option<String>,
        /// 清除延期显示（与 --at 互斥）
        #[arg(long, conflicts_with = "at")]
        clear: bool,
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
    /// 查询数据（供快捷指令 / 脚本读取，需应用在运行）
    Query {
        #[command(subcommand)]
        target: QueryTarget,
    },
    /// 导出任务 CSV 到文件（经运行中的应用，与设置页导出同源）
    ExportCsv {
        /// 输出文件路径（缺省 ./trove-tasks-<日期>.csv）
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum QueryTarget {
    /// 今日逾期 / 到期 / 重点 / 提醒
    Today,
    /// 逾期任务
    Overdue {
        #[arg(long)]
        limit: Option<i64>,
    },
    /// 收件箱未完成任务
    Inbox {
        #[arg(long)]
        limit: Option<i64>,
    },
    /// 指定清单的未完成任务
    List {
        list_id: String,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// 按条件查询任务（JSON 输出为任务摘要）
    Tasks {
        /// todo | completed | archived
        #[arg(long)]
        status: Option<String>,
        /// 清单名称或 uuid
        #[arg(long)]
        list: Option<String>,
        /// 标题/备注搜索词
        #[arg(long)]
        search: Option<String>,
        /// none | low | medium | high
        #[arg(long)]
        priority: Option<String>,
        /// 截止日期下界 YYYY-MM-DD（含）
        #[arg(long)]
        due_from: Option<String>,
        /// 截止日期上界 YYYY-MM-DD（含）
        #[arg(long)]
        due_to: Option<String>,
        /// 父任务 uuid（查子任务）
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        offset: Option<i64>,
    },
    /// 列出全部清单（id / 名称 / 未完成数）
    Lists,
    /// 搜索记忆
    Memories {
        query: String,
        #[arg(long)]
        limit: Option<i64>,
    },
    /// 收藏的文本片段
    Snippets {
        query: Option<String>,
        #[arg(long)]
        limit: Option<i64>,
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

#[derive(Clone, Copy, ValueEnum)]
enum DeleteChildren {
    /// 一并删除全部子任务
    Cascade,
    /// 删除本任务，子任务提升为顶层
    Promote,
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
            list,
            priority,
            tags,
            parent,
            due_time,
            yes,
        } => {
            let has_task_extras = list.is_some()
                || priority.is_some()
                || !tags.is_empty()
                || parent.is_some()
                || due_time.is_some();
            if *yes {
                if has_task_extras && !matches!(kind, CreateKind::Task) {
                    return Err("--list/--priority/--tag/--parent/--due-time 仅支持 create task".into());
                }
                let mut action = build_create_action(
                    *kind,
                    title.clone(),
                    notes.clone(),
                    due_date.clone(),
                    fire_at.clone(),
                )?;
                if let WorkbenchAction::CreateTask { input, confirmed: true } = &mut action {
                    input.priority = priority
                        .as_deref()
                        .map(parse_priority)
                        .transpose()?;
                    input.tag_names = (!tags.is_empty())
                        .then(|| tags.clone())
                        .filter(|_| matches!(kind, CreateKind::Task));
                    input.parent_id = parent
                        .as_deref()
                        .map(|p| parse_uuid(p, "parent"))
                        .transpose()?;
                    input.due_time = due_time.clone().filter(|_| matches!(kind, CreateKind::Task));
                    if let Some(spec) = list {
                        input.list_id = Some(resolve_list_ref(&cli, spec)?);
                    }
                }
                dispatch_action(&cli, action)
            } else {
                if has_task_extras {
                    return Err("--list/--priority/--tag/--parent/--due-time 需要搭配 --yes（URL 预览暂不支持）".into());
                }
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
        Commands::CreateBatch { file, list, yes } => {
            if !*yes {
                return Err("批量创建须加 --yes".into());
            }
            create_batch(&cli, file.clone(), list.as_deref())
        }
        Commands::Show { task_id } => {
            let id = parse_uuid(task_id, "task_id")?;
            dispatch_action(&cli, WorkbenchAction::GetTask { task_id: id })
        }
        Commands::Update {
            task_id,
            title,
            notes,
            due_date,
            due_time,
            clear_due,
            priority,
            list,
            tags,
            yes,
        } => {
            if !*yes {
                return Err("更新任务须加 --yes".into());
            }
            let id = parse_uuid(task_id, "task_id")?;
            let list_id = list.as_deref().map(|s| resolve_list_ref(&cli, s)).transpose()?;
            dispatch_action(
                &cli,
                WorkbenchAction::UpdateTask {
                    input: ActionUpdateTaskInput {
                        id,
                        title: title.clone(),
                        notes: notes.clone(),
                        priority: priority
                            .as_deref()
                            .map(parse_priority)
                            .transpose()?,
                        list_id,
                        due_date: due_date.clone(),
                        due_time: due_time.clone(),
                        clear_due: *clear_due,
                        tag_names: (!tags.is_empty()).then(|| tags.clone()),
                    },
                    confirmed: true,
                },
            )
        }
        Commands::Reopen { task_id, yes } => {
            if !*yes {
                return Err("重开任务须加 --yes".into());
            }
            let id = parse_uuid(task_id, "task_id")?;
            dispatch_action(
                &cli,
                WorkbenchAction::ReopenTask {
                    task_id: id,
                    confirmed: true,
                },
            )
        }
        Commands::Archive { task_id, yes } => {
            if !*yes {
                return Err("归档任务须加 --yes".into());
            }
            let id = parse_uuid(task_id, "task_id")?;
            dispatch_action(
                &cli,
                WorkbenchAction::ArchiveTask {
                    task_id: id,
                    confirmed: true,
                },
            )
        }
        Commands::Delete {
            task_id,
            children,
            yes,
        } => {
            if !*yes {
                return Err("删除任务须加 --yes".into());
            }
            let id = parse_uuid(task_id, "task_id")?;
            dispatch_action(
                &cli,
                WorkbenchAction::DeleteTask {
                    task_id: id,
                    disposition: match children {
                        DeleteChildren::Cascade => TaskDeleteDisposition::Cascade,
                        DeleteChildren::Promote => TaskDeleteDisposition::Promote,
                    },
                    confirmed: true,
                },
            )
        }
        Commands::Postpone { task_id, days, yes } => {
            if !*yes {
                return Err("延后任务须加 --yes".into());
            }
            let id = parse_uuid(task_id, "task_id")?;
            dispatch_action(
                &cli,
                WorkbenchAction::PostponeTask {
                    task_id: id,
                    days: *days,
                    confirmed: true,
                },
            )
        }
        Commands::Defer {
            task_id,
            at,
            clear,
            yes,
        } => {
            if !*yes {
                return Err("设置延期须加 --yes".into());
            }
            if at.is_none() && !*clear {
                return Err("须指定 --at <日期> 或 --clear".into());
            }
            let id = parse_uuid(task_id, "task_id")?;
            dispatch_action(
                &cli,
                WorkbenchAction::SetTaskDefer {
                    task_id: id,
                    available_at: at.clone(),
                    confirmed: true,
                },
            )
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
        Commands::Query { target } => {
            let action = query_action(&cli, target)?;
            dispatch_action(&cli, action)
        }
        Commands::ExportCsv { out } => export_csv(&cli, out.clone()),
    }
}

fn query_action(cli: &Cli, target: &QueryTarget) -> Result<WorkbenchAction, String> {
    match target {
        QueryTarget::Today => Ok(WorkbenchAction::QueryToday),
        QueryTarget::Overdue { limit } => Ok(WorkbenchAction::QueryOverdue { limit: *limit }),
        QueryTarget::Inbox { limit } => Ok(WorkbenchAction::QueryInbox { limit: *limit }),
        QueryTarget::List { list_id, limit } => {
            let id = parse_uuid(list_id, "list_id")?;
            Ok(WorkbenchAction::QueryList {
                list_id: id,
                limit: *limit,
            })
        }
        QueryTarget::Tasks {
            status,
            list,
            search,
            priority,
            due_from,
            due_to,
            parent,
            limit,
            offset,
        } => {
            let list_id = list.as_deref().map(|s| resolve_list_ref(cli, s)).transpose()?;
            Ok(WorkbenchAction::QueryTasks {
                query: TaskQuery {
                    status: status.as_deref().map(parse_status).transpose()?,
                    list_id,
                    search: search.clone().filter(|s| !s.trim().is_empty()),
                    priority: priority.as_deref().map(parse_priority).transpose()?,
                    due_from: due_from.clone(),
                    due_to: due_to.clone(),
                    parent_id: parent.as_deref().map(|p| parse_uuid(p, "parent")).transpose()?,
                    limit: *limit,
                    offset: *offset,
                    ..Default::default()
                },
            })
        }
        QueryTarget::Lists => Ok(WorkbenchAction::QueryLists),
        QueryTarget::Memories { query, limit } => Ok(WorkbenchAction::SearchMemories {
            query: query.clone(),
            limit: *limit,
        }),
        QueryTarget::Snippets { query, limit } => Ok(WorkbenchAction::GetSnippets {
            query: query.clone(),
            limit: *limit,
        }),
    }
}

fn parse_uuid(value: &str, label: &str) -> Result<EntityId, String> {
    uuid::Uuid::parse_str(value).map_err(|_| format!("{label} 不是有效 UUID"))
}

fn parse_priority(value: &str) -> Result<TaskPriority, String> {
    match value {
        "none" => Ok(TaskPriority::None),
        "low" => Ok(TaskPriority::Low),
        "medium" => Ok(TaskPriority::Medium),
        "high" => Ok(TaskPriority::High),
        other => Err(format!("priority 须为 none|low|medium|high，得到 {other}")),
    }
}

fn parse_status(value: &str) -> Result<TaskStatus, String> {
    match value {
        "todo" => Ok(TaskStatus::Todo),
        "completed" => Ok(TaskStatus::Completed),
        "archived" => Ok(TaskStatus::Archived),
        other => Err(format!("status 须为 todo|completed|archived，得到 {other}")),
    }
}

/// Resolve a `--list` reference (uuid or exact list name) via QueryLists.
fn resolve_list_ref(cli: &Cli, spec: &str) -> Result<EntityId, String> {
    if let Ok(id) = uuid::Uuid::parse_str(spec) {
        return Ok(id);
    }
    if cli.dry_run {
        // 预览模式不唤醒应用；占位 id 不会落库，描述输出也不含清单 id。
        return Ok(EntityId::default());
    }
    let lists = fetch_lists(cli)?;
    let hits: Vec<&ActionListHit> = lists.iter().filter(|l| l.name == spec).collect();
    match hits.len() {
        1 => Ok(hits[0].id),
        0 => Err(format!(
            "清单「{spec}」不存在；可用清单：{}",
            lists
                .iter()
                .map(|l| l.name.as_str())
                .collect::<Vec<_>>()
                .join("、")
        )),
        _ => Err(format!(
            "清单名「{spec}」不唯一，请改用 uuid：{}",
            hits.iter()
                .map(|l| format!("{} = {}", l.name, l.id))
                .collect::<Vec<_>>()
                .join("，")
        )),
    }
}

fn fetch_lists(cli: &Cli) -> Result<Vec<ActionListHit>, String> {
    let outcome = dispatch_for_outcome(cli, WorkbenchAction::QueryLists)?;
    match outcome {
        ActionOutcome::ListsQueried { lists } => Ok(lists),
        ActionOutcome::Rejected { reason } => Err(format!("动作被拒绝：{reason}")),
        other => Err(format!("意外的返回结果：{other:?}")),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchTaskSpec {
    title: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    due_time: Option<String>,
    /// none | low | medium | high
    #[serde(default)]
    priority: Option<String>,
    /// 清单名称或 uuid（优先于全局 --list）
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    parent: Option<EntityId>,
}

const BATCH_MAX_ITEMS: usize = 100;

enum BatchItemResult {
    Created { task_id: EntityId, title: String },
    Failed { title: String, error: String },
}

fn create_batch(
    cli: &Cli,
    file: Option<PathBuf>,
    global_list: Option<&str>,
) -> Result<(), String> {
    let raw = match file {
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|e| format!("读取 {}: {e}", path.display()))?,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("读取 stdin: {e}"))?;
            buf
        }
    };
    let mut specs: Vec<Result<BatchTaskSpec, String>> = Vec::new();
    for (lineno, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        specs.push(
            serde_json::from_str::<BatchTaskSpec>(line)
                .map_err(|e| format!("第 {} 行 JSON 无效: {e}", lineno + 1)),
        );
    }
    if specs.is_empty() {
        return Err("没有可导入的任务（输入为空）".into());
    }
    if specs.len() > BATCH_MAX_ITEMS {
        return Err(format!(
            "一次最多导入 {BATCH_MAX_ITEMS} 条，得到 {} 条",
            specs.len()
        ));
    }
    let global_list_id = global_list.map(|s| resolve_list_ref(cli, s)).transpose()?;

    let mut results: Vec<BatchItemResult> = Vec::with_capacity(specs.len());
    for (index, entry) in specs.into_iter().enumerate() {
        let spec = match entry {
            Ok(spec) => spec,
            Err(err) => {
                if !cli.json {
                    eprintln!("✗ #{index} 失败：{err}");
                }
                results.push(BatchItemResult::Failed {
                    title: String::new(),
                    error: err,
                });
                continue;
            }
        };
        let item = (|| -> Result<EntityId, String> {
            let priority = spec.priority.as_deref().map(parse_priority).transpose()?;
            let list_id = match &spec.list {
                Some(name) => Some(resolve_list_ref(cli, name)?),
                None => global_list_id,
            };
            let action = WorkbenchAction::CreateTask {
                input: CreateTaskInput {
                    title: spec.title.clone(),
                    notes: spec.notes.clone(),
                    priority,
                    list_id,
                    due_date: spec.due_date.clone(),
                    due_time: spec.due_time.clone(),
                    tag_names: spec.tags.clone(),
                    parent_id: spec.parent,
                },
                confirmed: true,
            };
            match dispatch_for_outcome(cli, action)? {
                ActionOutcome::TaskCreated { task } => Ok(task.id),
                ActionOutcome::Rejected { reason } => Err(format!("被拒绝：{reason}")),
                other => Err(format!("意外的返回结果：{other:?}")),
            }
        })();
        match item {
            Ok(id) => {
                if !cli.json {
                    println!("✓ #{index} 已创建 {id}");
                }
                results.push(BatchItemResult::Created {
                    task_id: id,
                    title: spec.title.clone(),
                });
            }
            Err(err) => {
                if !cli.json {
                    eprintln!("✗ #{index}「{}」失败：{err}", spec.title);
                }
                results.push(BatchItemResult::Failed {
                    title: spec.title.clone(),
                    error: err,
                });
            }
        }
    }
    let ok_count = results
        .iter()
        .filter(|r| matches!(r, BatchItemResult::Created { .. }))
        .count();
    if cli.json {
        let items: Vec<serde_json::Value> = results
            .iter()
            .enumerate()
            .map(|(index, r)| match r {
                BatchItemResult::Created { task_id, title } => serde_json::json!({
                    "index": index, "ok": true, "taskId": task_id.to_string(), "title": title,
                }),
                BatchItemResult::Failed { title, error } => serde_json::json!({
                    "index": index, "ok": false, "title": title, "error": error,
                }),
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total": results.len(),
                "succeeded": ok_count,
                "failed": results.len() - ok_count,
                "items": items,
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "批量创建完成：{}/{} 成功",
            ok_count,
            results.len()
        );
    }
    if ok_count < results.len() {
        return Err(format!("{} 条导入失败", results.len() - ok_count));
    }
    Ok(())
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
                parent_id: None,
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
    let outcome = dispatch_for_outcome(cli, action)?;
    print_outcome(cli, outcome)
}

/// Forward the action to the running app and wait for its outcome. The CLI
/// never opens SQLite itself; the app (via single-instance forwarding or its
/// own startup argv handling) executes the action and writes the response.
fn dispatch_for_outcome(cli: &Cli, action: WorkbenchAction) -> Result<ActionOutcome, String> {
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
            Ok(outcome)
        }
        None => Err(format!(
            "等待应用响应超时（{}）",
            response_path.display()
        )),
    }
}

fn export_csv(cli: &Cli, out: Option<PathBuf>) -> Result<(), String> {
    if cli.dry_run {
        return print_outcome(cli, dry_run_outcome(&WorkbenchAction::ExportTasksCsv));
    }
    let outcome = dispatch_for_outcome(cli, WorkbenchAction::ExportTasksCsv)?;
    let csv = match outcome {
        ActionOutcome::TasksCsvExported { csv } => csv,
        ActionOutcome::Rejected { reason } => return Err(format!("动作被拒绝：{reason}")),
        other => return Err(format!("意外的返回结果：{other:?}")),
    };
    if csv.trim().is_empty() {
        return Err("导出结果为空".into());
    }
    let out = out.unwrap_or_else(|| {
        PathBuf::from(format!("trove-tasks-{}.csv", chrono::Local::now().format("%Y-%m-%d")))
    });
    std::fs::write(&out, &csv).map_err(|e| format!("写入 {}: {e}", out.display()))?;
    let rows = csv.lines().count().saturating_sub(1);
    if cli.json {
        println!(
            "{}",
            serde_json::json!({ "ok": true, "path": out.display().to_string(), "rows": rows, "bytes": csv.len() })
        );
    } else {
        println!("已导出 {rows} 条任务到 {}（{} 字节）", out.display(), csv.len());
    }
    Ok(())
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
        ActionOutcome::TodayQueried { data } => format!(
            "今日 {}：逾期 {} · 到期 {} · 重点 {} · 提醒 {}",
            data.today,
            data.overdue.len(),
            data.due_today.len(),
            data.focus.len(),
            data.reminders_today.len()
        ),
        ActionOutcome::TasksQueried { items, total } => {
            format!("任务 {}/{} 条", items.len(), total)
        }
        ActionOutcome::MemoriesQueried { items, total } => {
            format!("记忆 {}/{} 条", items.len(), total)
        }
        ActionOutcome::SnippetsQueried { items, total } => {
            format!("片段 {}/{} 条", items.len(), total)
        }
        ActionOutcome::TasksCsvExported { csv } => {
            format!("任务 CSV {} 字节", csv.len())
        }
        ActionOutcome::TaskFetched { task } => format!(
            "任务 {}：{}（{:?}，{:?}，清单 {}）",
            task.id, task.title, task.status, task.priority, task.list_name
        ),
        ActionOutcome::TaskUpdated { task } => format!("已更新任务 {}", task.id),
        ActionOutcome::TaskReopened { task } => format!("已重开任务 {}", task.id),
        ActionOutcome::TaskArchived { task } => format!("已归档任务 {}", task.id),
        ActionOutcome::TaskDeleted { task_id } => format!("已删除任务 {task_id}"),
        ActionOutcome::TaskPostponed { task } => format!(
            "已延后任务 {}（新截止 {}）",
            task.id,
            task.due_date.as_deref().unwrap_or("无")
        ),
        ActionOutcome::TaskDeferred { task } => match task.available_at.as_deref() {
            Some(date) => format!("已设置延期 {}（{date} 起显示）", task.id),
            None => format!("已清除任务 {} 的延期显示", task.id),
        },
        ActionOutcome::ListsQueried { lists } => {
            let parts: Vec<String> = lists
                .iter()
                .map(|l| format!("{}（{}）{} 待办", l.name, l.id, l.open_count))
                .collect();
            format!("清单 {} 个：{}", lists.len(), parts.join("、"))
        }
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
