#!/usr/bin/env python3
"""Build the advisor briefing DOCX from structured content (python-docx)."""
from docx import Document
from docx.shared import Pt, RGBColor
from docx.oxml.ns import qn

OUT = "docs/paper/导师汇报-Decide-Act研究数据与内容.docx"

doc = Document()

# ---- CJK font helper ----
def set_cjk(run, name="Songti SC"):
    run.font.name = name
    r = run._element
    rPr = r.get_or_add_rPr()
    rFonts = rPr.find(qn("w:rFonts"))
    if rFonts is None:
        rFonts = rPr.makeelement(qn("w:rFonts"), {})
        rPr.append(rFonts)
    rFonts.set(qn("w:eastAsia"), name)

def para(text, bold=False, size=10.5, color=None, style=None):
    p = doc.add_paragraph(style=style)
    run = p.add_run(text)
    run.bold = bold
    run.font.size = Pt(size)
    if color:
        run.font.color.rgb = RGBColor(*color)
    set_cjk(run)
    return p

def heading(text, level):
    h = doc.add_heading(text, level=level)
    for run in h.runs:
        set_cjk(run, "Heiti SC")
    return h

def table(rows, widths=None):
    t = doc.add_table(rows=len(rows), cols=len(rows[0]))
    t.style = "Table Grid"
    for i, row in enumerate(rows):
        for j, cell in enumerate(row):
            c = t.cell(i, j)
            c.text = ""
            run = c.paragraphs[0].add_run(str(cell))
            run.font.size = Pt(9.5)
            run.bold = (i == 0)
            set_cjk(run)
    return t

# ================= Title block =================
title = doc.add_heading("研究汇报：Decide-Act —— 可审计的 Agent 运行时与「假成功」基准", level=0)
for run in title.runs:
    set_cjk(run, "Heiti SC")
para("汇报日期：2026-11-17 · 项目：System for future ai（sfai）· 状态：实验冻结（36 个结果文件校验封版）、论文框架就绪", size=9.5, color=(0x66, 0x66, 0x66))
para("仓库：https://github.com/4gcy6tzy6n-coder/SFai · 产品：macOS 桌面 App + CLI + HTTP API（v0.3.3 已发布）", size=9.5, color=(0x66, 0x66, 0x66))

# ================= 1. Story =================
heading("一、一句话故事", 1)
para("Agent 框架都报告「任务完成」——但谁验证交付物是真的对的？我们构建了 Rust 运行时 sfai（核心是模型自路由循环 Decide-Act），并提出一个衡量框架仪表盘漏掉的指标——假成功（false success）：状态「完成」但交付物错误的运行。在 18 个模型 × 4 个框架 + 3 个公开基准上的实证：")
p = doc.add_paragraph()
run = p.add_run("有验证门：假成功 0.000（72/72 格零例外）｜无验证门：平均 0.324（最高 1.00）")
run.bold = True; run.font.size = Pt(11); run.font.color.rgb = RGBColor(0xB0, 0x00, 0x00); set_cjk(run)
p2 = doc.add_paragraph()
run2 = p2.add_run("公开基准（HumanEval）上最强烈对照：无门 0.80 vs 有门 0.00")
run2.bold = True; run2.font.size = Pt(11); run2.font.color.rgb = RGBColor(0xB0, 0x00, 0x00); set_cjk(run2)

# ================= 2. System =================
heading("二、系统与方法（四个支柱）", 1)
heading("1. Decide-Act 内核（核心创新）", 2)
for t in [
    "每步一个 decide（模型从 9 类动作意图中选一个）+ 一个 act（运行时按策略执行）",
    "运行时保障：策略分级（读/可逆写/外部写/高影响）、审批收据、幂等键、循环守卫（同动作×3 无状态变化即拒绝）、路由遥测",
    "三层记忆 Index / Wiki / Way：Way 层记每任务的「方法/优点/出错重点」，AI 自写自记（对齐 Claude Code 记忆架构并扩展）",
]:
    para("• " + t)
heading("2. 验证门（消除假成功）", 2)
for t in [
    "运行时强制验收（工件存在/引用可解析/JSON 合法/答案匹配/测试通过/无占位符），有界修复；算错就阻塞，宁可如实报失败也不谎报成功",
    "门精度矩阵：捕获 1.00 / 误杀 0.00（零误杀）",
]:
    para("• " + t)
heading("3. 稀疏上下文激活（成本不随规模涨）", 2)
for t in [
    "预算化工作集（固定约束不驱逐、依赖一致、检索回执）",
    "同质量 1/78 token（160 文档历史：41 tok 恒定 vs 810→3210 tok 线性涨）",
]:
    para("• " + t)
heading("4. Rust 内核（系统优势）", 2)
for t in [
    "单二进制、事件账本（SQLite WAL）、崩溃恢复、内容寻址",
    "16 路并发 CPU 扩展 5.1×（Python 线程 1.0×，GIL 实证）；内存 2.6× 优；冷启动 255×–378× 优",
]:
    para("• " + t)

# ================= 3. Data =================
heading("三、核心数据（全部冻结，可复现）", 1)

