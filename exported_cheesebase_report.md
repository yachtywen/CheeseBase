# CheeseBase：基于 Rust、BM25 与向量数据库的本地知识库混合检索系统

## 一、项目简介

CheeseBase 是一个使用 Rust 实现的本地知识库混合检索系统，面向个人学习资料、课程笔记、代码文件和论文 PDF 的统一管理与检索场景。用户可以将资料放入本地 `knowledge_base` 目录，程序会递归扫描该目录下的多级子文件夹，解析 Markdown、文本、Rust 源码、TOML 配置文件和文本型 PDF，并构建可持久化的本地索引。

系统提供 CLI 和 TUI 两种交互方式。CLI 适合快速构建索引、查看统计和执行搜索；TUI 则提供更直观的终端界面，包括首页、帮助页、文件目录页、词频页、统计页、搜索页和检索策略选择页。检索能力方面，系统默认使用 BM25 关键词检索，同时支持接入本地 Qdrant 向量数据库和阿里云百炼 Embedding，实现 BM25 与向量检索结合的 Hybrid 混合检索。

项目名称 CheeseBase 取“芝士库”的谐音，寓意将零散知识沉淀为一个可搜索、可维护、可扩展的个人知识基地。

GitHub 链接：待补充（当前本地仓库尚未配置 GitHub remote，上传后在此处填写仓库地址）。

主要功能包括：

- 支持 `knowledge_base` 正式知识库目录，用户可以自由添加、删除、移动文件和子文件夹。

- 支持 `.md`、`.txt`、`.rs`、`.toml`、`.pdf` 等多种资料格式。

- 支持递归扫描、文本解析、分词、倒排索引构建和 JSON 持久化。

- 支持 BM25 关键词检索、PDF 页码提示、命中片段展示和结果排序。

- 支持 Qdrant 向量索引和 Hybrid 混合检索。

- 支持 TUI 中 `/update` 更新索引、`/strategy` 切换检索方式、鼠标点击打开文件和预览区滚动查看片段。

## 二、小组成员分工

本项目为单人完成，因此不需要进行小组成员分工。

本人独立完成的工作包括：

- 项目选题与需求分析。

- Rust 项目结构设计与模块划分。

- 文件扫描、文本解析、PDF 解析和分词逻辑实现。

- 倒排索引、BM25 检索和搜索结果片段生成。

- Qdrant 向量索引、Embedding 配置和 Hybrid 混合检索实现。

- CLI 命令行交互和 TUI 终端界面实现。

- 测试用例、README、实验报告和演示文档编写。

## 三、项目结构

项目主要目录结构如下：

```plaintext
rust-note-search/
  Cargo.toml
  Cargo.lock
  README.md
  plan.md
  report_demo_guide.md
  writing_and_video_guide.md
  .env.example
  knowledge_base/
    编程语言/
      Rust笔记/
      Java笔记/
    数据库/
      MySQL/
      Redis/
    课程资料/
    论文资料/
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
    integration.rs
```

各目录和文件说明：

- `Cargo.toml`：项目依赖和元信息配置。

- `knowledge_base/`：正式知识库目录，用户后续可以自行维护其中的文件和子文件夹。

- `src/main.rs`：程序入口，负责分发 CLI 命令。

- `src/lib.rs`：库模块导出，便于测试和模块复用。

- `src/cli.rs`：命令行参数解析，定义 `index`、`search`、`stats`、`tui`、`vector-index` 等命令。

- `src/model.rs`：核心数据结构定义，如 `Document`、`Posting`、`InvertedIndex`、`SearchResult`、`SearchStrategy`。

- `src/scanner.rs`：递归扫描目录并识别支持的文件类型。

- `src/parser.rs`：解析 Markdown、文本、代码、TOML 和 PDF 文件。

- `src/index.rs`：构建倒排索引并统计文档信息。

- `src/search.rs`：实现 BM25 检索、排序和片段生成。

- `src/vector.rs`：实现文档 chunk 切分、向量写入和 Qdrant 检索。

- `src/hybrid.rs`：实现 BM25 与向量检索结果融合。

