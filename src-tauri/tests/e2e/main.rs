//! e2e 冒烟用例（10 条）。
//!
//! 运行：`E2E=1 cargo test --test e2e -- --ignored`（需 `tauri-driver` 在 PATH、
//! 先 `cargo build`）。macOS 无 WKWebView WebDriver 实现，用例自动跳过；
//! 真实验证走 CI 的 `e2e` job（ubuntu）。详见 `helper/mod.rs` 头注释。
//!
//! 用例与 PRD 对应：`../../.trellis/tasks/10-27-e2e-smoke/prd.md`（R2 用例集）。

mod helper;

use helper::Harness;

const ENTER: &str = "\u{E007}";

const NAV_LINKS: [&str; 6] = ["今日", "收件箱", "任务", "看板", "记忆", "剪切板"];

/// 收件箱空态「新建任务」→ 详情面板标题输入（autofocus）→ 输入标题提交。
/// 新库（tempdir）收件箱必为空，入口确定存在。
async fn create_task(
    ctx: &helper::Ctx,
    title: &str,
) -> Result<(), String> {
    ctx.click(&ctx.nav_link("收件箱"), "收件箱导航").await?;
    ctx.click(&ctx.button_with_text("新建任务"), "新建任务").await?;
    let active = ctx
        .client
        .active_element()
        .await
        .map_err(|e| format!("定位标题输入焦点失败: {e}"))?;
    active
        .send_keys(title)
        .await
        .map_err(|e| format!("输入标题失败: {e}"))?;
    active
        .send_keys(ENTER)
        .await
        .map_err(|e| format!("回车提交失败: {e}"))?;
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
        // 完成：行内「完成」按钮（TaskRow aria-label）。
        ctx.click(&ctx.row_button("e2e 完成与撤销", "完成"), "完成按钮")
            .await?;
        // 完成后行内按钮翻转为「取消完成」。
        ctx.wait_for(
            &ctx.row_button("e2e 完成与撤销", "取消完成"),
            "取消完成按钮",
        )
        .await?;
        // 撤销：recent-actions toast。
        ctx.click(&ctx.button_with_text("撤销"), "撤销按钮").await?;
        ctx.wait_for(&ctx.row_button("e2e 完成与撤销", "完成"), "恢复的完成按钮")
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
        // 双击标题（JS 合成 dblclick，React onDoubleClick 可触发）。
        let row = ctx.find(&ctx.task_row("e2e 改名前"), "任务行").await?;
        ctx.double_click(&row).await?;
        // 行内输入框出现（TaskRow/BoardCard 同模式：非受控 defaultValue）。
        let input = ctx
            .wait_for(
                "//div[@role='button'][contains(normalize-space(.),'e2e 改名前')]//input",
                "改名输入框",
            )
            .await?;
        // 非受控输入：直接置 value 后派发 Enter keydown，React onKeyDown 读取 currentTarget.value。
        ctx.client
            .execute(
                "arguments[0].value = arguments[1]; \
                 arguments[0].dispatchEvent(new KeyboardEvent('keydown', {key: 'Enter', bubbles: true}));",
                vec![
                    serde_json::to_value(input).map_err(|e| e.to_string())?,
                    serde_json::json!("e2e 改名后"),
                ],
            )
            .await
            .map_err(|e| format!("提交改名失败: {e}"))?;
        ctx.wait_for(&ctx.task_row("e2e 改名后"), "改名持久化的任务").await?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
    // 重开保持：同数据目录第二连。
    let Some(mut h) = Harness::acquire("04-rename-reopen").await.expect("acquire") else {
        return;
    };
    let result = h.run(|ctx| async {
        ctx.wait_main_ready().await?;
        ctx.find(&ctx.task_row("e2e 改名后"), "改名持久化的任务").await?;
        Ok(())
    });
    let r = result.await;
    h.finish(r).await;
}

/// 用例 #5：看板跨列拖拽。dnd-kit PointerSensor 不响应 WebDriver 合成拖拽（已实测）。
#[tokio::test]
#[ignore = "PointerSensor 不响应合成拖拽；待 WebDriver 原生 action 序列验证后启用"]
async fn case05_board_cross_column_drag() {
    println!("[e2e:05-drag] skip：见 ignore 原因");
}

/// 用例 #6：看板同清单列内排序。同 #5。
#[tokio::test]
#[ignore = "PointerSensor 不响应合成拖拽；待 WebDriver 原生 action 序列验证后启用"]
async fn case06_board_same_column_reorder() {
    println!("[e2e:06-reorder] skip：见 ignore 原因");
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
        ctx.click(
            &ctx.button_aria("新建分组或清单"),
            "新建分组或清单入口",
        )
        .await?;
        // 弹层里的「新建分组」项（MainShell 右键/入口菜单）。
        ctx.click(&ctx.button_with_text("新建分组"), "新建分组菜单项")
            .await?;
        // 行内命名输入：输入分组名后回车。
        let input = ctx.wait_for("//input", "分组命名输入框").await?;
        input
            .send_keys("e2e 分组")
            .await
            .map_err(|e| format!("输入分组名失败: {e}"))?;
        input
            .send_keys(ENTER)
            .await
            .map_err(|e| format!("回车提交分组失败: {e}"))?;
        ctx.wait_for(&ctx.nav_link("e2e 分组"), "侧栏分组节点").await?;
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

/// 用例 #9：URL Scheme 直达。dev 二进制未注册 scheme，装注册流程后启用。
#[tokio::test]
#[ignore = "deep-link 需应用注册 scheme；dev 构建不含注册，待打包流程后启用"]
async fn case09_url_scheme_today() {
    println!("[e2e:09-deeplink] skip：见 ignore 原因");
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
