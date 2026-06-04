# RustNoteSearch 本地知识库搜索系统实现计划

## 当前增量计划：正式知识库与 TUI 命令首页

- 主知识库目录切换为 `knowledge_base/`，目录结构只是初始样例，用户后续可以自由添加、删除和移动文件夹或文件。
- CLI 更新索引使用 `cargo run -- index knowledge_base`。
- TUI 启动进入 Home 封面页，展示 RustNoteSearch、小狗像素画、励志短句、索引摘要和命令提示。
- TUI 支持 `/help`、`/files`、`/terms`、`/stats`、`/update`、`/select`、`/home`、`/clear`、`/quit`。
- `/update` 使用当前索引记录的 root 目录重新构建索引，保存回当前 TUI 加载的 index 文件；失败时保留旧索引。
- `/files`、`/terms`、`/stats` 都基于当前索引，用户修改知识库后需要执行 `/update` 或重新运行 CLI 索引命令。

## 当前增量计划：同文档多命中与 PDF 页码

- 将倒排索引中的单纯位置列表升级为 occurrence 列表，保存 token 位置、字符范围和可选 PDF 页码。
- 搜索结果中的一个文档可以包含多个 `SearchMatch`，用于展示同文档内多处命中内容。
- CLI 搜索输出显示前几处 `match`，PDF 命中会附带 `p.页码`。
- TUI Results 列表显示文件名和页码摘要，Preview 区展示多处命中片段。
- PDF 打开仍使用系统默认程序，本轮只展示页码，不做阅读器级精确跳页。
- 索引版本升级到 `3`，需要重新生成 `index.json`。

## 1. 项目概述

RustNoteSearch 是一个可直接在本地 Windows 环境运行的 Markdown / 文本 / 代码 / PDF 文件知识库搜索系统。用户指定目录后，程序递归扫描文件、解析文本、建立倒排索引，并支持 BM25 命令行搜索、索引分析与简易 TUI 搜索界面。

项目不使用 Docker，不依赖容器环境，只使用标准 Rust 工具链：

```bash
cargo build
cargo test
cargo run -- index knowledge_base
cargo run -- search ownership
cargo run -- report
cargo run -- tui
```

目标代码规模控制在 1500-3000 行有效 Rust 代码，重点体现 Rust 的模块化设计、错误处理、所有权与借用、struct / enum / trait / 泛型、并发处理和测试能力。

## 2. 功能与命令

采用 CLI + 简易 TUI 的实现范围：

```bash
cargo run -- index <path> [-o index.json]
cargo run -- search <query> [-i index.json] [-n 10] [--mode any|all]
cargo run -- stats [-i index.json]
cargo run -- terms [-i index.json] [-n 20]
cargo run -- inspect <doc_id> [-i index.json] [-n 12]
cargo run -- report [-i index.json] [-o index-report.md] [-n 20]
cargo run -- tui [-i index.json]
```

核心功能：

- `index <path>`：递归扫描 `.md`、`.txt`、`.rs`、`.toml`、`.pdf` 文件，建立倒排索引并保存为 JSON。
- `search <query>`：加载索引，使用 BM25 执行多关键词搜索，展示文档编号、路径、标题、相关度分数和命中片段。
- `--mode any|all`：支持任意关键词命中或要求全部关键词命中。
- `stats`：展示文档数量、词项数量、总 token 数、平均文档长度、高频词和最大文档。
- `terms`：展示索引中的高频词，辅助分析知识库内容。
- `inspect <doc_id>`：查看指定文档的元信息、高频词和内容预览。
- `report`：导出 Markdown 索引报告，方便实验报告引用。
- `tui`：进入终端搜索界面，支持输入关键词、上下选择结果、查看高亮片段，并通过 Enter 或鼠标点击打开结果文件。

明确不做 Web 前端、不使用 Docker、不调用搜索 API 或 AI API、不实现复杂数据库和复杂中文 NLP。

## 3. 模块设计

源码模块：

```text
src/
  main.rs
  lib.rs
  cli.rs
  error.rs
  model.rs
  scanner.rs
  parser.rs
  index.rs
  search.rs
  storage.rs
  analysis.rs
  report.rs
  ui.rs
```

核心类型：

- `Document`：文档 ID、路径、标题、文件类型、修改时间、文件大小、token 数。
- `Posting`：某个词在某个文档中的出现频次和位置。
- `InvertedIndex`：文档集合、倒排表、索引元信息。
- `SearchResult`：命中文档、分数、命中片段。
- `AppError`：统一表示 IO、JSON、路径、索引、终端 UI 等错误。
- `Tokenizer` trait：分词器抽象。
- `SimpleTokenizer`：默认中英文分词实现。

