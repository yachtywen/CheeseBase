# RustNoteSearch 实验报告与演示视频讲解指南

本文档用于帮助你完成 Rust 大作业的实验报告和演示视频。它的重点不是复述 README，而是告诉你如何把这个项目讲得清楚、有技术亮点，并且突出 Rust 语言特性。

项目当前可以概括为：

> RustNoteSearch 是一个使用 Rust 实现的本地知识库搜索系统。它支持 Markdown、文本、Rust 源码、TOML 和文本型 PDF 文件，能够递归扫描 `knowledge_base` 多级目录，建立倒排索引，使用 BM25 进行关键词检索，并可结合 Qdrant 向量数据库和阿里云百炼 Embedding 实现 Hybrid 混合检索。项目还提供 CLI 和 TUI 两种交互方式，TUI 支持检索策略选择、文件跳转、命中片段预览和知识库更新。

## 1. 报告中的核心定位

报告开头建议直接说明：这个项目不是简单命令行练习，而是一个完整的本地知识库检索工具。

可以这样写：

```text
本项目面向个人学习资料管理场景，实现了一个本地知识库搜索系统。用户可以将课程笔记、代码文件、数据库资料和论文 PDF 放入 knowledge_base 目录，程序会递归扫描文件、解析文本、构建索引，并支持基于 BM25 和向量检索的混合搜索。项目同时提供命令行和终端 TUI 两种交互方式，兼顾工程完整性和演示效果。
```

项目关键词：

- 本地知识库
- 多格式文档解析
- 倒排索引
- BM25 排序
- Qdrant 向量检索
- Hybrid 混合检索
- TUI 交互界面
- Rust 类型系统与安全并发

## 2. 实验报告推荐结构

### 2.1 选题背景

你可以从“资料越来越多、普通文件搜索不够好用”切入。

建议写法：

```text
在课程学习和项目开发过程中，个人资料通常分散在 Markdown 笔记、代码文件、PDF 论文和文本资料中。传统文件搜索通常更关注文件名，难以对正文内容进行结构化检索，也缺少相关度排序、命中片段展示和知识库统计能力。因此，本项目选择实现一个本地知识库搜索系统，帮助用户更高效地查找自己的学习资料。
```

### 2.2 需求分析

建议拆成功能需求和技术需求。

功能需求：

- 支持 `knowledge_base` 多级目录。
- 支持 `.md`、`.txt`、`.rs`、`.toml`、`.pdf`。
- 支持构建索引、保存索引、加载索引。
- 支持关键词搜索和命中片段展示。
- 支持 PDF 正文检索和页码提示。
- 支持 TUI 首页、帮助页、目录页、词频页、统计页和搜索页。
- 支持在 TUI 中选择 `BM25` 或 `Hybrid` 检索策略。
- 支持 `/update` 更新知识库索引。

技术需求：

- 使用 Rust 标准工具链构建。
- 使用 `Result` 做统一错误处理。
- 使用 `struct`、`enum`、`trait`、泛型、模块化设计。
- 使用 `rayon` 并发解析文件。
- 使用 `serde` 持久化 JSON 索引。
- 使用 Qdrant 实现向量检索扩展。

### 2.3 系统架构

报告中可以放下面这个架构图：

```text
knowledge_base/
      |
      v
scanner: 递归扫描支持的文件
      |
      v
parser: 读取文本、提取 PDF、中文分词
      |
      v
index: 构建倒排索引
      |
      +----------------------+
      |                      |
      v                      v
storage: index.json       vector-index: chunk + embedding + Qdrant
      |                      |
      +----------+-----------+
                 |
                 v
search / hybrid: BM25 或混合检索
                 |
        +--------+--------+
        v                 v
      CLI               TUI
```

讲解重点：

- `scanner` 只负责发现文件。
- `parser` 只负责把文件变成文本和 token。
- `index` 只负责构建倒排索引。
- `search` 负责 BM25 检索。
- `vector` 负责向量索引和 Qdrant 查询。
- `hybrid` 负责合并 BM25 和向量检索结果。
- `ui` 负责 TUI 交互，不把所有逻辑塞在一个文件里。

## 3. 重点突出 Rust 特性

这部分是报告和视频中最重要的内容。不要只说“我用了 Rust”，要说明“Rust 在项目中具体体现在哪里”。

### 3.1 所有权与借用

推荐展示文件：

- `src/index.rs`
- `src/parser.rs`
- `src/search.rs`

讲法：

