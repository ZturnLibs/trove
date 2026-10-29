//! e2e 冒烟用例（10 条）。
//!
//! 运行：`E2E=1 cargo test --test e2e -- --ignored`（需 `tauri-driver` 在 PATH、
//! 先 `cargo build`）。macOS 无 WKWebView WebDriver 实现，用例自动跳过；
//! 真实验证走 CI 的 `e2e` job（ubuntu）。详见 `helper/mod.rs` 头注释。
//!
//! 用例与 PRD 对应：`../../.trellis/tasks/10-27-e2e-smoke/prd.md`（R2 用例集）。

mod helper;

use std::time::{Duration, Instant};

use helper::Harness;

const ENTER: &str = "\u{E007}";

const NAV_LINKS: [&str; 6] = ["今日", "收件箱", "任务", "看板", "记忆", "剪切板"];

/// 收件箱空态「新建任务」→ 详情面板标题输入 → 输入标题提交。
/// 新库（tempdir）收件箱必为空，入口确定存在。
/// 注意：点击「新建任务」即以默认标题「新任务」建卡并开详情面板，但焦点
/// 不保证移入输入框（CI 实证 DOM 停在 value="新任务"）——必须显式点进
/// 标题输入框再键入。
async fn create_task(
    ctx: &helper::Ctx,
    title: &str,
) -> Result<(), String> {
    ctx.click(&ctx.nav_link("收件箱"), "收件箱导航").await?;
    ctx.click(&ctx.button_with_text("新建任务"), "新建任务").await?;
    ctx.click(&ctx.title_input(), "标题输入框").await?;
    let active = ctx
        .client
        .active_element()
        .await
        .map_err(|e| format!("定位标题输入焦点失败: {e}"))?;
    // 全选默认标题「新任务」，键入即替换（点击后 caret 在尾部，直接键入会追加）。
    active
        .send_keys("\u{E009}a") // Ctrl+A
        .await
        .map_err(|e| format!("全选失败: {e}"))?;
    active
        .send_keys(title)
        .await
        .map_err(|e| format!("输入标题失败: {e}"))?;
    // 详情面板标题输入是 onBlur={save} 提交（无 Enter 处理）。实测 ENTER+TAB
    // 连发会与末字键入竞态（丢尾字）且键语义 blur 不可靠：静置后用 JS 直接
    // blur 触发 save。
    tokio::time::sleep(Duration::from_millis(250)).await;
    ctx.blur_active().await?;
    ctx.wait_for(&ctx.task_row(title), "新建的任务").await?;
    Ok(())
}