依赖：

- `clap`：命令行参数解析。
- `walkdir`：递归目录扫描。
- `serde` / `serde_json`：索引 JSON 持久化。
- `thiserror`：自定义错误类型。
- `rayon`：并发解析文件。
- `pdf-extract`：提取文本型 PDF 内容。
- `open`：使用系统默认程序打开搜索结果文件。
- `ratatui` / `crossterm`：终端 TUI。
- `tempfile`：测试临时目录。

## 4. 实现步骤

1. 创建 Rust 二进制项目，配置依赖，完成 CLI 命令入口和统一错误处理。
2. 实现目录扫描，支持 `.md`、`.txt`、`.rs`、`.toml`、`.pdf`，跳过 `.git/`、`target/`、隐藏目录。
3. 实现文本解析、PDF 文本提取和中英文分词。英文、数字、代码标识符按边界切分；中文使用 `jieba-rs`。
4. 实现倒排索引，记录文档元信息、词频和位置，并使用 `rayon` 并发解析文件。
5. 实现 JSON 保存和加载，默认索引文件为 `index.json`。
6. 实现 BM25 关键词搜索、Top-N 截断、`any/all` 匹配模式和命中片段生成。
7. 实现索引分析能力：高频词、最大文档、单文档高频词和内容预览。
8. 实现 Markdown 报告导出能力，输出摘要、高频词和文档表格。
9. 实现单页面 TUI：输入框、结果列表、片段预览，支持退格、上下键、Enter 打开文件、鼠标点击打开文件、`q` / `Esc` 退出。
10. 编写 README、示例数据、单元测试和集成测试。
11. 本地运行 `cargo fmt`、`cargo clippy`、`cargo test` 和演示命令完成验收。

## 5. Rust 特性展示点

- 所有权与借用：文本解析、token 处理、索引构建中控制数据所有权和借用。
- struct：`Document`、`Posting`、`InvertedIndex`、`SearchResult`、`IndexSummary`。
- enum：`AppError`、`SupportedFileType`、`SearchState`、`SearchMode`。
- trait：`Tokenizer` 抽象分词策略。
- 泛型：Top-N 截断、JSON 读写辅助函数、搜索引擎的 tokenizer 泛型。
- Result 错误处理：文件读取、PDF 提取、JSON 加载、路径错误、TUI 错误统一传播。
- 并发：`rayon` 并发解析多个文件后汇总生成索引。
- 模块化：项目拆分为多个模块，满足课程对模块化设计的要求。
- BM25：使用正式检索排序公式替换简化词频评分。
- 文件跳转：TUI Results 列表支持 Enter 和鼠标点击打开本地文件。

## 6. 测试与验收

单元测试：

- 中英文 tokenizer。
- Markdown 标题提取。
- 文件类型过滤。
- PDF 文本提取和 PDF 错误处理。
- 倒排索引词频和位置。
- BM25 排序和 `any/all` 匹配模式。
- 命中片段生成。
- TUI Results 行点击映射和文件打开失败处理。
- 高频词和单文档分析。
- Markdown 报告导出。

集成测试：

- 创建临时目录和测试文件。
- 完成扫描、索引、保存、加载、搜索全流程。
- 验证搜索结果包含预期文档。
- 验证不存在索引文件和损坏 JSON 的错误处理。

本地验收：

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo run -- index knowledge_base
cargo run -- stats
cargo run -- terms
cargo run -- inspect 0
cargo run -- report
cargo run -- search ownership
cargo run -- search 所有权
cargo run -- tui
```

## 7. 演示视频脚本

1. 展示项目目录和 README。
2. 运行 `cargo run -- index knowledge_base` 构建索引。
3. 运行 `cargo run -- stats` 查看统计信息。
4. 运行 `cargo run -- terms` 展示高频词。
5. 运行 `cargo run -- search ownership` 展示英文搜索。
6. 运行 `cargo run -- search 所有权` 展示中文搜索。
7. 运行 `cargo run -- inspect 0` 展示单文档分析。
8. 运行 `cargo run -- report` 导出 Markdown 报告。
9. 运行 `cargo run -- tui` 展示 TUI 搜索。
10. 在 TUI 中演示高亮片段、Enter 打开文件和鼠标点击打开文件。
11. 简要介绍核心实现：倒排索引、BM25、PDF 提取、分词器 trait、Result 错误处理、rayon 并发、模块化设计。
