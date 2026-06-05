# CheeseBase

CheeseBase 是一个基于 Rust 实现的本地知识库混合检索系统，面向课程笔记、代码文件、数据库资料和论文 PDF 等个人学习资料管理场景。用户将文件放入 `knowledge_base/` 目录后，程序可以递归扫描文档、解析正文、构建本地索引，并通过 CLI 或 TUI 完成检索、预览和文件跳转。

项目默认使用 BM25 本地关键词检索；如果配置了阿里云百炼 DashScope Embedding 和本地 Qdrant，也可以启用 BM25 + 向量检索的 Hybrid 混合检索。

项目全名：

```text
CheeseBase：基于 Rust、BM25 与向量数据库的本地知识库混合检索系统
```

## 功能特性

- 支持正式知识库目录 `knowledge_base/`，用户可以自由添加、删除、移动文件和子文件夹。
- 支持递归扫描多级目录。
- 支持 `.md`、`.txt`、`.rs`、`.toml`、`.pdf` 文件。
- 支持文本型 PDF 正文提取和页码提示。
- 支持基于 `jieba-rs` 的中文分词，以及英文、数字、代码标识符分词。
- 支持倒排索引构建，并使用 JSON 保存为 `index.json`。
- 支持 BM25 相关度排序。
- 支持同一文档内多处命中片段展示。
- 支持 Qdrant + DashScope Embedding 的 Hybrid 混合检索。
- 支持 CLI 命令和终端 TUI 两种交互方式。
- TUI 支持首页、帮助页、目录页、词频页、统计页、策略选择页和搜索页。
- TUI 支持 `/update` 更新索引、`/strategy` 选择检索策略、Enter 或鼠标点击打开文件。
- 包含单元测试和集成测试，方便验证核心功能。

## 环境要求

基础 BM25 功能只需要 Rust 工具链：

- Windows / macOS / Linux
- Rust stable
- Cargo

安装 Rust：

```bash
rustup --version
cargo --version
```

如果命令不可用，请先安装 Rust：

```text
https://www.rust-lang.org/tools/install
```

Windows 用户需要确认 Cargo 路径已经加入环境变量：

```text
C:\Users\<你的用户名>\.cargo\bin
```

Hybrid 混合检索额外需要：

- 本地 Qdrant 服务
- 阿里云百炼 / DashScope API Key
- `.env` 配置文件

## 快速开始

进入项目目录：

```bash
cd rust-note-search
```

构建项目：

```bash
cargo build
```

构建本地知识库索引：

```bash
cargo run -- index knowledge_base
```

查看索引统计：

```bash
cargo run -- stats
```

执行 BM25 搜索：

```bash
cargo run -- search ownership --strategy bm25
cargo run -- search 所有权 --strategy bm25
```

打开 TUI：

```bash
cargo run -- tui
```

## 命令说明

### 构建索引

```bash
cargo run -- index knowledge_base
```

指定输出索引文件：

```bash
cargo run -- index knowledge_base -o index.json
```

### 搜索

默认使用 BM25：

```bash
cargo run -- search "rust ownership"
```

指定搜索策略：

```bash
cargo run -- search "rust ownership" --strategy bm25
cargo run -- search "事务隔离级别" --strategy hybrid
```

限制返回数量：

```bash
cargo run -- search "trait" -n 5
```

要求查询词全部命中：

```bash
cargo run -- search "ownership borrowing" --mode all
```

### 查看统计

```bash
cargo run -- stats
```

### 查看高频词

```bash
cargo run -- terms
cargo run -- terms -n 30
```

### 查看单个文档分析

```bash
cargo run -- inspect 0
```

### 导出索引报告

```bash
cargo run -- report
cargo run -- report -o index-report.md
```

### 构建向量索引

使用 Hybrid 检索前，需要先构建本地 BM25 索引，再构建 Qdrant 向量索引：

```bash
cargo run -- index knowledge_base
cargo run -- vector-index
```

## Hybrid 检索配置

Hybrid 检索需要 `.env` 文件。请复制模板：

```bash
copy .env.example .env
```

macOS / Linux：

```bash
cp .env.example .env
```

然后填写自己的 DashScope API Key：

```text
EMBED_MODEL_TYPE=dashscope
EMBED_MODEL_NAME=text-embedding-v3
EMBED_API_KEY=your_dashscope_api_key_here
EMBED_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
EMBED_DIMENSIONS=1024

QDRANT_URL=http://localhost:6333
QDRANT_COLLECTION=cheesebase_chunks
HYBRID_SCORE_THRESHOLD=0.45
```

注意：

- 不要把真实 `.env` 提交到 GitHub。
- 当前 `.gitignore` 已忽略 `.env`。
- `.env.example` 只保留占位符，方便别人了解配置项。

`HYBRID_SCORE_THRESHOLD` 是混合检索结果阈值，范围为 `0.0` 到 `1.0`。数值越高，结果越少但通常更相关。默认值为 `0.45`。

Hybrid 得分融合公式：

```text
bm25_norm = bm25_score / max_bm25_score
vector_norm = clamp(vector_score, 0.0, 1.0)
hybrid_score = 0.45 * bm25_norm + 0.55 * vector_norm
```

## TUI 使用说明

启动：