/// 用例 #1：启动 → 侧栏六入口渲染。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case01_launch_renders_sidebar() {
    let Some(h) = Harness::acquire("01-launch").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        for label in NAV_LINKS {
            ctx.find(&ctx.nav_link(label), label).await?;
        }
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #2：收件箱新建任务 → 列表命中。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case02_create_task_via_inbox() {
    let Some(h) = Harness::acquire("02-create").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        create_task(ctx, "e2e 建任务冒烟").await
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #3：完成任务 → 撤销 toast → 回滚为待办。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case03_complete_then_undo() {
    let Some(h) = Harness::acquire("03-complete-undo").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        create_task(ctx, "e2e 完成与撤销").await?;
        // 完成：走详情面板「完成」按钮——只有该路径 push recent-action
        // （行内按钮静默完成，撤销 toast 无从弹出）。inbox 查询过滤
        // status=todo，完成后任务离开视图，以「行消失」为完成断言。
        ctx.click(&ctx.button_with_text("完成"), "详情面板完成按钮")
            .await?;
        ctx.wait_gone(&ctx.task_row("e2e 完成与撤销"), "完成的任务离开收件箱")
            .await?;
        // 撤销：recent-actions toast（5s 自动消失）；任务回归收件箱。
        ctx.click(&ctx.button_with_text("撤销"), "撤销按钮").await?;
        ctx.wait_for(&ctx.task_row("e2e 完成与撤销"), "撤销后任务回归收件箱")
            .await?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #4：双击标题行内改名 → 重开保持。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case04_double_click_rename() {
    let Some(mut h) = Harness::acquire("04-rename").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        create_task(ctx, "e2e 改名前").await?;
        // 双击行内改名。编辑框节点毫秒级失效，跨命令句柄必脱钩——先预埋页内
        // 驻留监听（30ms 粒度，编辑框一出现立即置值+input+Enter 原子提交），
        // 再做真实指针双击，最后轮读监听结果。
        helper::Ctx::arm_editor_submit(&ctx.client, "e2e 改名后", 8000).await?;
        let mut last = String::new();
        let mut opened = false;
        for _ in 0..4 {
            match ctx
                .double_click_retry(&ctx.task_title("e2e 改名前"), "标题展示元素")
                .await
            {
                Ok(()) => {
                    opened = true;
                    break;
                }
                Err(e) => last = e,
            }
        }
        if !opened {
            return Err(format!("双击改名失败（重试耗尽）: {last}"));
        }
        let mut submitted: Result<Option<serde_json::Value>, String> = Ok(None);
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            submitted = helper::Ctx::editor_submit_status(&ctx.client).await;
            match submitted.as_ref().map(|s| s.as_ref()) {
                Ok(Some(_)) => break,
                _ if Instant::now() > deadline => break,
                _ => {}
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let status = submitted.map_err(|e| format!("读状态失败: {e}"))?;
        let is_true = status.as_ref().and_then(|v| v.as_bool()) == Some(true);
        if !is_true {
            let display = status
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into());
            return Err(format!("双击改名失败: 页内提交状态 {display}；last={last}"));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        ctx.wait_for(&ctx.task_row("e2e 改名后"), "改名持久化的任务")
            .await
            .map(|_| ())?;
        Ok(())
    });
    let r = result.await;
    if r.is_ok() {
        // 重开保持：**同一数据目录**二次连接。必须复用 h（reconnect 同 driver +
        // tempdir）——重新 acquire 会开全新 tempdir（空库），持久化断言永假
        //（run 40749589555 的 tasks-on-failure.csv 实证 reopen 库为空）。
        if let Err(e) = h.reconnect().await {
            h.finish(Err(format!("重连失败: {e}"))).await;
            return;
        }
        let reopen = h
            .run(|ctx| async {
                ctx.wait_main_ready().await?;
                // 冷启后收件箱列表异步加载，用带重试的 wait_for。
                ctx.wait_for(&ctx.task_row("e2e 改名后"), "改名持久化的任务")
                    .await
                    .map(|_| ());
                Ok(())
            })
            .await;
        h.finish(reopen).await;
    } else {
        h.finish(r).await;
    }
}

