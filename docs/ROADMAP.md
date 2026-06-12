# CCometixLine Roadmap

基于 2026-06-12 的生态调研（ccstatusline ~11K stars vs CCometixLine ~2.3K stars），
按"先补正确性、再追平体验、最后做差异化"排序。

## 已完成（render/columns-lines 分支）

- [x] 相位化渲染管线（filter → composition → separator → width → join）
- [x] COLUMNS 宽度感知截断（Claude Code ≥ v2.1.153 注入 COLUMNS/LINES）
- [x] CJK/宽字符正确计宽（unicode-width）
- [x] 省略号截断：单段超宽就地截断；末段放不下时若剩余 ≥4 列则截断回填
- [x] 预留宽度 `width.reserve`（默认 40，给 auto-compact 提示留空间，对标
      ccstatusline 的 full-minus-40）
- [x] `CCLINE_WIDTH` 环境变量：精确宽度兜底（嵌套 PTY / IDE 包装进程，
      对标 CCSTATUSLINE_WIDTH）
- [x] 多行输出 `width.max_lines`（>1 时按段折行而非截断，受 LINES 封顶）
- [x] 段截断优先级：`[[segments]] options.priority = N`（高优先级存活更久，
      默认从末尾丢弃）
- [x] Subagent statusline：`ccline --subagent` 渲染 `subagentStatusLine`
      每行（状态图标着色 + 名称/描述 + 右对齐 token 数，按 `columns` 截断；
      Rust 生态首个支持）

## P1 — 体验追平（ccstatusline 已有）✅ 全部完成（render/p1-experience 分支）

- [x] 宽度模式切换 `width.mode = "full" | "reserve" | "adaptive"`（默认
      reserve；adaptive 在 context 用量低于 `width.adaptive_threshold`
      （默认 60%）时用满 COLUMNS，逼近 auto-compact 后回到 reserve，
      对标 ccstatusline 的 full / full-minus-40 / until-compact）
- [x] Flex separator：`[[segments]] id = "flex"` 弹性空隙段，把其后的段
      推到行尾；有宽度预算时按剩余列均分（多个 flex 平摊），无预算退化为
      单空格；flex 两侧不渲染普通分隔符，截断时永不优先牺牲 flex
- [x] Custom widget：`[[segments]] id = "custom"`，`options.text` 静态文本
      或 `options.command` shell 命令首行输出（`timeout` 默认 1s 轮询杀死，
      `cache_duration` 默认 0 不缓存）；允许多个 custom 段各带各的 options

## P2 — 差异化（社区高频诉求 + Rust 端空白）

1. ~~Token 速率 widget~~ ✅ 已实现（`token_rate` 段：滑动窗口输出 tok/s，
   `options.window_seconds` 可配，默认 60s；从 transcript 时间戳无状态计算）。
2. ~~周度 Sonnet/Opus 用量拆分~~ ✅ 已实现（`weekly_usage` 段：OAuth 登录走
   官方 usage 接口显示 `W 42% · O 13%`，未登录回退本地 transcript 七天聚合
   显示 `S 1.2M · O 300k`，600s 缓存）。
3. ~~Subagent statusline~~ ✅ 已实现（`ccline --subagent`）。

## P3 — 工程质量

- [x] OSC 转义序列支持（visible_width / truncate_visible 识别
      `ESC ] … BEL` 与 `ESC ] … ESC \`，OSC 8 超链接计零宽、截断时透传）
- [x] 主输出路径的快照测试（tests/render_snapshots.rs：9 个内置主题 ×
      4 档宽度的 golden 文件，`UPDATE_SNAPSHOTS=1` 刷新）
- [x] TUI 配置器暴露 width.mode / reserve / max_lines / priority
      （`O` 打开 Width Settings 弹窗：↑↓ 选字段、←→ 调整、Enter 应用；
      主面板 `+`/`-` 调整选中段的 `options.priority`，归零时移除该键；
      custom 段等任意 options 的通用编辑器另行排期）
