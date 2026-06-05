# RustNoteSearch 写作指导与视频展示指导

本文档用于辅助完成 Rust 大作业的实验报告和展示视频。它不是 README 的重复说明，而是从“如何向老师讲清楚这个项目”的角度整理当前项目的亮点、报告结构、展示脚本和 Rust 特色对应关系。

项目当前定位：`RustNoteSearch` 是一个本地知识库搜索系统。正式知识库目录为 `knowledge_base/`，程序可以递归扫描 Markdown、文本、Rust 源码、TOML 和文本型 PDF，建立倒排索引，使用 BM25 排序，并支持 Qdrant + 阿里云百炼 Embedding 的 Hybrid 混合检索。

## 1. 报告写作总思路

实验报告建议围绕三个问题展开：

1. 为什么做这个项目：本地课程资料、笔记、代码和 PDF 越来越多，普通文件名搜索不能很好地定位正文内容，因此需要一个本地知识库检索工具。
2. 这个项目怎么实现：扫描文件、解析文本、分词、建立倒排索引、持久化 JSON、BM25 检索、TUI 可视化。
3. 这个项目如何体现 Rust：类型系统、所有权与借用、错误处理、trait 抽象、泛型、并发、模块化和测试。

报告不要只写“用了哪些库”，要重点说明“为什么这样设计”和“Rust 在这里解决了什么问题”。例如不要只写“使用 rayon 并发”，还要说明“文件解析天然是相互独立的任务，使用 `par_iter()` 可以并行解析多个文件，同时通过所有权转移把解析结果安全汇总，避免共享可变状态带来的数据竞争”。

## 2. 推荐报告结构

### 2.1 项目背景与选题意义

可以从以下角度写：

- 本地学习资料分散在 Markdown、代码文件、PDF 和文本笔记中。
- 操作系统自带搜索通常偏向文件名或浅层内容搜索，缺少面向知识库的结果排序、命中片段和统计分析。
- 本项目使用 Rust 实现一个不依赖 Docker、不依赖网络服务、可直接在 Windows 本地运行的轻量级知识库搜索工具。
- 项目既有实际使用价值，也适合展示 Rust 在系统工具开发中的优势。

可写示例：

```text
本项目面向学生日常学习资料管理场景，设计并实现了一个本地知识库搜索系统。用户可以将课程笔记、代码片段、数据库资料和论文 PDF 放入 knowledge_base 目录，程序会递归扫描文件并建立倒排索引。搜索时系统使用 BM25 算法对结果进行排序，并展示命中页码和上下文片段。相比普通文件搜索，本系统更关注正文内容、相关度排序和知识库可维护性。
```

### 2.2 需求分析

建议分成功能需求和非功能需求。

功能需求：

- 支持 `knowledge_base/` 多级目录。
- 支持 `.md`、`.txt`、`.rs`、`.toml`、`.pdf`。
- 支持构建索引、保存索引和加载索引。
- 支持关键词搜索、BM25 排序、Qdrant 混合检索和命中片段展示。
- 支持 PDF 页码显示。
- 支持 TUI 首页、命令模式、搜索页面、文件目录、高频词和统计信息。
- 支持在 TUI 中通过 `/strategy` 选择 BM25 或 Hybrid，并通过 `/update` 更新索引。

非功能需求：

- 本地直接运行，不使用 Docker。
- 使用 Cargo 管理依赖。
- 错误处理清晰，不依赖大量 `unwrap()`。
- 代码模块化，便于测试和维护。
- 代码规模控制在课程要求范围内。

### 2.3 系统架构

可以在报告中画一个流程图：

```text
knowledge_base/
      |
      v
scanner 扫描支持的文件
      |
      v
parser 读取文本 / 提取 PDF / 分词
      |
      v
index 建立倒排索引
      |
      v
storage 保存 index.json
      |
      v
search 使用 BM25 检索
      |
      +--> CLI 输出结果
      |
      +--> TUI 可视化搜索、预览、文件跳转
```