- `src/ui.rs`：实现 TUI 首页、搜索页、帮助页、目录页、统计页和策略选择页。

- `src/storage.rs`：负责索引 JSON 保存与加载。

- `src/config.rs`：读取 `.env` 中的 Embedding 和 Qdrant 配置。

- `src/error.rs`：定义统一错误类型。

- `tests/`：集成测试。

## 四、设计与实现

### 4.1 总体设计

CheeseBase 的整体数据流如下：

```plaintext
knowledge_base/
      |
      v
scanner：递归扫描支持的文件
      |
      v
parser：读取文本、解析 PDF、提取标题
      |
      v
tokenizer：中英文分词
      |
      v
index：构建倒排索引
      |
      +-------------> storage：保存 / 加载 index.json
      |
      +-------------> search：BM25 检索
      |
      +-------------> vector：切分 chunk、写入 Qdrant
      |
      +-------------> hybrid：BM25 + 向量检索融合
      |
      v
CLI / TUI：命令行与终端交互
```

系统采用“本地索引为基础、向量检索为增强”的设计。基础功能只依赖本地文件和 Rust 程序即可运行；当用户配置 Qdrant 和 Embedding API 后，可以进一步启用 Hybrid 混合检索。

### 4.2 文件扫描设计

文件扫描使用 `walkdir` 递归遍历 `knowledge_base` 目录。扫描时只保留支持的文件扩展名，并跳过 `.git`、`target`、隐藏目录等无关内容。这样既避免索引过大，也能减少无意义文件带来的噪声。

支持的文件类型包括：

- Markdown：`.md`

- 文本：`.txt`

- Rust 源码：`.rs`

- TOML 配置：`.toml`

- PDF：`.pdf`

### 4.3 文本解析设计

不同文件类型使用不同解析策略：

- Markdown 文件优先提取第一个一级标题作为文档标题。

- 文本、Rust 源码和 TOML 文件直接读取 UTF-8 正文。

- PDF 文件使用 `pdf-extract` 提取文本内容，并以文件名作为标题。

对于扫描版 PDF，本项目暂不做 OCR，因为 OCR 会显著增加依赖和实现复杂度，也不符合本项目“本地直接运行、复杂度适中”的定位。

### 4.4 倒排索引设计

倒排索引是 BM25 关键词检索的基础。系统将文档正文分词后，记录每个词项在哪些文档中出现、出现次数、出现位置以及 PDF 页码信息。结构可以概括为：

```plaintext
term -> [Posting(doc_id, frequency, positions, pages)]
```

其中：

- `term` 是分词后的词项。

- `doc_id` 是文档编号。

- `frequency` 是词项在该文档中的出现次数。

- `positions` 用于生成命中片段。

- `pages` 用于 PDF 命中页码展示。

这种结构能快速根据查询词找到候选文档，再进行相关度排序。

### 4.5 BM25 检索设计

项目使用 BM25 替代简单词频排序。BM25 能同时考虑词频、词项稀有度和文档长度，比单纯统计关键词出现次数更合理。

本项目固定使用参数：

```plaintext
k1 = 1.5
b = 0.75
```

检索时使用：

- 文档总数 `N`

- 查询词文档频率 `df`

- 当前文档词频 `tf`

- 当前文档长度 `dl`

- 平均文档长度 `avgdl`

BM25 检索保留 `any` 和 `all` 两种模式：

- `any`：文档命中任意查询词即可进入候选。

- `all`：文档必须包含全部查询词才进入候选。

### 4.6 向量检索与 Hybrid 设计

在 Hybrid 模式下，系统会基于已有本地索引构建 Qdrant 向量索引。实现流程为：

1. 从 `index.json` 中读取文档正文。

1. 将长文档切分为约 700 字符的 chunk。

1. chunk 之间保留约 100 字符重叠，减少语义被截断的问题。

1. 调用阿里云百炼 DashScope Embedding API 生成向量。

1. 将向量和 payload 写入本地 Qdrant collection。

Hybrid 检索时，系统分别执行 BM25 检索和 Qdrant 向量检索，然后进行得分归一化和加权融合：