heading("表 1｜18 模型主表（聚合，72 格）", 2)
table([
    ["配置", "假成功（均值）", "说明"],
    ["有验证门", "0.000", "72/72 格全绿，零例外"],
    ["无验证门", "0.324", "最高 1.00（deepseek-v4-pro / qwen3.7-max 多格）"],
])
para("模型家族：DeepSeek-chat、MiMo-v2.6-pro、百炼 16 文本模型（Qwen 3.6/3.7/3.8/4-max、GLM-5.2/5.3、DeepSeek-v4 系、auto、decision-model-preview）。无门失败模式互补：DeepSeek 抗捏造但算术弱、MiMo/Qwen 算得准但爱编引用——门对所有失败模式都有效。", size=9.5)

heading("表 2｜公开基准（真实迁移，NeurIPS D&B 硬要求达成）", 2)
table([
    ["套件", "来源", "无验证门", "有验证门"],
    ["HumanEval（20 题，官方测试为裁判）", "OpenAI，MIT", "success 0.80 / correct 0.00 / 假成功 0.80", "success 0.00 / correct 0.00 / 假成功 0.00"],
    ["GSM8K（20 题整数子集）", "OpenAI，MIT", "cor 0.70 / 假成功 0.15", "cor 0.95 / 假成功 0.00"],
    ["SpreadsheetBench-sheets（20 题）", "NeurIPS'24，CC-BY-SA-4.0", "0.15 / 0.15 / 0.00", "0.15 / 0.15 / 0.00（门中性）"],
])
para("HumanEval 的解读：无门时框架自信报告 80%「完成」，代码根本过不了测试；有门时全部被如实拦下——这正是「假成功」问题的最强实证。", size=9.5)

heading("表 3a｜比率分解（评审风险 #2 回应：零假成功 ≠ 零成功）", 2)
table([
    ["配置", "完成率", "正确率", "假成功/全部", "假成功/声称成功", "拒绝率"],
    ["E5-gate 有门", "0.47", "0.50", "0.00", "0.00", "0.47"],
    ["E5-gate 无门", "0.69", "0.31", "0.38", "0.55", "0.00"],
    ["GSM8K 有门", "0.95", "0.95", "0.00", "0.00", "0.05"],
    ["GSM8K 无门", "0.80", "0.70", "0.15", "0.19", "0.05"],
    ["HumanEval 有门", "0.00", "0.00", "0.00（平凡零）", "n/a", "1.00"],
    ["HumanEval 无门", "0.80", "0.00", "0.80", "1.00", "0.15"],
])
para("关键澄清：① 双分母口径已固化（主指标=假成功/全部运行；必附条件指标=假成功/声称成功）；② GSM8K 有门是健康对照（0.95 真实完成、非全拒绝）；③ HumanEval 有门 fs=0 是平凡零（无声称成功可假），证据价值是「无门声称成功 100% 是假的」，不解读为「门解决了任务」。", size=9.5)

heading("表 3｜假成功 CI 收紧（n=16）与门精度（配对检验修正版）", 2)
table([
    ["指标", "无验证门", "有验证门"],
    ["假成功（Wilson 95% CI）", "0.38 [0.18, 0.61]", "0.00 [0.00, 0.19]（旧「CI 不相交」表述有误，CI 于 0.18–0.19 重叠）"],
    ["配对 McNemar 精确检验", "—", "p=0.00049（b=12,c=0）；18 模型 176 对 p=4.4e-16"],
    ["门捕获率（拦截错交付）", "—", "1.00"],
    ["门误杀率（误拦对交付）", "—", "0.00"],
])
para("自路由真实行为：捏造压力下 ask_user 率 0.94（模型拒绝编造引用、反问用户）——Decide-Act 的「模型自路由」被真实使用。", size=9.5)

heading("表 4｜稀疏激活成本曲线（E5 sweep）", 2)
table([
    ["文档规模", "working_set（我们的）", "full_history"],
    ["40 docs", "41 tok", "810 tok"],
    ["80 docs", "41 tok", "1620 tok"],
    ["160 docs", "41 tok（恒定）", "3210 tok（78×）"],
])
para("质量持平（正确率同、约束保真 1.00 vs 0.00 衰减）；top-k 消融：k=2 于 160 文档翻车 → 32 处 target 权重锚定。", size=9.5)

heading("表 5｜Decide-Act 消融（自路由 vs 固定管线，n=4）", 2)
table([
    ["任务类型", "自路由", "固定管线", "结论"],
    ["long-horizon（检索密集）", "1.00", "0.00", "自路由决定性（固定检索路线全灭）"],
    ["dataset（流程化计算）", "0.25", "0.75", "固定管线更优（自路由开销）"],
    ["tool-use", "1.00（19k tok）", "1.00（5k tok）", "平，自路由 4× 成本"],
])
para("诚实双向结论：「适应性换开销」——流程固定时固定管线赢效率，任务不确定时只有自路由能完成。定位主张：route when uncertain, pipeline when certain（future work：混合路由）。", size=9.5)

