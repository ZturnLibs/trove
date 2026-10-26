-- 任务看板阶段列：看板「进行中」列的可空标记。
--
-- 唯一合法非空值 'doing'（应用层枚举校验，不加 CHECK，与既有迁移风格一致）；
-- NULL = 待办列。不变式：board_stage 只在 status='todo' 时有意义——
-- complete_task / archive_task（含级联与跳过实例）把任务移出 todo 时一并置空。
-- 纯增量列：revert 代码后旧库可继续打开；旧数据全 NULL，行为不变。
-- 不加索引：看板查询走既有 status 条件后由客户端按 stage 分流（个人量级）。

ALTER TABLE tasks ADD COLUMN board_stage TEXT;