```text
索引构建时，parse_file 会生成 ParsedDocument。随后 build_from_parsed 使用 into_iter() 消费这些解析结果，把 path、title、content 和 tokens 移动到最终的 Document 和 Posting 结构中。这样既减少了不必要的复制，也由 Rust 编译器保证数据不会出现悬垂引用。
```

可以重点讲：

- 解析阶段返回拥有所有权的 `ParsedDocument`。
- 构建索引时通过所有权转移把数据放入 `InvertedIndex`。
- 搜索阶段 `SearchEngine` 只借用 `&InvertedIndex`，不会修改索引。
- Rust 编译器在编译期保证引用安全。

### 3.2 struct

推荐展示文件：

- `src/model.rs`

重点类型：

- `Document`：表示一个知识库文档。
- `Posting`：表示一个词在某个文档中的出现情况。
- `IndexedOccurrence`：表示词出现的位置、字符范围和 PDF 页码。
- `InvertedIndex`：表示整个倒排索引。
- `SearchResult`：表示搜索结果。

讲法：

```text
项目使用 struct 表达搜索系统中的核心实体。相比直接使用松散的 HashMap 或 JSON，struct 可以让数据含义更清晰，也便于使用 serde 进行序列化和反序列化。
```

### 3.3 enum

推荐展示文件：

- `src/model.rs`
- `src/error.rs`
- `src/ui.rs`
- `src/cli.rs`

重点类型：

- `SupportedFileType`
- `SearchMode`
- `SearchStrategy`
- `SearchState`
- `AppError`
- `TuiView`

讲法：

```text
enum 用于表达有限状态集合。例如 SearchStrategy 只有 BM25 和 Hybrid 两种检索策略，TuiView 表示 TUI 当前页面，AppError 表示各种错误来源。这样比用字符串判断更安全，编译器也能帮助检查 match 是否覆盖完整。
```

### 3.4 trait

推荐展示文件：

- `src/parser.rs`

重点代码：

- `Tokenizer` trait
- `SimpleTokenizer`

讲法：

```text
项目定义了 Tokenizer trait，把分词行为抽象出来。IndexBuilder 和 SearchEngine 并不依赖具体的分词器实现，只要求传入的类型实现 Tokenizer。这体现了 Rust 通过 trait 实现抽象和解耦的能力。
```

可以补充：

```text
如果以后要替换中文分词器，或者为向量检索单独设计新的文本切分策略，只需要实现同一个 trait，而不需要重写整个搜索流程。
```

### 3.5 泛型

推荐展示文件：

- `src/index.rs`
- `src/search.rs`
- `src/hybrid.rs`

讲法：

```text
IndexBuilder<T> 和 SearchEngine<'a, T> 使用泛型参数接收 tokenizer，并通过 trait bound 限制 T 必须实现 Tokenizer。这种设计让索引构建和搜索逻辑不绑定某个具体分词器，提高了扩展性。
```

重点说明：

- `T: Tokenizer`
- `SearchEngine<'a, T>` 中的生命周期 `'a`
- 泛型让编译期类型检查更严格。

### 3.6 Result 错误处理

推荐展示文件：

- `src/error.rs`
- `src/storage.rs`
- `src/parser.rs`
- `src/vector.rs`

讲法：

```text
项目没有依赖大量 unwrap 或 panic，而是定义 AppError 和 AppResult<T>，统一处理 I/O、JSON、PDF、配置、HTTP、Embedding 和 Qdrant 错误。函数通过 ? 传播错误，最终在 main 中统一输出。
```

可展示的错误类型：

- `Io`
- `Json`
- `Pdf`
- `Config`
- `Embedding`
- `Qdrant`
- `MissingPath`
- `IncompatibleIndex`

### 3.7 并发

推荐展示文件：

- `src/index.rs`

讲法：

```text
文件解析是天然可以并行的任务，因为每个文件之间没有依赖。项目使用 rayon 的 par_iter() 并发解析多个文件，得到 ParsedDocument 列表后再统一汇总为倒排索引。这样既提升了索引构建效率，又避免多个线程同时修改同一个 HashMap 带来的复杂同步问题。
```

这里非常适合突出 Rust：

- 并发解析不共享可变索引。
- tokenizer 满足 `Clone + Send + Sync`。
- 编译器帮助保证线程安全。

### 3.8 模块化

推荐展示 `src/` 目录结构：