写作重点：

- `scanner` 只负责找到候选文件。
- `parser` 负责把不同格式的文件统一转换为可检索文本和 token。
- `index` 负责构建倒排索引。
- `search` 负责 BM25 评分和命中片段。
- `ui` 负责用户交互，不直接处理底层检索细节。

### 2.4 模块划分说明

可以按下表写：

| 模块 | 职责 | 可展示的 Rust 特性 |
| --- | --- | --- |
| `cli.rs` | 定义命令行参数和子命令 | `enum`、derive 宏 |
| `error.rs` | 统一错误类型 | `Result`、`thiserror`、错误传播 |
| `model.rs` | 核心数据结构 | `struct`、`enum`、序列化 |
| `scanner.rs` | 递归扫描目录 | `PathBuf`、错误处理 |
| `parser.rs` | 文本解析、PDF 提取、分词 | `trait`、泛型、借用 |
| `index.rs` | 并发构建倒排索引 | `rayon`、所有权转移 |
| `search.rs` | BM25 搜索和片段生成 | 泛型、集合类型、排序 |
| `storage.rs` | JSON 保存和加载 | 泛型辅助函数、Serde |
| `analysis.rs` | 统计、高频词、文档分析 | 迭代器、排序 |
| `report.rs` | Markdown 报告导出 | 文件 I/O、格式化 |
| `ui.rs` | TUI 首页和搜索界面 | 状态枚举、事件循环 |

### 2.5 核心数据结构

建议重点介绍 `model.rs` 中的几个类型：

- `Document`：保存文档 ID、路径、标题、文件类型、大小、token 数和正文内容。
- `Posting`：倒排表中的一条记录，表示某个词在某个文档中的频次和出现位置。
- `IndexedOccurrence`：记录 token 位置、字符范围和 PDF 页码。
- `InvertedIndex`：包含文档列表、倒排表和索引元信息。
- `SearchResult`：搜索结果，包含得分、命中词、片段和多处命中。

写作重点：

```text
倒排索引的核心思想是从“文档到词”的结构转换为“词到文档”的结构。搜索时不需要遍历所有文档正文，而是先根据查询词在 postings 中定位候选文档，再计算 BM25 得分。
```

### 2.6 索引构建流程

建议结合 `index.rs` 说明：

1. `scan_directory(root)` 找到所有支持的文件。
2. 使用 `rayon` 的 `par_iter()` 并发解析文件。
3. 每个文件被解析为 `ParsedDocument`。
4. 为每个文档分配连续的 `doc_id`。
5. 将 token 按词项聚合，生成 `HashMap<String, Vec<Posting>>`。
6. 统计文档数、词项数、总 token 数并生成 `IndexMetadata`。

Rust 亮点：

- 并发解析阶段没有共享可变索引，而是先并行生成 `Vec<ParsedDocument>`，再单线程汇总，降低并发复杂度。
- `ParsedDocument` 进入汇总阶段时使用所有权转移，避免无意义复制。
- `HashMap` 和 `Vec` 组合适合表达倒排索引结构。

### 2.7 分词与 PDF 支持

建议结合 `parser.rs` 说明：

- 英文、数字、代码标识符按连续 ASCII 字符切分，并统一转小写。
- 中文使用 `jieba-rs`，比最初的简单 bigram 更自然。
- PDF 使用 `pdf-extract` 提取整体文本，同时使用 `lopdf` 辅助逐页提取 token 页码。
- 文本型 PDF 可以检索；扫描件 PDF 不做 OCR，这是当前系统边界。

Rust 亮点：

- `Tokenizer` 是 trait，定义统一分词接口。
- `SimpleTokenizer` 实现该 trait，因此搜索引擎和索引构建器不依赖具体分词器。
- `SimpleTokenizer` 内部用 `Arc<Jieba>` 共享分词引擎，使 tokenizer 可以 `Clone + Send + Sync`，满足并发解析需求。

可写示例：