```plaintext
bm25_norm = bm25_score / max_bm25_score
vector_norm = clamp(vector_score, 0.0, 1.0)
hybrid_score = 0.45 * bm25_norm + 0.55 * vector_norm
```

为了避免向量检索返回过多弱相关结果，系统增加了阈值配置：

```plaintext
HYBRID_SCORE_THRESHOLD=0.45
```

只有综合分数达到阈值的结果才会展示。

### 4.7 TUI 设计

TUI 使用 `ratatui` 和 `crossterm` 实现，主要页面包括：

- 首页：展示 CheeseBase 标题、欢迎语、像素小狗、励志短句和索引摘要。

- 帮助页：展示所有 slash 命令。

- 文件页：展示当前知识库目录。

- 词频页：展示高频词。

- 统计页：展示文档数、词项数、token 数和根目录。

- 策略页：用鼠标点击选择 BM25 或 Hybrid。

- 搜索页：输入关键词、查看结果、滚动预览片段、打开文件。

TUI 支持的命令包括：

```plaintext
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

## 五、各模块详细说明

### 5.1 cli.rs：命令行解析模块

`cli.rs` 使用 `clap` 定义命令行参数。主要命令包括：

```bash
cargo run -- index knowledge_base
cargo run -- stats
cargo run -- search ownership --strategy bm25
cargo run -- search 事务 --strategy hybrid
cargo run -- vector-index
cargo run -- tui
```

关键 Rust 特性：

- 使用结构体和枚举描述命令。

- 使用 `clap` 的 derive 宏减少手写解析代码。

- 使用枚举区分不同子命令，使命令分发更类型安全。

### 5.2 model.rs：核心数据模型模块

`model.rs` 定义系统的核心数据结构，包括：

- `Document`：文档元数据和正文。

- `Posting`：倒排索引中的词项记录。

- `InvertedIndex`：完整索引。

- `SearchResult`：搜索结果。

- `SupportedFileType`：支持的文件类型。

- `SearchStrategy`：检索策略。

关键 Rust 特性：

- 使用 `struct` 表达复杂业务实体。

- 使用 `enum` 表达有限状态和类型选择。

- 使用 `serde` 派生序列化和反序列化能力。

### 5.3 scanner.rs：文件扫描模块

该模块负责递归扫描知识库目录，筛选支持的文件类型，并跳过无关目录。

实现思路：

- 使用 `walkdir` 遍历目录。

- 根据扩展名判断文件类型。

- 将扫描结果转换为后续解析阶段可使用的结构。

关键 Rust 特性：

- 使用 `Path` 和 `PathBuf` 处理跨平台文件路径。

- 使用 `Result` 返回扫描错误。

- 使用模式匹配判断文件类型。

### 5.4 parser.rs：文件解析模块

该模块负责将文件转换为可索引的文档内容。

实现思路：

- 文本类文件使用 UTF-8 读取。

- Markdown 文件提取一级标题。

- PDF 文件调用 `pdf-extract` 提取正文。

- 解析失败时返回错误，而不是直接中断程序。

关键 Rust 特性：

- 使用所有权将文件内容移动到 `Document` 中。

- 使用借用读取路径和文件元数据。

- 使用 `Result` 统一处理 IO 和解析错误。

### 5.5 index.rs：索引构建模块

该模块负责构建倒排索引。文件解析适合并行执行，因此项目使用 `rayon` 并发解析多个文件，然后再统一汇总倒排表。

实现思路：

```plaintext
文件列表 -> rayon 并发解析 -> ParsedDocument 列表 -> 汇总生成 InvertedIndex
```

关键 Rust 特性：

- 使用 `rayon` 实现安全并发。

- 使用 `HashMap<String, Vec<Posting>>` 存储倒排表。

- 先并发解析、后单线程合并，避免多个线程同时修改同一倒排表。

### 5.6 search.rs：BM25 检索模块

该模块负责查询分词、候选文档查找、BM25 评分、排序和片段生成。

实现思路：

- 对查询语句分词。

- 根据倒排索引找到候选文档。

- 根据 BM25 公式计算相关度。

- 按分数排序并截断 Top-N。

- 根据命中位置生成上下文片段。

关键 Rust 特性：

- 使用不可变借用读取索引，避免复制大对象。

- 使用迭代器处理候选文档和排序结果。

- 使用泛型辅助函数处理 Top-N 截断。

### 5.7 vector.rs：向量索引模块

该模块负责构建 Qdrant 向量索引和执行向量搜索。

实现思路：

- 将长文档切分为 chunk。

- 调用 Embedding API 生成向量。

- 调用 Qdrant REST API 创建 collection 和写入 points。

- 查询时将 query 转为向量，再请求 Qdrant 返回相似 chunk。

关键 Rust 特性：

- 使用结构体表达 Qdrant payload 和搜索结果。

- 使用 `reqwest` 进行 HTTP 请求。

- 使用 `Result` 处理网络错误、配置错误和返回格式错误。

### 5.8 hybrid.rs：混合检索模块

该模块负责统一 BM25 和 Hybrid 两种检索策略。

实现思路：

- BM25 模式直接调用本地搜索。

- Hybrid 模式同时获取 BM25 结果和向量结果。

- 对两类分数归一化。

- 按固定权重合并。

- 使用阈值过滤低相关结果。

关键 Rust 特性：

- 使用 `SearchStrategy` 枚举进行策略分发。

- 使用 `Option<&AppConfig>` 表达 Hybrid 模式下才需要的配置。

- 使用 `HashMap` 合并同一文档的多路召回结果。

### 5.9 ui.rs：TUI 交互模块

该模块负责终端界面渲染和事件处理。

实现思路：

- 使用 `ratatui` 绘制页面布局。

- 使用 `crossterm` 捕获键盘和鼠标事件。

- 使用 `TuiView` 枚举管理页面状态。

- 搜索页实时刷新结果，预览区支持滚轮滚动。

- 策略页支持鼠标点击选择 BM25 或 Hybrid。

关键 Rust 特性：

- 使用枚举表达页面状态。

- 使用结构体保存 TUI 应用状态。

- 使用事件循环处理输入和页面跳转。

- 使用错误传播避免 UI 操作失败时 panic。

### 5.10 storage.rs、config.rs、error.rs

`storage.rs` 负责索引 JSON 的保存和加载。`config.rs` 负责读取 `.env` 中的 Embedding 与 Qdrant 配置。`error.rs` 使用 `thiserror` 定义统一错误类型 `AppError`。

关键 Rust 特性：

- 使用 `serde_json` 完成结构化数据持久化。

- 使用 `thiserror` 简化错误枚举定义。

- 使用 `Result<T, AppError>` 统一错误返回类型。

### 5.11 Rust 工程实践体现

本项目不仅实现了搜索系统的功能，也尽量按照 Rust 工程实践进行组织。下面按照作业技术要求进行对应说明。

#### 5.11.1 模块化设计

项目采用单 crate 多模块结构组织代码。`Cargo.toml` 定义了当前项目 crate，`src/lib.rs` 统一导出各功能模块，`src/main.rs` 作为二进制入口负责命令分发。由于本项目是单一应用程序，规模适中，因此没有额外拆分 Cargo workspace；如果后续扩展为多个独立子项目，例如 CLI、服务端、共享检索库，可以再升级为 workspace。

`src/lib.rs` 中的模块声明示例：

```rust
pub mod cli;
pub mod config;
pub mod embedding;
pub mod error;
pub mod hybrid;
pub mod index;
pub mod model;
pub mod parser;
pub mod scanner;
pub mod search;
pub mod storage;
pub mod ui;
pub mod vector;
```

这种设计将文件扫描、解析、索引、搜索、存储、TUI、向量检索等功能拆分到不同模块中，避免所有逻辑堆在 `main.rs` 中，提高了可维护性和测试便利性。

#### 5.11.2 错误处理

项目使用 `Result` 作为主要错误处理方式，并定义统一的返回类型 `AppResult<T>`：

```rust
pub type AppResult<T> = Result<T, AppError>;
```

`AppError` 使用枚举统一封装不同错误来源：

```rust
#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("PDF extraction error: {0}")]
    Pdf(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("Qdrant error: {0}")]
    Qdrant(String),

    #[error("index contains no documents")]
    EmptyIndex,
}
```

在业务代码中，文件读取、索引加载、配置读取和向量索引构建等操作都会通过 `?` 向上传播错误：

```rust
fn run() -> AppResult<()> {
    let index = storage::load_index(&index_path)?;
    let config = AppConfig::from_env()?;
    let stats = vector::build_vector_index(&index, &config)?;
    Ok(())
}
```

项目正式业务逻辑中尽量避免大量使用 `unwrap` 或 `expect`。这些方法主要出现在测试代码中，用于让测试失败时给出明确原因，例如 `expect("build")`、`expect("search")`。

#### 5.11.3 Rust 核心特性

第一，项目体现了 ownership 和 borrowing。文档内容、标题和路径由 `Document` 结构体拥有：

```rust
pub struct Document {
    pub id: usize,
    pub path: PathBuf,
    pub title: String,
    pub content: String,
}
```

搜索阶段不会复制整个索引，而是通过借用访问索引数据：

```rust
pub struct SearchEngine<'a, T>
where
    T: Tokenizer,
{
    index: &'a InvertedIndex,
    tokenizer: T,
}
```

这里的生命周期 `'a` 表示 `SearchEngine` 不能比它借用的 `InvertedIndex` 活得更久，从而避免悬垂引用。

