# 仓库拆分说明（2026-11-17，ICML 2027 双论文）

| 仓库 | 论文 | 内容 | 可见性 |
|---|---|---|---|
| **[agent-reliability-benchmark](https://github.com/4gcy6tzy6n-coder/agent-reliability-benchmark)** | Paper A（Datasets/Evaluations） | 评价协议（Y/C/A 六指标）、公开任务集、判分器审计、跨框架接口、18 模型普查结果、统计工具 | 私有 |
| **[decide-act](https://github.com/4gcy6tzy6n-coder/decide-act)** | Paper B（General） | Rust 内核（Decide-Act + 三层记忆 + 验证门 + Runtime）、路由消融、B0–B4 因果消融、桌面 App | 私有 |
| **SFai（本仓）** | — | 产品与发布（App/CLI/Release）、全量历史 | 公开（现状） |

**规则：**
- 两新仓**私有**（双盲投稿：不能有可识别作者的公开仓）；录用后再决定公开策略；
- 两仓互为同期相关工作，README 已含**双向披露声明**（ICML 2027 双投要求）；
- 数据共享、贡献不重叠（边界裁决表见 `docs/paper/dual-paper-protocol.md`）；
- 本仓继续作为产品仓（v0.3.3 发布版）与全量开发历史。