```text
项目中定义了 Tokenizer trait，将分词行为抽象为统一接口。IndexBuilder 和 SearchEngine 都以泛型参数接收 tokenizer，因此它们不关心具体分词策略。后续如果要接入新的中文分词器或向量检索前的文本切分策略，只需要实现同一个 trait 即可。
```

### 2.8 BM25 检索算法

建议说明为什么使用 BM25：

- 只按词频排序会偏向长文档。
- BM25 同时考虑词频、文档长度和词项区分度。
- 高频但不具区分度的词权重较低，稀有但匹配的关键词权重较高。

报告中可以写出简化公式：

```text
score(D, Q) = sum IDF(q) * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * |D| / avgdl))
```

本项目参数：

- `k1 = 1.5`
- `b = 0.75`

解释：

- `tf` 是词项在文档中的出现次数。
- `|D|` 是文档 token 数。
- `avgdl` 是平均文档长度。
- `IDF` 体现词项在整个知识库中的稀有程度。

### 2.9 TUI 设计

建议介绍当前 TUI 的两个层次：

1. 首页命令模式：
   - 展示 `Cheese base`、欢迎语、小狗像素画、知识库统计。
   - 支持 `/help`、`/files`、`/terms`、`/stats`、`/update`、`/select`。
2. 搜索页面：
   - 实时搜索。
   - Results 列表显示得分、文件名和页码。
   - Preview 展示多个命中片段并高亮关键词。
   - Enter 或鼠标点击打开本地文件。

Rust 亮点：

- 使用 `TuiView` 枚举管理页面状态。
- 事件循环处理键盘和鼠标事件。
- UI 层只调用搜索引擎、索引更新和文件打开函数，逻辑边界清晰。

### 2.10 测试与工程规范

报告中建议列出：

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- index knowledge_base
cargo run -- stats
cargo run -- search ownership
cargo run -- search 事务
cargo run -- tui
```

测试覆盖点：

- 文件扫描和扩展名识别。
- 中英文 tokenizer。
- Markdown 标题提取。
- PDF 文本提取和错误处理。
- 倒排索引词频和位置。
- BM25 排序。
- 同文档多命中片段。
- TUI 输入、页面返回、点击映射、文件打开错误。
- 保存、加载和损坏 JSON 错误处理。

## 3. Rust 特性重点展示清单

这一部分是报告和视频中最应该强调的内容。

### 3.1 所有权与借用

推荐展示位置：

- `index.rs`
- `parser.rs`
- `search.rs`

讲法：

```text
Rust 的所有权机制在索引构建中非常明显。文件解析阶段生成 ParsedDocument，汇总索引时通过 into_iter() 消费这些解析结果，把其中的 path、title、content 和 tokens 移入最终的 Document 与 postings。这样可以减少复制，同时编译器保证不会出现悬垂引用。
```

可以重点说明：

- `PathBuf`、`String`、`Vec<TokenOccurrence>` 都是拥有所有权的数据。
- `parse_file(&candidate.path, candidate.file_type, &tokenizer)` 中路径和 tokenizer 通过借用传入，避免复制。
- 搜索时 `SearchEngine` 持有 `&InvertedIndex`，只读借用索引，不夺取索引所有权。

### 3.2 struct

推荐展示位置：

- `model.rs`

讲法：

```text
项目使用 struct 表达系统中的核心实体，例如 Document 表示文档，Posting 表示倒排表项，InvertedIndex 表示整个索引，SearchResult 表示搜索结果。这些类型让数据结构的含义清晰，也方便 serde 自动序列化为 JSON。
```

重点类型：

- `Document`
- `Posting`
- `IndexedOccurrence`
- `IndexMetadata`
- `InvertedIndex`
- `SearchResult`

### 3.3 enum

推荐展示位置：

- `model.rs`
- `error.rs`
- `ui.rs`
- `cli.rs`

讲法：

```text
enum 用于表达有限状态集合。例如 SupportedFileType 表达支持的文件类型，SearchMode 表达 any/all 搜索模式，TuiView 表达 TUI 当前页面，AppError 表达不同错误来源。相比用字符串或数字标记状态，enum 更安全，匹配时也更清晰。
```

重点类型：

- `SupportedFileType`
- `SearchMode`
- `SearchState`
- `AppError`
- `TuiView`

### 3.4 trait

推荐展示位置：

- `parser.rs`
- `index.rs`
- `search.rs`

讲法：

```text
Tokenizer trait 把分词算法抽象出来，IndexBuilder 和 SearchEngine 使用泛型接收任何实现 Tokenizer 的类型。当前默认实现是 SimpleTokenizer，内部使用 jieba-rs 处理中文。如果后续需要替换分词策略或接入 embedding 前的文本切分，只需要新增实现，不需要重写索引和搜索流程。
```

这是一个非常适合在视频里展示的 Rust 点，因为它能体现抽象能力。

### 3.5 泛型

推荐展示位置：

- `IndexBuilder<T>`
- `SearchEngine<'a, T>`
- `storage` 读写辅助函数