第二，项目大量使用 `struct` 和 `enum` 进行建模。代表性结构体包括 `Document`、`Posting`、`InvertedIndex`、`SearchResult`、`AppConfig`、`VectorSearchHit`。代表性枚举包括：

```rust
pub enum SupportedFileType {
    Markdown,
    Text,
    Rust,
    Toml,
    Pdf,
}

pub enum SearchStrategy {
    Bm25,
    Hybrid,
}
```

这些枚举让文件类型和检索策略在编译期受到约束，减少字符串常量带来的错误。

第三，项目使用 trait 抽象分词器：

```rust
pub trait Tokenizer: Clone + Send + Sync + 'static {
    fn tokenize(&self, text: &str) -> Vec<TokenOccurrence>;
}

impl Tokenizer for SimpleTokenizer {
    fn tokenize(&self, text: &str) -> Vec<TokenOccurrence> {
        self.tokenize_internal(text, None)
    }
}
```

这样索引构建和搜索模块只依赖 `Tokenizer` trait，而不强绑定某个具体分词器。后续如果需要接入更专业的中文分词器，只需要实现同一 trait。

第四，项目使用泛型提升复用性。例如索引构建器和搜索引擎都可以接收任意实现了 `Tokenizer` 的类型：

```rust
pub struct IndexBuilder<T>
where
    T: Tokenizer,
{
    tokenizer: T,
}

pub fn parse_file<T>(
    path: impl AsRef<Path>,
    file_type: SupportedFileType,
    tokenizer: &T,
) -> AppResult<ParsedDocument>
where
    T: Tokenizer,
```

