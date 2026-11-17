# sfai · 对话工作台

一个对话优先、可交互的 sfai 前端原型。目标、决策、工具回执、审批与核验都留在同一段会话里；任务详情、轨迹、上下文、能力和运行设置收进设置层，不再占据主界面。

## 打开方式

直接打开根目录的 `index.html`。没有构建步骤，字体、图标和脚本都在本地。

如果浏览器限制本地文件访问，也可以启动一个静态服务器：

```bash
python3 -m http.server 8080
```

然后访问 `http://localhost:8080/`。

## 主界面

- **会话栏**：新建对话、搜索历史、切换会话；移动端为抽屉。
- **对话流**：用户消息、sfai 回复、内联工具回执、审批卡、核验收据。
- **输入区**：固定在底部；`Enter` 换行，`⌘↵` 发送。
- **顶栏**：当前会话标题与状态、模型入口、命令搜索、设置入口。

日常使用只需要对话。工具执行和审批不会跳出会话；需要查看完整事件账本时再进入设置。

## 设置与运行详情

从顶栏齿轮或模型入口打开，包含五个页签：

| 页签 | 内容 |
|---|---|
| 任务 | 目标、固定约束、验收规则、当前 Skill、最近核验、工作集、预算 |
| 轨迹 | Duration / Turns / Steps / Cost 摘要、活动标尺、按轮次展开的事件账本、JSON 导出 |
| 上下文 | 检索引用、查看有界摘录与 provenance、加入/移出工作集 |
| 能力 | 能力注册表、风险分级、调用示例、Skill 激活 |
| 设置 | Provider endpoint、主模型、审批策略、主题、信息密度、动效、消息时间、发送方式、自动跟随、重置演示数据 |

## 可以操作的流程

- 新建对话，填写目标；高级设置里可补充约束、验收规则、模型与预算。
- 发送补充消息，写入本地事件账本并模拟一次 `capabilities.discover`。
- 批准一次、始终允许或拒绝高影响操作；批准后模拟 `workspace.exec` 与 `verify.run`，对话、任务状态和费用一起更新。
- 展开工具参数与结果；在设置里按类型筛选轨迹并导出 JSON。
- 在设置里检索上下文、加入工作集；激活或停用 Skill，只影响当前会话。
- 在设置里切换主题、信息密度、消息时间、发送方式与自动跟随；配置保存在浏览器本地。
- `⌘K` 命令面板：新建对话、打开任意设置页签、切换会话、导出或复制会话 ID。
- 桌面与移动端均可键盘操作；移动端有安全区与 44px 触控目标。

## 设计方向

**Graphite × Paper × Signal Blue**：冷灰纸面、墨黑正文、单一信号蓝强调色、1px 发丝分隔线、克制的 3–8px 圆角，不使用渐变、玻璃拟态、霓虹或装饰性阴影。

- 英文 UI：Instrument Sans；中文回退到 PingFang SC / Microsoft YaHei。
- 标识符、时间、成本、事件序列：IBM Plex Mono。
- 用户消息是一块紧凑的浅色文本面；sfai 回复直接落在纸面上，左侧一条细线只标记当前回复线程。
- 对话栏 48rem 居中，输入区与正文左右对齐；不会出现左右聊天气泡和卡片堆叠。
- 品牌标记是“三节点、两条路径汇聚”的自路由符号；矢量源文件与使用规范在 `assets/logo/`。
- 动效只用于状态变化与结果反馈，支持 `prefers-reduced-motion` 与设置里的“减少动效”。
- 深浅色各自定义语义 token。

## 文件结构

```text
index.html                 对话壳、设置层与对话框
styles/tokens.css          色板、字体、间距、动效 token
styles/base.css            重置、排版、按钮、表单
styles/overlays.css        对话框、toast、滚动条、reduced-motion
styles/page.css            页面与状态徽章
styles/views-tasks.css     工具回执、审批、核验等共享组件
styles/views-trajectory.css 轨迹视图
styles/views-context.css   上下文视图
styles/views-capabilities.css 能力与 Skill 视图
styles/views-responsive.css 视图响应式规则
styles/palette.css         命令面板
styles/settings.css        设置层框架与数据面板
styles/settings-controls.css 设置分组、开关与主题控件
styles/settings-responsive.css 设置层响应式
styles/chat.css            会话栏、顶栏、对话流
styles/composer.css        底部输入区
styles/chat-responsive.css 对话响应式与安全区
js/data.js                 演示数据：会话、事件、审批、引用、能力、Skill
js/icons.js                单线 SVG 图标与品牌标记
js/util.js                 转义、高亮、剪贴板、下载、toast、对话框
js/store.js                本地状态、持久化、模拟决策/审批/核验
js/views-chat.js           对话流
js/views-settings.js       设置页签
js/views-tasks.js          会话列表与共享事件组件
js/views-trajectory.js     轨迹
js/views-context.js        上下文
js/views-capabilities.js   能力
js/palette.js              命令面板
js/app.js                  应用壳、渲染、交互、键盘与响应式
assets/fonts/              本地字体与 OFL 许可
assets/logo/               品牌标记、应用图标与使用规范
assets/favicon.svg         品牌 favicon
```

## 数据边界

当前版本是**本地演示**：

- 顶栏与设置层明确标注“本地演示 / 未连接运行时”。
- `js/data.js` 是初始数据，`js/store.js` 是唯一状态入口。修改保存在 `localStorage` 的 `sfai.console.v2` 键下；本地存储不可用时退回内存状态。
- Provider endpoint 只保存到本机，不发起网络请求。
- 审批、执行、核验是客户端模拟：批准后会出现运行回执与核验结果，用于验证交互与状态一致性，不代表真实 Rust runtime 已执行。

接入真实运行时的时候，保留 `views-chat.js`、`views-settings.js` 的渲染逻辑，把 `store.js` 中的会话加载、事件读取、发送消息、审批、工作集和 Skill 操作替换为 HTTP/SSE 调用。事件账本仍是唯一事实来源；前端不应自行推导“已完成”或“已核验”。

## 已知边界

- 没有真实后端、鉴权、多租户或服务端持久化。
- 演示费用、token、时长与事件内容是示例数据，用于展示信息结构。
- 尚未实现真实模型流式输出、MCP 连接器、任务删除/重命名与多用户协作。
- 字体许可见 `assets/fonts/`。
