# RustNoteSearch

## 正式知识库使用方式

本项目的正式知识库目录为 `knowledge_base/`。用户可以手动添加、删除、移动其中的文件和子文件夹；程序不会限制目录层级。修改知识库内容后，需要更新索引：

```bash
cargo run -- index knowledge_base
```

也可以在 TUI 中输入 `/update` 直接重新扫描当前知识库根目录并保存索引。

TUI 启动后默认进入封面命令模式：

- `/help`：展示所有 TUI 命令。
- `/files`：展示当前索引中的知识库目录。
- `/terms`：展示词频排序。
- `/stats`：展示索引统计。
- `/update`：更新索引。
- `/select`：进入搜索页面。
- `/quit`：退出。

推荐演示命令：

```bash
cargo run -- index knowledge_base
cargo run -- stats
cargo run -- search ownership
cargo run -- search 事务
cargo run -- tui
```

## 本轮优化说明

- 搜索结果现在支持同一文档内的多个命中片段，CLI 会按 `match 1`、`match 2` 展示，TUI 预览区会同步列出多处命中。
- PDF 正文命中会记录页码，CLI 和 TUI 会以 `p.页码` 的形式展示，方便用户手动定位到对应页面。
- PDF 文件仍通过系统默认程序打开；本轮不做 PDF 阅读器级别的精确页内跳转。
- 索引格式版本已升级到 `3`，旧的 `index.json` 需要重新运行 `cargo run -- index knowledge_base` 生成。

RustNoteSearch 是一个使用 Rust 编写的本地知识库搜索工具。它可以扫描 Markdown、纯文本、Rust 源码、TOML 配置文件和文本型 PDF，构建 JSON 格式的倒排索引，并支持基于 BM25 的命令行搜索和轻量级终端 TUI 搜索界面。


本项目面向 Rust 课程大作业设计，可以直接在本地 Windows 环境运行，不需要 Docker 或其他容器环境。

## 功能特性

- 递归扫描本地文件夹。
- 支持 `.md`、`.txt`、`.rs`、`.toml`、`.pdf` 文件。
- 自动跳过 `.git`、`target` 和隐藏目录。
- 自动提取 Markdown 标题。
- 支持英文、代码标识符和基于 `jieba-rs` 的中文分词。
- 构建包含词频和位置的倒排索引。
- 使用可读的 JSON 文件保存和加载索引。
- 使用 BM25 算法进行相关度排序，支持命中词展示和命中片段预览。
- 提供简单的 TUI 交互式搜索界面，支持 Enter 或鼠标点击打开搜索结果文件。
- 提供索引统计、高频词分析、单文档分析和 Markdown 报告导出。
- 包含单元测试和集成测试。

## 快速开始

```bash
cd /d D:\AGENT_workspace\rust大作业\rust-note-search
cargo build
cargo run -- index knowledge_base
cargo run -- stats
cargo run -- search ownership
cargo run -- search 所有权
cargo run -- terms
cargo run -- inspect 0
cargo run -- report
cargo run -- tui
```

默认情况下，索引会保存到 `index.json`。

## 命令说明


构建索引：

```bash
cargo run -- index knowledge_base
```

构建索引并保存到指定文件：

```bash
cargo run -- index knowledge_base -o knowledge-index.json
```

执行搜索：

```bash
cargo run -- search "ownership borrowing"
cargo run -- search "所有权"
```

使用指定索引文件并限制结果数量：

```bash
cargo run -- search "rust trait" -i knowledge-index.json -n 5
```

要求搜索结果同时包含所有查询词：

```bash
cargo run -- search "ownership borrowing" --mode all
```

查看索引统计信息：

```bash
cargo run -- stats
```

查看高频词：

```bash
cargo run -- terms -n 15
```

按文档编号查看单个文档的分析结果：

```bash
cargo run -- inspect 0
```

导出 Markdown 报告：

```bash
cargo run -- report -o index-report.md
```

打开终端 TUI：

```bash
cargo run -- tui
```

TUI 按键说明：

- 直接输入内容进行搜索。
- 使用 Backspace 删除字符。
- 使用 Up / Down 选择搜索结果。
- 按 Enter 打开当前选中的搜索结果文件。
- 鼠标点击 Results 列表中的结果项可直接打开文件。
- 使用 Esc 退出。
- 当输入框为空时，按 `q` 退出。

## 项目结构

项目按照职责拆分为多个模块：

- `cli`：命令行参数和子命令定义。
- `error`：统一的 `AppError` 和 `AppResult` 错误处理。
- `model`：核心数据结构和枚举类型。
- `scanner`：目录遍历和文件过滤。
- `parser`：标题提取、PDF 文本提取、英文/代码标识符分词和 `jieba-rs` 中文分词。
- `index`：并发构建倒排索引。
- `storage`：JSON 索引持久化。
- `search`：BM25 搜索排序和命中片段生成。
- `analysis`：索引统计、高频词和单文档分析。
- `report`：Markdown 报告导出。
- `ui`：终端 TUI 用户界面。

## Rust 特性体现

- 所有权与借用：在文件内容解析、token 处理和索引构建中控制数据所有权和借用关系。
- `struct`：用于表示文档、倒排项、索引元信息、搜索结果和分析结果。
- `enum`：用于表示文件类型、搜索状态、搜索模式和应用错误。
- `trait`：使用 `Tokenizer` 抽象分词策略，默认实现内部使用 `jieba-rs` 处理中文。
- 泛型：用于搜索引擎的分词器参数、JSON 读写辅助函数和通用截断逻辑。
- `Result` 错误处理：贯穿文件 I/O、JSON 解析、索引构建、搜索和 TUI。
- 并发：使用 `rayon` 并发解析多个文件，再汇总生成倒排索引。
- 模块化：项目拆分为多个清晰模块，满足课程对工程结构的要求。
- PDF 解析：使用 `pdf-extract` 支持文本型 PDF 内容检索。
- 文件跳转：TUI 中可通过 Enter 或鼠标点击打开命中的本地文件。

## 测试

运行所有测试：

```bash
cargo test
```

运行格式化和 lint 检查：

```bash
cargo fmt
cargo clippy
```

## 演示脚本

1. 展示 README 和 `knowledge_base` 正式知识库目录。
2. 运行 `cargo run -- index knowledge_base` 构建索引。
3. 运行 `cargo run -- stats` 查看索引统计。
4. 运行 `cargo run -- search ownership` 展示英文搜索。
5. 运行 `cargo run -- search 所有权` 展示中文搜索。
6. 运行 `cargo run -- terms` 展示高频词。
7. 运行 `cargo run -- inspect 0` 展示单文档分析。
8. 运行 `cargo run -- report` 导出 Markdown 报告。
9. 运行 `cargo run -- tui` 展示 TUI 搜索界面。
10. 在 TUI 中搜索关键词，演示高亮片段、Enter 打开文件和鼠标点击打开文件。
11. 简要介绍核心实现：倒排索引、BM25、PDF 提取、`Tokenizer` trait、`Result` 错误处理、`rayon` 并发和模块化设计。