存储模块也使用泛型封装 JSON 读写：

```rust
pub fn write_json<T>(path: impl AsRef<Path>, value: &T) -> AppResult<()>
where
    T: Serialize,

pub fn read_json<T>(path: impl AsRef<Path>) -> AppResult<T>
where
    T: DeserializeOwned,
```

#### 5.11.4 并发或异步

本项目索引构建阶段使用 `rayon` 进行并发解析。文件之间相互独立，非常适合并行处理。实现思路是先并发解析每个文件，再统一汇总倒排索引，从而避免多个线程同时修改同一个 `HashMap`。

核心代码示例：

```rust
let parsed = files
    .par_iter()
    .map(|file| parse_file(&file.path, file.file_type, &self.tokenizer))
    .collect::<AppResult<Vec<_>>>()?;
```

这里 `par_iter()` 会并行处理文件列表。由于 `Tokenizer` trait 约束中包含 `Send + Sync`，可以保证分词器能够在线程之间安全使用：

```rust
pub trait Tokenizer: Clone + Send + Sync + 'static {
    fn tokenize(&self, text: &str) -> Vec<TokenOccurrence>;
}
```

项目没有使用 `tokio` 或 `async/await`，原因是核心任务以本地文件扫描、解析和 CPU/IO 混合处理为主，使用 `rayon` 已经能较好满足并发需求。

