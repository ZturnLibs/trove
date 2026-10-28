//! e2e 冒烟用例（10 条）。
//!
//! 运行：`E2E=1 cargo test --test e2e -- --ignored`（需 `tauri-driver` 在 PATH、
//! 先 `cargo build`）。macOS 无 WKWebView WebDriver 实现，用例自动跳过；
//! 真实验证走 CI 的 `e2e` job（ubuntu）。详见 `helper/mod.rs` 头注释。
//!
//! 用例与 PRD 对应：`../../.trellis/tasks/10-27-e2e-smoke/prd.md`（R2 用例集）。

mod helper;

use std::time::Duration;

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
        // 双击标题展示元素（div.truncate 的 onDoubleClick → startEdit）；
        // 行内输入是受控 Input（commit 读 React state），必须真实键入让
        // onChange 更新 draft，再 Enter 提交。
        // 注意效果定位：进入编辑态后行的 text content 不再含标题（标题 div
        // 被替换成 input），不能用 contains(.,title) 定位编辑框——改用其
        // 类特征（h-7 text-[13px]）。
        // 编辑框节点在 WebKitGTK 上毫秒级被替换（跨命令元素句柄必脱钩）——
        // 双击进编辑态后，用单条页内 JS 原子完成「置值 + input 事件 + Enter」。
        // 受控输入必须走原型 value setter + input 事件，React 才接受变更。
        const SET_AND_SUBMIT: &str = r#"
            const input = document.querySelector("div[role='button'] input.h-7");
            if (!input) throw new Error("no editing input");
            const setter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, "value").set;
            setter.call(input, arguments[0]);
            input.dispatchEvent(new Event("input", { bubbles: true }));
            input.dispatchEvent(new KeyboardEvent("keydown", {
                key: "Enter", bubbles: true, cancelable: true }));
        "#;
        let mut last = String::new();
        let mut renamed: Result<(), String> = Err("未尝试".into());
        for _ in 0..3 {
            match ctx
                .double_click_until(
                    &ctx.task_title("e2e 改名前"),
                    "//div[@role='button']//input[contains(@class,'text-[13px]')]",
                    "双击改名",
                )
                .await
            {
                Ok(_) => {}
                Err(e) => return Err(format!("双击改名失败: {e}；last={last}")),
            }
            if let Err(e) = ctx
                .client
                .execute(SET_AND_SUBMIT, vec![serde_json::json!("e2e 改名后")])
                .await
            {
                last = format!("置值提交失败: {e}");
                continue;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
            renamed = ctx
                .wait_for(&ctx.task_row("e2e 改名后"), "改名持久化的任务")
                .await
                .map(|_| ());
            if renamed.is_ok() {
                break;
            }
            last = renamed.as_ref().unwrap_err().clone();
        }
        renamed.map_err(|e| format!("双击改名失败: {e}；last={last}"))?;
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