讲法：

```text
IndexBuilder 和 SearchEngine 都以泛型参数 T 接收 tokenizer，并约束 T: Tokenizer。这让索引构建和搜索逻辑与具体分词器解耦，同时编译期可以检查接口是否满足要求。
```

可以强调：

- 泛型不是为了炫技，而是为后续扩展分词器和向量检索打基础。
- trait bound 中的 `Clone + Send + Sync + 'static` 与并发构建有关。

### 3.6 Result 错误处理

推荐展示位置：

- `error.rs`
- `scanner.rs`
- `parser.rs`
- `storage.rs`
- `main.rs`

讲法：

```text
项目定义 AppError 和 AppResult<T>，统一处理 I/O、JSON、目录遍历、PDF 提取、索引版本不兼容和终端错误。多数函数返回 Result，并使用 ? 进行错误传播，避免在业务逻辑中大量 unwrap 或 panic。
```

可以举例：

- 目录不存在返回 `MissingPath`。
- PDF 解析失败返回 `Pdf`。
- 旧版本索引返回 `IncompatibleIndex`。
- 空索引搜索返回 `EmptyIndex`。

### 3.7 并发

推荐展示位置：

- `index.rs`

讲法：

```text
索引构建阶段对多个文件的解析互不依赖，适合并发。项目使用 rayon 的 par_iter() 并发解析候选文件，把每个文件转换成 ParsedDocument。并发阶段只产生独立结果，不共享可变倒排表；随后再统一汇总，兼顾性能和安全性。
```

这里要突出 Rust 的安全并发：

- tokenizer 满足 `Send + Sync`。
- 不使用手动线程锁。
- 避免多个线程同时修改同一个 `HashMap`。

### 3.8 模块化

推荐展示位置：

- `src/` 目录结构

讲法：

```text
项目拆分为 CLI、扫描、解析、索引、搜索、存储、分析、报告和 TUI 多个模块。每个模块职责单一，既便于测试，也便于后续继续加入 Qdrant 向量检索或新的文件类型。
```

### 3.9 第三方 crate 的合理使用

可以说明依赖不是随意堆砌，而是服务于功能：

- `clap`：命令行参数。
- `walkdir`：递归扫描。
- `serde` / `serde_json`：索引持久化。
- `thiserror`：错误类型。
- `rayon`：并发解析。
- `jieba-rs`：中文分词。
- `pdf-extract` / `lopdf`：PDF 正文和页码。
- `ratatui` / `crossterm`：TUI。
- `open`：打开本地文件。
- `tempfile`：测试临时目录。

## 4. 视频展示指导

建议视频控制在 5 分钟左右。如果老师没有严格限制，也尽量控制在 6 分钟以内。视频不要只跑命令，要边跑边解释“这个功能背后的 Rust 设计”。

### 4.1 展示前准备

录制前先执行：