```text
cli.rs       命令行参数
config.rs    环境变量和配置
error.rs     统一错误类型
scanner.rs   文件扫描
parser.rs    文本解析和分词
index.rs     倒排索引构建
search.rs    BM25 检索
vector.rs    Qdrant 向量索引
hybrid.rs    混合检索排序
storage.rs   JSON 持久化
analysis.rs  统计分析
report.rs    Markdown 报告导出
ui.rs        TUI 交互界面
```

讲法：

```text
项目没有把所有代码写在 main.rs 中，而是按照职责拆分为多个模块。这样不仅满足课程对模块化的要求，也让后续增加 Qdrant 混合检索时可以只新增 config、embedding、vector 和 hybrid 模块，而不破坏原有 BM25 功能。
```

## 4. 项目脱颖而出的技术特点

这一部分可以在报告的“创新点”或“项目亮点”中重点写。

### 4.1 不是简单 CRUD，而是完整检索系统

很多课程项目可能是管理系统、小游戏或简单命令行工具。本项目的特点是实现了搜索系统的核心流程：

- 文档扫描
- 文本解析
- 分词
- 倒排索引
- BM25 排序
- 向量检索
- 混合排序
- TUI 可视化

可以这样写：

```text
相比普通增删改查项目，本项目更接近一个小型搜索引擎。它不仅管理文件，还对正文内容进行索引和排序，并支持关键词检索与语义检索的结合。
```

### 4.2 BM25 + Qdrant Hybrid 混合检索

这是非常强的亮点。

讲解方式：

```text
BM25 擅长精确关键词匹配，例如代码符号、专业术语和文件名；向量检索擅长语义相近的内容召回，例如用户输入的问题和原文措辞不完全一致时。项目将两者结合，使用固定公式融合得分：
```

```text
hybrid_score = 0.45 * bm25_norm + 0.55 * vector_norm
```

同时项目增加了入选阈值：

```text
HYBRID_SCORE_THRESHOLD=0.45
```

报告中可以强调：

- BM25 保证关键词精确性。
- Qdrant 提供语义召回能力。
- 阈值过滤减少不相关结果。
- 用户可以在 TUI 中选择检索策略。

### 4.3 多格式知识库

项目不是只搜 `.txt`，而是支持：

- Markdown
- TXT
- Rust 源码
- TOML
- 文本型 PDF

PDF 还支持页码提示，这很适合展示。

讲法：

```text
系统支持文本型 PDF 检索，并在结果中展示命中页码。对于课程资料和论文阅读场景，这比只检索 Markdown 更贴近真实使用需求。
```

### 4.4 TUI 交互体验

项目不仅有 CLI，还有 TUI。

TUI 亮点：

- Cheese base 首页
- 像素小狗
- 中文命令说明
- `/help`
- `/files`
- `/terms`
- `/stats`
- `/strategy`
- `/update`
- 搜索结果点击打开文件
- Preview 滚动查看多个命中片段

讲法：

```text
TUI 使用 ratatui 和 crossterm 实现。它不是简单打印结果，而是提供了首页、命令页、目录页、策略选择页和搜索页。用户可以通过鼠标选择检索策略，也可以点击搜索结果打开本地文件。
```

### 4.5 可维护的知识库目录

正式目录是：

```text
knowledge_base/
```

用户可以自由添加、删除、移动文件和子文件夹，然后通过：

```bash
cargo run -- index knowledge_base
```

或在 TUI 中输入：

```text
/update
```

刷新索引。

这说明项目不是一次性 demo，而是可以持续维护的本地工具。

### 4.6 安全和工程规范

可以写：

- API Key 只放在 `.env` 中。
- `.env` 被 `.gitignore` 忽略。
- `.env.example` 只提供占位符。
- 项目使用 `cargo fmt`、`cargo clippy`、`cargo test` 验证。
- 单元测试和集成测试覆盖核心逻辑。

## 5. 演示视频推荐流程

建议视频控制在 5 到 7 分钟。不要只跑命令，要边演示边解释 Rust 技术点。

### 5.1 开场 30 秒

展示项目目录：

```bash
dir
tree knowledge_base /F
```

讲：

```text
这是我的 Rust 大作业 RustNoteSearch，一个本地知识库搜索系统。它可以扫描 knowledge_base 目录中的笔记、代码和 PDF，建立本地索引，并支持 BM25 与 Qdrant 向量数据库的混合检索。
```

### 5.2 构建本地索引

运行：

```bash
cargo run -- index knowledge_base
```

讲：