#### 5.11.5 测试

项目包含单元测试和集成测试，覆盖关键功能。例如：

- tokenizer 中英文分词测试。

- Markdown 标题提取测试。

- 文件扫描过滤测试。

- 倒排索引词频和位置测试。

- BM25 搜索排序测试。

- PDF 文件识别与解析错误测试。

- Hybrid 融合与阈值过滤测试。

- TUI 页面跳转和策略选择测试。

测试代码示例：

```rust
#[test]
fn bm25_ranks_more_relevant_document_first() {
    let index = builder.build(temp.path()).expect("build");
    let engine = SearchEngine::new(&index, SimpleTokenizer::default());
    let results = engine.search("ownership", 10).expect("search");
    assert_eq!(results[0].title, "A");
}
```

集成测试还会创建临时目录，写入测试文件，完成扫描、索引、保存、加载和搜索的完整流程，验证系统端到端行为。

#### 5.11.6 工程规范

项目提交前使用 Rust 标准工具链进行格式化、静态检查和测试：

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

其中：

- `cargo fmt` 保证代码格式统一。

- `cargo clippy` 检查潜在代码质量问题。

- `cargo test` 验证单元测试和集成测试。

此外，项目通过 `.gitignore` 忽略 `.env`、`target/`、日志和生成索引文件，避免将敏感配置和编译产物提交到仓库。真实的 API Key 只应保存在本地 `.env` 中，不能上传到 GitHub。

## 六、运行截图

本章用于在最终提交时插入运行截图。建议至少包含以下截图：

### 6.1 CLI 构建索引截图

运行命令：

```bash
cargo run -- index knowledge_base
```

截图内容应展示程序成功扫描知识库并生成 `index.json`。

### 6.2 CLI 查看统计信息截图

运行命令：

```bash
cargo run -- stats
```

截图内容应展示文档数量、词项数量、token 数量和知识库根目录。

### 6.3 BM25 搜索截图

运行命令：

```bash
cargo run -- search ownership --strategy bm25
```

截图内容应展示搜索结果的分数、文件名、命中词和片段。

### 6.4 Hybrid 搜索截图

运行命令：

```bash
cargo run -- search 事务 --strategy hybrid
```

截图内容应展示 Hybrid 检索结果。演示前需要确保 Qdrant 容器已启动，并且已经执行过：

```bash
cargo run -- vector-index
```

### 6.5 TUI 首页截图

运行命令：

```bash
cargo run -- tui
```

截图内容应展示 CheeseBase 首页、欢迎语、像素小狗、知识库统计和命令提示。

### 6.6 TUI 搜索页截图

在 TUI 中输入：

```plaintext
/select
```

然后输入关键词进行搜索。截图内容应展示结果列表、命中页码、预览片段和高亮效果。

### 6.7 TUI 策略选择页截图

在 TUI 中输入：

```plaintext
/strategy
```

截图内容应展示 `BM25 (default)` 和 `Hybrid` 两种策略说明。

## 七、遇到的问题与解决方法

### 7.1 Cargo 命令无法识别

问题：最初在 Windows 终端中执行 `cargo build` 时出现 `'cargo' 不是内部或外部命令`。

原因：Rust 工具链没有正确加入系统环境变量 `Path`。

解决方法：将 Rust 安装目录中的 Cargo 路径加入环境变量，例如：

```plaintext
C:\Users\<用户名>\.cargo\bin
```

重新打开终端后即可使用 `cargo build`、`cargo run` 和 `cargo test`。

### 7.2 在错误目录运行 Cargo

问题：在父目录执行 `cargo test` 时提示找不到 `Cargo.toml`。

原因：Cargo 命令必须在 Rust 项目根目录下执行。

解决方法：先进入项目目录：

```bash
cd /d D:\AGENT_workspace\rust大作业\rust-note-search
```

再执行 Cargo 命令。