```bash
cd /d D:\AGENT_workspace\rust大作业\rust-note-search
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- index knowledge_base
```

确认：

- `index.json` 的 root 是 `knowledge_base`。
- TUI 首页显示的是 `Cheese base`。
- 搜索 `ownership` 有结果。
- 搜索 `事务` 有结果。
- 如果要展示 PDF，提前准备一个能命中的 PDF 正文关键词。

录制建议：

- 终端字号调大。
- 先关闭旧的 TUI 进程，避免索引文件被旧进程覆盖。
- 不要在视频里临时 debug。
- 尽量使用同一个终端窗口完成演示。

### 4.2 推荐 5 分钟视频脚本

#### 0:00 - 0:30 项目介绍

展示项目目录和 `knowledge_base/`。

建议旁白：

```text
大家好，我的 Rust 大作业是 RustNoteSearch，一个本地知识库搜索系统。它可以扫描本地 knowledge_base 目录中的笔记、代码和 PDF 文件，建立倒排索引，并通过命令行和 TUI 进行搜索。项目不依赖 Docker，也不调用外部搜索 API，可以直接在 Windows 本地运行。
```

展示命令：

```bash
dir
tree knowledge_base /F
```

#### 0:30 - 1:10 构建索引

展示命令：

```bash
cargo run -- index knowledge_base
cargo run -- vector-index
```

旁白重点：

```text
这里程序会递归扫描 knowledge_base 中的多级目录，过滤支持的文件类型，然后使用 rayon 并发解析文件。每个文件会被解析成 ParsedDocument，再汇总成 InvertedIndex。随后 vector-index 会把文档切成片段，调用阿里云百炼 Embedding，并写入本地 Qdrant 的 cheesebase_chunks 集合。
```

可以切到 `src/index.rs`，展示：

- `IndexBuilder<T>`
- `par_iter()`
- `HashMap<String, Vec<Posting>>`

#### 1:10 - 1:50 统计和高频词

展示命令：

```bash
cargo run -- stats
cargo run -- terms
```

旁白重点：

```text
stats 展示当前知识库的文档数、词项数、总 token 数和最大文档。terms 展示高频词，方便观察知识库主题。这里可以看到索引根目录是 knowledge_base，说明当前搜索的是正式知识库，而不是临时 demo 目录。
```

#### 1:50 - 2:40 CLI 搜索

展示命令：

```bash
cargo run -- search ownership --strategy bm25
cargo run -- search 事务 --strategy hybrid
cargo run -- search "rust ownership" --mode all
```

旁白重点：

```text
搜索阶段默认使用 BM25 算法排序，而不是简单词频排序。Hybrid 模式会同时使用 BM25 和 Qdrant 向量检索，并按 0.45 * BM25 + 0.55 * Vector 的公式融合得分。系统还设置了默认 0.45 的入选阈值，低于阈值的候选不会展示，从而减少不相关结果。
```

如果展示 PDF：

```bash
cargo run -- search <PDF正文关键词>
```

旁白：

```text
对于文本型 PDF，系统会提取正文并记录命中页码，因此结果中可以看到对应 page 信息。扫描版 PDF 目前不做 OCR，这是后续可以扩展的方向。
```

#### 2:40 - 3:50 TUI 首页和命令

展示命令：

```bash
cargo run -- tui
```

在 TUI 中依次输入：

```text
/help
/files
/terms
/stats
/strategy
/select
```

旁白重点：

```text
TUI 启动后首先进入 Cheese base 首页，展示知识库摘要和命令提示。这里使用 TuiView 枚举管理不同页面状态，例如 Home、Help、Files、Terms、Stats、Strategy 和 Search。输入 /strategy 后会进入检索方式选择页，用户可以用鼠标点击 BM25(default) 或 Hybrid。
```

进入搜索页后搜索：

```text
ownership
事务
```

展示：

- Results 中的得分、文件名、页码。
- Preview 中的高亮片段。
- 鼠标滚轮滚动 Preview。
- Enter 或鼠标点击打开文件。