```text
这一步会递归扫描知识库目录，解析支持的文件类型，并使用 rayon 并发解析文件。最终生成本地 JSON 倒排索引。
```

可以切到 `src/index.rs` 展示：

- `IndexBuilder<T>`
- `par_iter()`
- `HashMap<String, Vec<Posting>>`

### 5.3 构建向量索引

如果 Qdrant 和 `.env` 已经配置好，运行：

```bash
cargo run -- vector-index
```

讲：

```text
这一步会把文档正文切分成片段，调用阿里云百炼 Embedding 生成向量，并写入本地 Qdrant 的 cheesebase_chunks 集合。
```

注意：

- 视频中不要打开 `.env`。
- 不要展示真实 API Key。

### 5.4 展示 BM25 搜索

运行：

```bash
cargo run -- search ownership --strategy bm25
cargo run -- search 事务 --strategy bm25
```

讲：

```text
BM25 检索适合精确关键词匹配。它会考虑词频、文档长度和词项区分度，比简单词频排序更合理。
```

### 5.5 展示 Hybrid 搜索

运行：

```bash
cargo run -- search 事务 --strategy hybrid
```

讲：

```text
Hybrid 检索会合并 BM25 和向量检索结果。BM25 负责关键词精确匹配，Qdrant 向量检索负责语义召回，最终用固定公式融合得分，并通过阈值过滤掉低相关结果。
```

可以强调：

```text
hybrid_score = 0.45 * bm25_norm + 0.55 * vector_norm
```

以及：

```text
HYBRID_SCORE_THRESHOLD=0.45
```

### 5.6 展示 TUI

运行：

```bash
cargo run -- tui
```

演示顺序：

1. 展示 Cheese base 首页。
2. 输入 `/help`。
3. 输入 `/files` 展示知识库目录。
4. 输入 `/terms` 展示高频词。
5. 输入 `/stats` 展示统计。
6. 输入 `/strategy`。
7. 鼠标点击 `BM25 (default)`。
8. 再输入 `/strategy`。
9. 鼠标点击 `Hybrid`。
10. 输入 `/select` 进入搜索。
11. 搜索 `ownership` 或 `事务`。
12. 展示 Preview 滚动。
13. 点击结果或按 Enter 打开文件。

讲：

```text
TUI 使用 TuiView 枚举管理不同页面状态，使用 crossterm 处理键盘和鼠标事件。这里可以通过鼠标选择检索策略，并且后续搜索会按照选择的策略执行。
```

### 5.7 展示 Rust 代码亮点

建议只展示 4 到 5 个文件，不要逐行讲太久。

推荐顺序：

1. `src/model.rs`
   - `Document`
   - `Posting`
   - `InvertedIndex`
   - `SearchStrategy`
2. `src/parser.rs`
   - `Tokenizer` trait
   - `SimpleTokenizer`
3. `src/index.rs`
   - `IndexBuilder<T>`
   - `par_iter()`
4. `src/error.rs`
   - `AppError`
   - `AppResult<T>`
5. `src/hybrid.rs`
   - Hybrid 得分融合
   - 阈值过滤

代码讲解话术：

```text
这个项目重点体现了 Rust 的类型系统和工程能力。struct 用来表示文档、倒排项和搜索结果；enum 用来表示文件类型、错误类型和 TUI 页面状态；trait 用来抽象分词器；泛型让索引构建和搜索逻辑不绑定具体分词器；Result 用来统一错误处理；rayon 用来实现安全并发解析。
```

## 6. 报告中的 Rust 特性对照表

| Rust 特性 | 项目体现 | 推荐展示文件 |
| --- | --- | --- |
| 所有权与借用 | 解析结果移动进索引，搜索引擎只读借用索引 | `index.rs`、`search.rs` |
| struct | 文档、倒排项、索引、搜索结果 | `model.rs` |
| enum | 文件类型、检索策略、错误类型、TUI 页面状态 | `model.rs`、`error.rs`、`ui.rs` |
| trait | `Tokenizer` 抽象分词策略 | `parser.rs` |
| 泛型 | `IndexBuilder<T>`、`SearchEngine<'a, T>` | `index.rs`、`search.rs` |
| Result | `AppResult<T>` 和 `AppError` 统一错误处理 | `error.rs` |
| 并发 | `rayon::par_iter()` 并发解析文件 | `index.rs` |
| 模块化 | 多模块分层设计 | `src/` |
| serde | JSON 索引持久化 | `storage.rs`、`model.rs` |
| HTTP 调用 | DashScope Embedding 和 Qdrant REST API | `embedding.rs`、`vector.rs` |