heading("表 6｜跨框架对比（同模型 deepseek-chat、同判分器、n=6）", 2)
table([
    ["框架", "dataset 假成功", "tool-use 假成功", "p50"],
    ["LangGraph ReAct", "0.50", "0.00", "2.2s"],
    ["smolagents", "0.00", "0.00", "5.5s"],
    ["Pydantic AI", "0.00", "0.00", "3.9s"],
    ["sfai", "0.00", "0.00", "7.6s"],
])
para("诚实解读：修正判分器后框架差距收窄；差异在错误处理——同模型算错时无验证层框架静默交付（LangGraph 0.50），sfai 门拦截（0.00）。sfai 的成本是验证/修复轮 + 自路由轨迹（表 5 量化）。大样本佐证见表 1。", size=9.5)

heading("表 7a｜E10 等验证器对等实验（评审风险 #3 回应：门的独特性）", 2)
table([
    ["臂（同任务/同模型/同判分/同修复预算 2 轮）", "完成率", "正确率", "fs/声称成功", "失败性质"],
    ["sfai（门+运行时）", "1.00", "1.00", "0.00", "无"],
    ["LangGraph 无门", "0.83", "0.83", "0.00", "2 次裸异常丢任务"],
    ["LangGraph + 等验证器", "0.92", "0.92", "0.00", "1 次裸异常→门阻断"],
])
para("诚实结论：① 低错误任务上框架身份不重要——门价值未触发（0 修复），与评审预判一致；② 有效验证是活性成分，收益随模型错误率放大（18 模型/HumanEval 高错误场景 fs→0.00）；③ Decide-Act 独特价值 = 运行时鲁棒性（12/12 vs 裸异常丢任务）+ 审计账本 + E8 自路由适应性 + 三层记忆。", size=9.5)

heading("表 7｜性能（对齐口径 + Rust vs Python）", 2)
table([
    ["指标", "sfai (Rust)", "对照", "比值"],
    ["单任务 p50", "1.54s", "LangGraph 2.2s / DeepAgents 3.5s", "1.4×–2.3× 快"],
    ["冷启动", "0.6–1.0ms", "DeepAgents 255ms / Claude Agent SDK 378ms", "255×–378×"],
    ["200k 信封构建", "4.2ms", "—", "微秒级激活"],
    ["16 路并发 CPU 扩展", "5.1×", "Python 线程 1.0×（GIL）", "GIL 实证"],
    ["内存 RSS", "10.6MB", "14.7–27.6MB", "2.6× 优"],
])

# ================= 4. Credibility =================
heading("四、数据可信度（如实披露，附录级）", 1)
for t in [
    "统计推断修正：旧「CI 不相交」表述废除（0.18–0.19 重叠）；配对设计改用 McNemar 精确检验；假成功双分母口径固化（benchmark-spec v0.2.2）",
    "判分器三次事故与修正：键名强校验 → 值匹配 → 数值比较 → 文件名无关+任务间清理；所有作废版本不入表（Debug 报告 #13）",
    "E9 网络事故：HumanEval 两轮（27 超时 + 37 DNS 失败）作废归档后改百炼补测完成（Debug 报告 #14）",
    "并发扫描延迟噪声：跨模型延迟对比仅供参考；成功率/正确率/token 不受影响",
    "样本量：n=2–16/cell（Wilson CI 全表给出）；订阅制计价（比 token 不比美元）；合成+公开混合任务",
    "E8 双向结果如实呈现（不主张自路由处处更优）",
]:
    para("• " + t)

# ================= 5. Paper plan =================
heading("五、论文计划", 1)
for t in [
    "题目方向：Decide-Act: An Auditable Agent Runtime That Eliminates Silent Failures（18 模型 × 4 框架基准）",
    "结构：Intro → Related Work（MemGPT/SLIM/Context Folding/验证环文献）→ Runtime（Decide-Act 形式化 + 三层记忆 + 门 + 稀疏激活）→ Benchmark → Experiments（T1–T9）→ Analysis → Limitations → Conclusion",
    "投稿定位：TMLR（主，实证+可复现）/ NeurIPS Datasets & Benchmarks（备，公开任务集已达硬要求）/ NMI（补料后冲刺）",
    "写作顺序：§4 Benchmark → §5 表格 → §3 方法 → §1/§2 → §6/§7 → 摘要",
    "全部数字来源：docs/paper/data-summary.md（程序化提取）；36 文件校验冻结：docs/paper/data-freeze.md",
]:
    para("• " + t)

# ================= 6. Questions =================
heading("六、请导师定夺的问题", 1)
for i, t in enumerate([
    "投稿目标：TMLR 稳 vs NeurIPS D&B 冲刺（公开任务集已备）——选哪条线？",
    "E8 叙事：「适应性换开销」的双向结论是否按此如实写，还是补混合路由实验后再写？",
    "公开基准扩容：HumanEval/GSM8K/sheets 之外是否再迁 1–2 个（如 GAIA 子集）增强 D&B 竞争力？",
    "作者与致谢安排。",
], 1):
    para(f"{i}. {t}")

doc.save(OUT)
print("saved:", OUT)