#### 3:50 - 4:40 代码亮点展示

快速切换到几个源文件，不要逐行解释，只讲关键点。

推荐顺序：

1. `src/model.rs`
   - `Document`
   - `Posting`
   - `InvertedIndex`
   - `SupportedFileType`
2. `src/parser.rs`
   - `Tokenizer` trait
   - `SimpleTokenizer`
   - PDF 解析
3. `src/error.rs`
   - `AppError`
   - `AppResult<T>`
4. `src/index.rs`
   - `IndexBuilder<T>`
   - `rayon::par_iter`
5. `src/search.rs`
   - BM25 参数
   - `SearchEngine<'a, T>`

旁白示例：

```text
这个项目重点展示了 Rust 的类型系统和工程化能力。model.rs 中用 struct 和 enum 表达核心实体；parser.rs 中用 Tokenizer trait 抽象分词策略；index.rs 中使用泛型和 rayon 并发构建索引；error.rs 中用 Result 和 thiserror 统一错误处理；search.rs 中实现 BM25 排序。整体代码按模块拆分，便于测试和后续扩展。
```

#### 4:40 - 5:00 总结

旁白：

```text
总的来说，RustNoteSearch 实现了本地知识库的扫描、解析、索引、搜索、统计和 TUI 展示。项目能够直接在 Windows 本地运行，支持中文、英文、代码和文本型 PDF。Rust 特色方面，项目体现了所有权与借用、struct、enum、trait、泛型、Result 错误处理、并发和模块化设计。后续可以继续扩展 Qdrant 向量检索、embedding 混合搜索和更强的 PDF/OCR 支持。
```

### 4.3 视频中必须展示的验收命令

至少展示这些：

```bash
cargo run -- index knowledge_base
cargo run -- vector-index
cargo run -- stats
cargo run -- search ownership --strategy bm25
cargo run -- search 事务 --strategy hybrid
cargo run -- tui
```

如果时间允许，再展示：

```bash
cargo test
cargo run -- terms
cargo run -- report
```

### 4.4 TUI 演示路线

推荐路线：

1. 启动 `cargo run -- tui`。
2. 展示 `Cheese base` 首页。
3. 输入 `/help` 展示命令说明。
4. 输入 `/files` 展示知识库目录。
5. 输入 `/terms` 展示高频词。
6. 输入 `/stats` 展示索引统计。
7. 输入 `/strategy`，鼠标点击选择 `BM25 (default)` 或 `Hybrid`。
8. 输入 `/select` 进入搜索。
9. 搜索 `ownership`。
10. 搜索 `事务`。
11. 展示 Preview 滚动。
12. 按 Enter 打开文件。
13. 回到首页或退出。

可以提一句：

```text
如果我手动添加或删除 knowledge_base 中的文件，只需要在 TUI 中输入 /update，就会重新扫描当前知识库并刷新索引。当前策略是 Hybrid 时，/update 还会同步刷新 Qdrant 向量索引。
```

### 4.5 常见问题与规避

#### 问题 1：TUI 首页仍显示 demo_notes

原因通常是旧的 `index.json` 没有重新生成，或者旧 TUI 进程仍在运行。

解决：

```bash
cargo run -- index knowledge_base
```

如果仍不生效，关闭所有 `rust-note-search.exe` 进程后再运行。

#### 问题 2：PDF 搜不到

可能原因：

- PDF 是扫描件，没有可提取文本。
- 搜索词不在正文文本中。
- 索引没有更新。

解决：

```bash
cargo run -- index knowledge_base
```

并确认 PDF 是文本型 PDF。

#### 问题 3：中文搜索效果不是完全符合预期

说明：

- 当前使用 `jieba-rs` 做中文分词。
- 中文检索效果依赖词典和文本质量。
- 后续可以扩展为向量检索或混合检索。

#### 问题 4：展示时间不够

优先展示：

1. 构建索引。
2. CLI 搜索。
3. TUI 搜索。
4. Rust 特性代码点。