/// 用例 #5：看板跨列拖拽（待办 → 进行中）。
/// 真实指针序列（W3C Actions）驱动 dnd-kit PointerSensor——JS 合成事件
/// 不激活传感器（归档结论）；细节见 .trellis/spec/tauri/e2e-webdriver.md。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case05_board_cross_column_drag() {
    let Some(h) = Harness::acquire("05-drag").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        ctx.click(&ctx.nav_link("看板"), "看板导航").await?;
        // 看板列尾新建两张卡（待办列）。
        for title in ["e2e 跨列垫片", "e2e 跨列"] {
            ctx.click(&ctx.board_quick_add("待办"), "待办列快速新建").await?;
            let composer = ctx
                .wait_for(&ctx.board_composer("待办"), "待办列新建输入框")
                .await?;
            composer
                .send_keys(title)
                .await
                .map_err(|e| format!("输入卡标题失败: {e}"))?;
            composer
                .send_keys(ENTER)
                .await
                .map_err(|e| format!("回车建卡失败: {e}"))?;
            ctx.wait_for(&ctx.board_card("待办", title), "待办列新卡").await?;
        }
        // 拖「e2e 跨列」卡到「进行中」列：卡片离开待办、出现在进行中。
        let mut last = String::new();
        let mut moved: Result<(), String> = Err("未尝试".into());
        for _ in 0..3 {
            let src = ctx
                .wait_for(&ctx.board_card("待办", "e2e 跨列"), "待办列源卡")
                .await?;
            let dst = ctx
                .wait_for(&ctx.board_column("进行中"), "进行中列")
                .await?;
            if let Err(e) = ctx.native_drag(&src, &dst).await {
                last = e;
                continue;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
            moved = ctx
                .wait_for(&ctx.board_card("进行中", "e2e 跨列"), "进行中列的卡")
                .await
                .map(|_| ());
            if moved.is_ok() {
                break;
            }
            last = moved.as_ref().unwrap_err().clone();
        }
        moved.map_err(|e| format!("跨列拖拽失败: {e}；last={last}"))?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #6：看板同列排序（拖「甲」到「乙」上，顺序互换）。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case06_board_same_column_reorder() {
    let Some(h) = Harness::acquire("06-reorder").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        ctx.click(&ctx.nav_link("看板"), "看板导航").await?;
        for title in ["e2e 排序甲", "e2e 排序乙"] {
            ctx.click(&ctx.board_quick_add("待办"), "待办列快速新建").await?;
            let composer = ctx
                .wait_for(&ctx.board_composer("待办"), "待办列新建输入框")
                .await?;
            composer
                .send_keys(title)
                .await
                .map_err(|e| format!("输入卡标题失败: {e}"))?;
            composer
                .send_keys(ENTER)
                .await
                .map_err(|e| format!("回车建卡失败: {e}"))?;
            ctx.wait_for(&ctx.board_card("待办", title), "待办列新卡").await?;
        }
        // 列内当前顺序（页内 JS 读取，规避 XPath 位置断言的文本污染问题）。
        let order_script = r#"
            const sec = Array.from(document.querySelectorAll("section")).find(
                (s) => {
                    const h = s.querySelector("h2");
                    return h && h.textContent.trim() === "待办";
                });
            if (!sec) return null;
            return Array.from(
                sec.querySelectorAll("div[role='button']"))
                .map((el) => el.textContent.trim());
        "#;
        let order_of = |items: serde_json::Value| -> Vec<String> {
            items
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default()
        };
        let before = ctx
            .client
            .execute(order_script, vec![])
            .await
            .map(order_of)
            .map_err(|e| format!("读取列内顺序失败: {e}"))?;
        // 拖「甲」到「乙」卡上 → 乙应在甲前。
        let mut last = String::new();
        let mut reordered: Result<(), String> = Err("未尝试".into());
        for _ in 0..3 {
            let src = ctx
                .wait_for(&ctx.board_card("待办", "e2e 排序甲"), "源卡甲")
                .await?;
            let dst = ctx
                .wait_for(&ctx.board_card("待办", "e2e 排序乙"), "目标卡乙")
                .await?;
            if let Err(e) = ctx.native_drag(&src, &dst).await {
                last = e;
                continue;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
            let after = ctx
                .client
                .execute(order_script, vec![])
                .await
                .map(order_of)
                .map_err(|e| format!("读取列内顺序失败: {e}"));
            match after {
                Ok(order) => {
                    let i_jia = order
                        .iter()
                        .position(|t| t.contains("e2e 排序甲"));
                    let i_yi = order.iter().position(|t| t.contains("e2e 排序乙"));
                    if let (Some(a), Some(b)) = (i_jia, i_yi) {
                        if b < a {
                            reordered = Ok(());
                            break;
                        }
                    }
                    last = format!("顺序未变：{order:?}");
                }
                Err(e) => last = e,
            }
        }
        reordered.map_err(|e| format!("列内排序失败: {e}；last={last}"))?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #7：侧栏新建分组 → 树上出现。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case07_create_group() {
    let Some(h) = Harness::acquire("07-group").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        // 菜单受「window click 即关闭」约束（TaskSidebarTree useEffect），
        // WebDriver 点击残留事件会让菜单瞬闭（CI 5 轮瞬时抓取全空实证）。
        // 改走 contextmenu 分支打开同一菜单：无后续 click，菜单稳定存活。
        let entry = ctx
            .wait_for(&ctx.button_aria("新建分组或清单"), "新建分组或清单入口")
            .await?;
        ctx.context_click(&entry, "新建分组或清单入口").await?;
        ctx.click(&ctx.button_with_text("新建分组"), "新建分组菜单项")
            .await?;
        // 行内命名输入（占位符区分分组/清单）。
        ctx.click(
            "//input[contains(@placeholder,'分组名称')]",
            "分组命名输入框",
        )
        .await?;
        let active = ctx
            .client
            .active_element()
            .await
            .map_err(|e| format!("定位分组输入焦点失败: {e}"))?;
        active
            .send_keys("e2e 分组")
            .await
            .map_err(|e| format!("输入分组名失败: {e}"))?;
        active
            .send_keys(ENTER)
            .await
            .map_err(|e| format!("回车提交分组失败: {e}"))?;
        ctx.wait_for(&ctx.sidebar_node("e2e 分组"), "侧栏分组节点").await?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #8：任务页内搜索命中。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case08_tasks_page_search() {
    let Some(h) = Harness::acquire("08-search").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        create_task(ctx, "e2e 搜索目标青鸟").await?;
        ctx.click(&ctx.nav_link("任务"), "任务导航").await?;
        let search = ctx
            .wait_for(
                "//input[contains(@placeholder,'搜索任务')]",
                "任务页搜索框",
            )
            .await?;
        search
            .send_keys("青鸟")
            .await
            .map_err(|e| format!("输入搜索词失败: {e}"))?;
        ctx.wait_for(&ctx.task_row("e2e 搜索目标青鸟"), "搜索命中的任务")
            .await?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #9：URL Scheme 直达（trove://today）。
/// Linux dev 二进制即可测：URL 经 single-instance 插件 argv 转发到主实例
/// （lib.rs `handle_trove_url` → Navigate /today → `main://navigate`），
/// 不依赖打包注册（xdg mimeapps 仅影响系统级 xdg-open 入口）。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case09_url_scheme_today() {
    let Some(h) = Harness::acquire("09-deeplink").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        // 先离开今日：激活态切到收件箱。
        ctx.click(&ctx.nav_link("收件箱"), "收件箱导航").await?;
        ctx.wait_for(&ctx.active_nav("收件箱"), "收件箱激活").await?;
        // 二实例 argv 携带 trove://today → 主实例导航回今日。
        ctx.spawn_url_instance("trove://today")?;
        ctx.wait_for(&ctx.active_nav("今日"), "URL 直达后今日重新激活")
            .await?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #10：CLI 导出任务 CSV（经运行中的应用转发，文件非空）。
#[tokio::test]
#[ignore = "E2E=1 + tauri-driver（仅 Linux/Windows；macOS 自动跳过）"]
async fn case10_cli_export_csv() {
    let Some(h) = Harness::acquire("10-export").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.dismiss_onboarding().await?;
        create_task(ctx, "e2e 导出样本").await?;
        let out = std::env::temp_dir().join(format!("trove-e2e-export-{}.csv", std::process::id()));
        let output = ctx.run_cli(&["export-csv", "--out", out.to_string_lossy().as_ref()])?;
        ctx.step(&format!("CLI 输出：{}", output.trim()));
        let meta = std::fs::metadata(&out)
            .map_err(|e| format!("导出文件不存在（{}）：{e}", out.display()))?;
        if meta.len() == 0 {
            return Err("导出 CSV 为空".into());
        }
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}