```bash
cargo run -- tui
```

TUI 默认进入首页，可以输入以下命令：

```text
/help      查看帮助
/select    进入搜索页
/home      返回首页
/files     查看知识库文件目录
/terms     查看高频词
/stats     查看索引统计
/strategy  选择 BM25 或 Hybrid 检索策略
/update    重新扫描知识库并更新索引
/clear     清空搜索输入
/quit      退出程序
```

搜索页支持：

- 直接输入关键词实时搜索。
- Up / Down 选择搜索结果。
- Enter 打开当前选中的文件。
- 鼠标点击结果列表中的文件打开。
- 鼠标滚轮滚动 Preview 区域，查看同一文档中的多个命中片段。
- 输入 `/strategy` 切换 BM25 或 Hybrid。
- 输入 `/update` 更新索引。

## 项目结构

```text
rust-note-search/
  Cargo.toml
  README.md
  .env.example
  knowledge_base/
  src/
    main.rs
    lib.rs
    cli.rs
    config.rs
    embedding.rs
    error.rs
    hybrid.rs
    index.rs
    model.rs
    parser.rs
    scanner.rs
    search.rs
    storage.rs
    ui.rs
    vector.rs
  tests/
```

核心模块说明：

- `cli`：命令行参数和子命令定义。
- `error`：统一 `AppError` 和 `AppResult` 错误处理。
- `model`：文档、索引、搜索结果和检索策略等核心数据结构。
- `scanner`：目录递归扫描和文件类型过滤。
- `parser`：文本解析、PDF 提取、标题提取和分词。
- `index`：并发构建倒排索引。
- `search`：BM25 检索、排序和命中片段生成。
- `storage`：JSON 索引保存与加载。
- `vector`：chunk 切分、Qdrant 写入和向量检索。
- `embedding`：DashScope Embedding 客户端。
- `hybrid`：BM25 与向量检索结果融合。
- `config`：读取 `.env` 和环境变量配置。
- `analysis`：索引统计、高频词和文档分析。
- `report`：Markdown 报告导出。
- `ui`：终端 TUI 界面。

## Rust 工程实践

本项目重点体现以下 Rust 工程能力：

- 模块化设计：通过 `lib.rs` 拆分并导出多个功能模块。
- crate 管理：项目由 `Cargo.toml` 管理依赖和构建流程。
- 错误处理：使用 `AppResult<T> = Result<T, AppError>` 统一错误返回。
- 错误传播：使用 `?` 传播 IO、JSON、PDF、HTTP、Qdrant 等错误。
- ownership / borrowing：文档正文由 `Document` 拥有，搜索阶段借用 `InvertedIndex`。
- struct / enum：使用结构体建模文档、索引、搜索结果，使用枚举表达文件类型、搜索策略和错误类型。
- trait：使用 `Tokenizer` 抽象分词器。
- 泛型：索引构建器、搜索引擎、TUI 和 JSON 读写使用泛型提高复用性。
- 生命周期：`SearchEngine<'a, T>` 借用索引并通过生命周期保证引用安全。
- 并发：使用 `rayon` 并发解析多个文件。
- 测试：包含单元测试和集成测试。

## 测试与检查

运行测试：

```bash
cargo test
```

检查格式：

```bash
cargo fmt --check
```

运行 Clippy：

```bash
cargo clippy -- -D warnings
```

提交前推荐完整执行：

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## PDF 支持说明

CheeseBase 当前支持文本型 PDF。也就是说，PDF 中的正文需要能够被复制、选中，程序才能通过 `pdf-extract` 提取文本并建立索引。

如果 PDF 是扫描图片生成的，正文实际是图片，不是文本。当前版本暂不支持 OCR。后续可以接入 Tesseract OCR 或 PaddleOCR，将扫描图片识别为文本后再进入索引流程。

## 常见问题

### cargo 命令不可用

请确认 Rust 已安装，并且 Cargo 路径已加入环境变量：

```text
C:\Users\<你的用户名>\.cargo\bin
```

重新打开终端后再执行：

```bash
cargo --version
```

### 找不到 Cargo.toml

需要在项目根目录运行 Cargo 命令：

```bash
cd rust-note-search
cargo build
```

### Hybrid 搜索失败

请检查：

- `.env` 是否存在。
- `EMBED_API_KEY` 是否填写。
- Qdrant 是否已经启动。
- `QDRANT_URL` 是否能访问。
- 是否已经执行 `cargo run -- vector-index`。

## 演示建议

推荐演示顺序：

1. 展示 `knowledge_base/` 多级目录。
2. 运行 `cargo run -- index knowledge_base`。
3. 运行 `cargo run -- stats`。
4. 运行 `cargo run -- search ownership --strategy bm25`。
5. 运行 `cargo run -- search 事务 --strategy hybrid`。
6. 运行 `cargo run -- terms`。
7. 运行 `cargo run -- tui`。
8. 在 TUI 中演示 `/help`、`/files`、`/strategy`、`/select`、`/update`。

## 安全说明

- `.env` 中可能包含真实 API Key，不能提交到 GitHub。
- `.env.example` 只包含示例配置和占位符，可以提交。
- `target/` 是编译产物，不需要提交。
- `index.json` 是本地生成的索引文件，通常不需要提交。