### 7.3 TUI 输入字符重复

问题：TUI 搜索框输入 `rust` 时显示为 `rruusstt`。

原因：终端键盘事件处理时同时处理了不同类型的 key event，导致一次按键被记录两次。

解决方法：在事件处理中只响应有效的按键事件，过滤重复事件，保证每次输入只追加一个字符。

### 7.4 PDF 正文无法检索

问题：向知识库中加入 PDF 后，部分内容无法搜索到。

原因：`pdf-extract` 只能提取文本型 PDF。如果 PDF 是扫描图片，正文并不以文本形式存在。

解决方法：明确项目当前只支持文本型 PDF；对于扫描件 PDF，后续可接入 OCR。

### 7.5 同一文档多处命中展示不足

问题：一个文档中多个位置命中同一关键词时，最初只展示第一个片段。

解决方法：扩展搜索结果结构，保留多个命中片段，并在 TUI Preview 区域支持滚轮滚动，展示同一文档中的多处匹配内容。

### 7.6 Hybrid 检索返回弱相关结果

问题：向量检索语义召回范围较广，可能返回一些不太相关的结果。

解决方法：增加 `HYBRID_SCORE_THRESHOLD` 阈值，只展示综合分数超过阈值的结果，从而提升搜索结果质量。

### 7.7 Qdrant 依赖外部服务

问题：Hybrid 检索依赖本地 Qdrant 服务，如果 Docker 或 Qdrant 未启动，搜索会失败。

解决方法：将 BM25 作为默认策略，保证基础功能完全本地可用；Hybrid 作为增强能力，仅在用户配置并启动 Qdrant 后使用。

## 八、其他需要说明的内容

### 8.1 本地运行说明

本项目基础功能无需 Docker，直接使用 Rust 工具链即可运行：

```bash
cargo build
cargo test
cargo run -- index knowledge_base
cargo run -- search ownership
cargo run -- tui
```

### 8.2 Hybrid 检索配置说明

Hybrid 检索需要额外配置 `.env` 文件。项目提供 `.env.example`，其中包括：

```plaintext
EMBED_MODEL_TYPE=dashscope
EMBED_MODEL_NAME=text-embedding-v3
EMBED_API_KEY=your_dashscope_api_key_here
EMBED_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
EMBED_DIMENSIONS=1024
QDRANT_URL=http://localhost:6333
QDRANT_COLLECTION=cheesebase_chunks
HYBRID_SCORE_THRESHOLD=0.45
```

出于安全考虑，真实 API Key 不应提交到 GitHub。

### 8.3 项目边界

本项目当前不做以下内容：

- 不做 Web 前端。

- 不做扫描版 PDF OCR。

- 不做 RAG 问答生成。

- 不由程序自动启动或停止 Docker 容器。

- 不实现复杂数据库或复杂查询语法。

这些内容可以作为后续扩展方向。

## 九、总结

CheeseBase 是一个围绕个人知识管理场景设计的 Rust 本地知识库混合检索系统。它从文件扫描、文本解析、倒排索引、BM25 排序、PDF 支持、Qdrant 向量数据库集成到 TUI 交互界面，形成了一条较完整的工程链路。

通过本项目，我实践了 Rust 中多个重要特性，包括所有权与借用、结构体建模、枚举状态管理、trait 抽象、泛型辅助函数、Result 错误处理、serde 序列化、rayon 并发处理和模块化工程组织。这些特性并不是孤立展示，而是自然服务于系统功能：所有权保证文本和索引数据安全流转，枚举让文件类型和页面状态更清晰，trait 让分词器具备可替换性，Result 让错误处理更加可靠，并发解析提升了索引构建效率。

相比普通课程项目，CheeseBase 的亮点在于它不仅完成了基础 CLI 功能，还进一步实现了 PDF 检索、BM25 排序、TUI 可视化交互、Qdrant 向量索引和 Hybrid 混合检索，具有较强的实用价值和扩展空间。后续可以继续优化中文分词、接入 OCR、增加文件监听增量索引、开发 Web 前端，或进一步扩展为基于本地知识库的智能问答系统。