不要在视频中花太多时间解释 UI 颜色和像素小狗，那个是加分项，不是核心技术点。

## 5. 实验报告中的 Rust 特色对照表

| 老师可能关注的点 | 本项目对应实现 | 建议展示文件 |
| --- | --- | --- |
| 所有权与借用 | 解析结果移动进索引，搜索引擎只读借用索引 | `index.rs`、`search.rs` |
| struct | 文档、倒排项、索引、搜索结果 | `model.rs` |
| enum | 文件类型、搜索状态、搜索模式、错误类型、TUI 页面 | `model.rs`、`error.rs`、`ui.rs` |
| trait | 分词器抽象 | `parser.rs` |
| 泛型 | `IndexBuilder<T>`、`SearchEngine<'a, T>` | `index.rs`、`search.rs` |
| Result 错误处理 | `AppResult<T>`、`AppError`、`?` 传播 | `error.rs`、`parser.rs`、`storage.rs` |
| 并发 | `rayon::par_iter()` 并发解析文件 | `index.rs` |
| 模块化 | 多模块拆分，每个模块职责明确 | `src/` |
| 文件 I/O | 扫描、读取、保存 JSON、打开文件 | `scanner.rs`、`storage.rs`、`ui.rs` |
| 第三方库使用 | clap、serde、rayon、ratatui、jieba-rs、pdf-extract | `Cargo.toml` |
| 测试 | 单元测试和集成测试 | `tests/`、各模块 `#[cfg(test)]` |

## 6. 可直接写入报告的项目创新点

可以选择写 3 到 5 个：

1. 从简单关键词搜索升级为 BM25 检索，提高排序合理性。
2. 支持正式 `knowledge_base` 多级目录，用户可以自由维护知识库。
3. 支持文本型 PDF 正文检索，并展示命中页码。
4. TUI 提供首页、命令模式、目录查看、高频词、统计和搜索页面。
5. 同一文档内支持多个命中片段，Preview 可以滚动查看。
6. 使用 trait 抽象分词器，为后续接入向量检索或其他分词策略预留扩展点。
7. 使用 rayon 并发解析文件，在保持代码安全性的同时提高索引构建效率。

## 7. 不足与展望

报告最后可以写：

- 当前 PDF 只支持文本型 PDF，不支持扫描件 OCR。
- 中文分词依赖 `jieba-rs`，对专业术语仍可能不够准确。
- 当前索引是全量更新，后续可以做增量索引。
- 当前检索是 BM25 关键词检索，后续可以接入 Qdrant 和阿里云百炼 embedding，实现 BM25 与向量检索的混合检索。
- 当前 TUI 已经支持本地交互，后续可以增加 Web 前端或更复杂的知识库管理功能。

推荐写法：

```text
后续计划将 BM25 关键词检索与 Qdrant 向量检索结合。BM25 适合精确关键词匹配，向量检索适合语义相近但关键词不同的场景。二者混合后可以提升本地知识库对自然语言问题的召回能力。
```

## 8. 最终提交前检查清单

提交前确认：

- `README.md` 中主流程是 `knowledge_base`。
- `writing_and_video_guide.md` 已加入项目。
- `knowledge_base/` 包含多级目录和样例文件。
- `cargo fmt --check` 通过。
- `cargo clippy -- -D warnings` 通过。
- `cargo test` 通过。
- `cargo run -- index knowledge_base` 可以生成索引。
- `cargo run -- vector-index` 可以写入 Qdrant 向量索引。
- `cargo run -- stats` 显示 root 为 `knowledge_base`。
- `cargo run -- search ownership --strategy bm25` 有结果。
- `cargo run -- search 事务 --strategy hybrid` 有结果。
- `cargo run -- tui` 可以进入首页和搜索页。
- TUI 中 `/strategy` 可以用鼠标选择 BM25 或 Hybrid。
- 视频中能清晰展示 Rust 特性，而不只是展示界面。