## 7. 技术亮点总结段落

报告中可以直接使用下面这段：

```text
本项目的技术亮点在于，它不是简单的文件管理工具，而是实现了一个小型本地搜索系统。系统首先通过 scanner、parser 和 index 模块构建倒排索引，再使用 BM25 算法完成关键词相关度排序。在此基础上，项目进一步接入 Qdrant 向量数据库和阿里云百炼 Embedding，实现了 BM25 与向量检索的 Hybrid 混合检索。Hybrid 检索既保留了关键词匹配的精确性，又具备语义召回能力，并通过得分阈值过滤低相关结果。与此同时，项目提供了 TUI 策略选择页和搜索结果可视化，使系统具有较好的交互体验和演示效果。
```

Rust 特性总结段落：

```text
在 Rust 特性方面，项目使用 struct 表达核心数据结构，使用 enum 表达文件类型、检索策略、错误类型和 TUI 状态，使用 trait 抽象分词器，使用泛型解耦索引构建和搜索逻辑，使用 Result 统一处理 I/O、JSON、PDF、HTTP 和 Qdrant 错误，并使用 rayon 实现安全并发解析。项目按模块拆分，具有较好的可维护性和扩展性。
```

## 8. 常见答辩问题准备

### 问：为什么不用数据库，选择 JSON 保存索引？

答：

```text
项目定位是本地轻量级知识库工具，JSON 足够直观，便于调试和展示，也降低了部署复杂度。对于课程大作业来说，JSON 持久化可以更清楚地展示索引结构。后续如果知识库规模更大，可以再替换为数据库或增量索引方案。
```

### 问：为什么使用 BM25？

答：

```text
BM25 是经典信息检索排序算法，它不只是统计词频，还考虑文档长度和词项区分度。相比简单词频排序，BM25 对长文档和常见词有更好的归一化处理。
```

### 问：为什么还要加向量检索？

答：

```text
BM25 依赖关键词重合，如果用户输入的表达和原文不同，可能召回不足。向量检索可以根据语义相似度召回内容。因此我使用 Qdrant 存储片段向量，再和 BM25 得分融合，实现 Hybrid 检索。
```

### 问：为什么设置 Hybrid 阈值？

答：

```text
向量检索有时会返回语义上较弱的候选，如果全部展示，会影响用户体验。因此我设置 HYBRID_SCORE_THRESHOLD，只有最终混合得分超过阈值的结果才会展示。这样可以减少不相关结果。
```

### 问：为什么用 Rust 实现？

答：

```text
Rust 适合实现命令行工具和本地系统工具。它的所有权机制可以减少内存错误，Result 错误处理适合文件解析和外部服务调用，trait 和泛型适合抽象分词器和搜索策略，rayon 可以方便地实现安全并发。
```

### 问：项目还有什么不足？

答：

```text
当前 PDF 只支持文本型 PDF，不支持 OCR；向量检索依赖本地 Qdrant 和外部 Embedding API；索引更新目前是全量更新，不是增量更新。后续可以继续优化为文件变更监听、增量索引、本地 embedding 模型和更完整的 RAG 问答系统。
```

## 9. 最终演示命令清单

基础验收：

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- index knowledge_base
cargo run -- stats
cargo run -- search ownership --strategy bm25
cargo run -- tui
```

Hybrid 验收：

```bash
cargo run -- vector-index
cargo run -- search 事务 --strategy hybrid
cargo run -- search ownership --strategy hybrid
```

TUI 演示：

```text
/help
/files
/terms
/stats
/strategy
/select
/update
```

注意：

- 演示 Hybrid 前确保 Docker Desktop 和 Qdrant 已启动。
- 不要在视频中展示 `.env` 和真实 API Key。
- 如果 Hybrid 结果太多，可以调高 `HYBRID_SCORE_THRESHOLD`。
- 如果 Hybrid 结果太少，可以调低 `HYBRID_SCORE_THRESHOLD`。

## 10. 一句话总结

视频最后可以这样收尾：

```text
RustNoteSearch 是一个用 Rust 实现的本地知识库搜索系统，它完整覆盖了文件扫描、文本解析、倒排索引、BM25 排序、Qdrant 向量检索、Hybrid 混合排序和 TUI 可视化交互。项目不仅功能完整，而且充分体现了 Rust 的所有权、类型系统、trait、泛型、Result 错误处理、安全并发和模块化工程能力。
```
