use std::collections::BTreeMap;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use unicode_width::UnicodeWidthChar;

use crate::analysis;
use crate::error::AppResult;
use crate::index::IndexBuilder;
use crate::model::{InvertedIndex, SearchResult, SearchState};
use crate::parser::Tokenizer;
use crate::search::SearchEngine;
use crate::storage;

const TUI_LIMIT: usize = 25;
const TERMS_LIMIT: usize = 20;
const DEFAULT_KNOWLEDGE_BASE: &str = "knowledge_base";
const LEGACY_DEMO_NOTES: &str = "demo_notes";

pub fn run_tui<T>(index: InvertedIndex, index_path: PathBuf, tokenizer: T) -> AppResult<()>
where
    T: Tokenizer,
{
    let mut terminal = TerminalGuard::enter()?;
    let mut app = TuiApp::new(index, index_path, tokenizer);

    loop {
        terminal.draw(|frame| app.draw(frame))?;
        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    if app.handle_key(key)? {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse)?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn enter() -> AppResult<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn draw<F>(&mut self, f: F) -> AppResult<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiView {
    Home,
    Search,
    Help,
    Files,
    Terms,
    Stats,
}

struct TuiApp<T>
where
    T: Tokenizer,
{
    index: InvertedIndex,
    index_path: PathBuf,
    tokenizer: T,
    view: TuiView,
    view_history: Vec<TuiView>,
    input: InputLine,
    last_search_query: String,
    results: Vec<SearchResult>,
    selected: usize,
    state: SearchState,
    status: String,
    results_area: Option<Rect>,
    content_area: Option<Rect>,
    content_scroll: usize,
}

impl<T> TuiApp<T>
where
    T: Tokenizer,
{
    fn new(index: InvertedIndex, index_path: PathBuf, tokenizer: T) -> Self {
        Self {
            index,
            index_path,
            tokenizer,
            view: TuiView::Home,
            view_history: Vec::new(),
            input: InputLine::default(),
            last_search_query: String::new(),
            results: Vec::new(),
            selected: 0,
            state: SearchState::Empty,
            status: "输入 /help 查看命令，输入 /select 进入搜索。".to_string(),
            results_area: None,
            content_area: None,
            content_scroll: 0,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        match key.code {
            KeyCode::Esc => return Ok(true),
            KeyCode::Char('q') if self.input.is_empty() => return Ok(true),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.move_to_start();
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.move_to_end();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
                self.after_input_change()?;
            }
            KeyCode::Char(ch) => {
                self.input.insert(ch);
                self.after_input_change()?;
            }
            KeyCode::Backspace if self.input.is_empty() => {
                self.go_back()?;
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.after_input_change()?;
            }
            KeyCode::Delete => {
                self.input.delete();
                self.after_input_change()?;
            }
            KeyCode::Enter => {
                if self.input.text().trim_start().starts_with('/') {
                    return self.execute_command();
                }
                if self.view == TuiView::Search {
                    self.open_selected_result();
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
                self.go_back()?;
            }
            KeyCode::Left => self.input.move_left(),
            KeyCode::Right => self.input.move_right(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Home => self.move_home(),
            KeyCode::End => self.move_end(),
            _ => {}
        }

        Ok(false)
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> AppResult<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if self.view == TuiView::Search => {
                let Some(index) = self.result_index_at(mouse.column, mouse.row) else {
                    return Ok(());
                };

                self.selected = index;
                self.content_scroll = 0;
                self.open_selected_result();
            }
            MouseEventKind::ScrollDown => {
                if self.is_in_content(mouse.column, mouse.row) {
                    self.scroll_content(3);
                }
            }
            MouseEventKind::ScrollUp => {
                if self.is_in_content(mouse.column, mouse.row) {
                    self.content_scroll = self.content_scroll.saturating_sub(3);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn after_input_change(&mut self) -> AppResult<()> {
        if self.view == TuiView::Search && !self.input.text().trim_start().starts_with('/') {
            self.last_search_query = self.input.text().to_string();
            self.refresh_search()?;
        }
        Ok(())
    }

    fn execute_command(&mut self) -> AppResult<bool> {
        let command = self.input.text().trim().to_ascii_lowercase();
        self.input.clear();

        match command.as_str() {
            "/help" => self.switch_view(TuiView::Help, "正在展示 TUI 命令说明。"),
            "/select" => {
                self.switch_view(TuiView::Search, "已进入搜索模式，直接输入关键词开始搜索。");
                self.input.set_text(&self.last_search_query);
                self.refresh_search()?;
            }
            "/files" => self.switch_view(TuiView::Files, "正在展示当前索引中的知识库目录。"),
            "/terms" => self.switch_view(TuiView::Terms, "正在展示当前索引的高频词。"),
            "/stats" => self.switch_view(TuiView::Stats, "正在展示索引统计信息。"),
            "/home" => self.switch_view(
                TuiView::Home,
                "输入 /help 查看命令，输入 /select 进入搜索。",
            ),
            "/clear" => {
                self.input.clear();
                self.last_search_query.clear();
                self.results.clear();
                self.selected = 0;
                self.state = SearchState::Empty;
                self.content_scroll = 0;
                self.status = "搜索内容已清空。".to_string();
            }
            "/update" => self.update_index()?,
            "/quit" | "/exit" => return Ok(true),
            "" => {}
            _ => {
                self.status = format!("未知命令：{command}。输入 /help 查看可用命令。");
            }
        }

        Ok(false)
    }

    fn switch_view(&mut self, view: TuiView, status: impl Into<String>) {
        if self.view != view {
            self.view_history.push(self.view);
        }
        self.set_view(view, status);
    }

    fn set_view(&mut self, view: TuiView, status: impl Into<String>) {
        self.view = view;
        self.content_scroll = 0;
        self.status = status.into();
    }

    fn go_back(&mut self) -> AppResult<()> {
        let Some(previous_view) = self.view_history.pop() else {
            self.status = "已经在第一个页面，无法继续返回。".to_string();
            return Ok(());
        };

        self.set_view(previous_view, format!("已返回到{}。", self.view_title()));
        if self.view == TuiView::Search {
            self.input.set_text(&self.last_search_query);
            self.refresh_search()?;
        } else {
            self.input.clear();
        }
        Ok(())
    }

    fn update_index(&mut self) -> AppResult<()> {
        let root = preferred_update_root(&self.index.metadata.root);
        if !root.exists() {
            self.status = format!("更新失败：知识库根目录不存在：{}", root.display());
            return Ok(());
        }

        let previous_index = self.index.clone();
        let builder = IndexBuilder::new(self.tokenizer.clone());
        match builder.build(&root).and_then(|new_index| {
            storage::save_index(&self.index_path, &new_index)?;
            Ok(new_index)
        }) {
            Ok(new_index) => {
                self.index = new_index;
                self.status = format!(
                    "索引已更新：{} 个文档，{} 个词项。知识库根目录：{}",
                    self.index.metadata.document_count,
                    self.index.metadata.term_count,
                    self.index.metadata.root.display()
                );
                if self.view == TuiView::Search {
                    self.input.set_text(&self.last_search_query);
                    self.refresh_search()?;
                }
            }
            Err(err) => {
                self.index = previous_index;
                self.status = format!("更新失败：{err}");
            }
        }
        Ok(())
    }

    fn refresh_search(&mut self) -> AppResult<()> {
        let query = self.input.text().trim();
        if query.is_empty() {
            self.results.clear();
            self.selected = 0;
            self.content_scroll = 0;
            self.state = SearchState::Empty;
            return Ok(());
        }

        let engine = SearchEngine::new(&self.index, self.tokenizer.clone());
        self.results = engine.search(query, TUI_LIMIT)?;
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
        self.content_scroll = 0;
        self.state = if self.results.is_empty() {
            SearchState::NoResults
        } else {
            SearchState::HasQuery
        };
        if self.results.is_empty() {
            self.status = "没有匹配结果。请修改关键词，或输入 /help 查看命令。".to_string();
        } else {
            self.status = format!(
                "找到 {} 条结果。按回车或点击结果可打开文件。",
                self.results.len()
            );
        }
        Ok(())
    }

    fn open_selected_result(&mut self) {
        let Some(result) = self.results.get(self.selected) else {
            self.status = "当前没有选中的搜索结果。".to_string();
            return;
        };

        match open_result_path(&result.path) {
            Ok(()) => {
                self.status = format!("已打开 {}", result_file_name(result));
            }
            Err(err) => {
                self.status = format!("打开文件失败：{err}");
            }
        }
    }

    fn move_up(&mut self) {
        if self.view == TuiView::Search {
            let next = self.selected.saturating_sub(1);
            if next != self.selected {
                self.selected = next;
                self.content_scroll = 0;
            }
        } else {
            self.content_scroll = self.content_scroll.saturating_sub(1);
        }
    }

    fn move_down(&mut self) {
        if self.view == TuiView::Search {
            if !self.results.is_empty() {
                let next = (self.selected + 1).min(self.results.len() - 1);
                if next != self.selected {
                    self.selected = next;
                    self.content_scroll = 0;
                }
            }
        } else {
            self.scroll_content(1);
        }
    }

    fn move_home(&mut self) {
        if self.view == TuiView::Search {
            self.selected = 0;
        }
        self.content_scroll = 0;
    }

    fn move_end(&mut self) {
        if self.view == TuiView::Search {
            if !self.results.is_empty() {
                self.selected = self.results.len() - 1;
            }
            self.content_scroll = 0;
        } else {
            let Some(area) = self.content_area else {
                return;
            };
            self.content_scroll = self.max_scroll_for_area(area);
        }
    }

    fn result_index_at(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.results_area?;
        result_index_at(area, column, row, self.results.len())
    }

    fn is_in_content(&self, column: u16, row: u16) -> bool {
        self.content_area
            .is_some_and(|area| point_in_inner_area(area, column, row))
    }

    fn scroll_content(&mut self, amount: usize) {
        let Some(area) = self.content_area else {
            return;
        };
        let max_scroll = self.max_scroll_for_area(area);
        self.content_scroll = (self.content_scroll + amount).min(max_scroll);
    }

    fn max_scroll_for_area(&self, area: Rect) -> usize {
        let visible_rows = area.height.saturating_sub(2) as usize;
        let content_rows = self.content_lines(area).len();
        content_rows.saturating_sub(visible_rows)
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(2),
            ])
            .split(area);

        self.draw_input(frame, chunks[0]);

        if self.view == TuiView::Search {
            self.draw_search(frame, chunks[1]);
        } else {
            self.draw_page(frame, chunks[1]);
        }

        let footer = Paragraph::new(format!(
            "按键：输入命令或关键词 | 回车执行/打开 | 上下键选择或滚动 | 鼠标滚轮滚动 | Esc 退出 | {}",
            self.status
        ))
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, chunks[2]);
    }

    fn draw_input(&self, frame: &mut ratatui::Frame, area: Rect) {
        let title = match self.view {
            TuiView::Search => "搜索或命令",
            _ => "命令",
        };
        let input = Paragraph::new(self.input.text())
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(input, area);
        let cursor_x = area.x
            + 1
            + self
                .input
                .prefix_width()
                .min(area.width.saturating_sub(2) as usize) as u16;
        frame.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }

    fn draw_search(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)])
            .split(area);

        let mut list_state = ListState::default();
        if !self.results.is_empty() {
            list_state.select(Some(self.selected));
        }

        let list_items = self
            .results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let file_name = result
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_else(|| result.path.to_string_lossy());
                let page_summary = result_page_summary(result);
                let text = format!(
                    "{:>2}. {:.2}  {}  {}",
                    idx + 1,
                    result.score,
                    file_name,
                    page_summary
                );
                ListItem::new(Line::from(text))
            })
            .collect::<Vec<_>>();

        let list_title = match self.state {
            SearchState::Empty => "搜索结果 - 请输入关键词",
            SearchState::HasQuery => "搜索结果",
            SearchState::NoResults => "搜索结果 - 无匹配",
        };
        let list = List::new(list_items)
            .block(Block::default().borders(Borders::ALL).title(list_title))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        self.results_area = Some(chunks[0]);
        frame.render_stateful_widget(list, chunks[0], &mut list_state);

        self.content_area = Some(chunks[1]);
        let preview_lines = self.search_preview_lines(chunks[1]);
        let visible_rows = chunks[1].height.saturating_sub(2) as usize;
        let max_scroll = preview_lines.len().saturating_sub(visible_rows);
        self.content_scroll = self.content_scroll.min(max_scroll);

        let preview = Paragraph::new(preview_lines)
            .block(Block::default().borders(Borders::ALL).title("命中预览"))
            .scroll((self.content_scroll as u16, 0));
        frame.render_widget(preview, chunks[1]);
    }

    fn draw_page(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        self.results_area = None;
        self.content_area = Some(area);
        let lines = self.content_lines(area);
        let visible_rows = area.height.saturating_sub(2) as usize;
        let max_scroll = lines.len().saturating_sub(visible_rows);
        self.content_scroll = self.content_scroll.min(max_scroll);

        let page = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.view_title()),
            )
            .scroll((self.content_scroll as u16, 0));
        frame.render_widget(page, area);
    }

    fn view_title(&self) -> &'static str {
        match self.view {
            TuiView::Home => "首页",
            TuiView::Search => "搜索",
            TuiView::Help => "帮助",
            TuiView::Files => "知识库目录",
            TuiView::Terms => "高频词",
            TuiView::Stats => "索引统计",
        }
    }

    fn content_lines(&self, area: Rect) -> Vec<Line<'static>> {
        match self.view {
            TuiView::Home => self.home_lines(area),
            TuiView::Help => self.help_lines(),
            TuiView::Files => self.file_tree_lines(),
            TuiView::Terms => self.term_lines(),
            TuiView::Stats => self.stats_lines(),
            TuiView::Search => self.search_preview_lines(area),
        }
    }

    fn home_lines(&self, area: Rect) -> Vec<Line<'static>> {
        let quote = inspirational_quote(self.index.metadata.created_secs);
        let root = self.index.metadata.root.display().to_string();
        let width = area.width.saturating_sub(2) as usize;
        let mut lines = cheese_title_lines(width);
        lines.extend([
            right_styled_line(
                "欢 迎 使 用 “芝 士” 库",
                width,
                Color::LightYellow,
                Modifier::BOLD,
            ),
            right_styled_line("让 知 识 更 好 搜 索", width, Color::White, Modifier::BOLD),
            Line::from(""),
            pixel_dog_ear_line(),
            pixel_dog_face_line(),
            pixel_dog_muzzle_line(),
            pixel_dog_paw_line(),
            pixel_dog_tail_line(),
            Line::from(""),
            styled_line(quote, Color::Yellow, Modifier::ITALIC),
            Line::from(""),
            Line::from(format!(
                "{} documents | {} terms | {} tokens",
                self.index.metadata.document_count,
                self.index.metadata.term_count,
                self.index.metadata.total_tokens
            )),
            Line::from(format!("知识库根目录: {root}")),
            Line::from(""),
        ]);

        for text in [
            "输入 /help 查看所有命令。",
            "输入 /select 进入搜索页面。",
            "手动添加、删除或移动知识库文件后，输入 /update 更新索引。",
        ] {
            append_plain_wrapped_lines(&mut lines, text, width);
        }
        lines
    }

    fn help_lines(&self) -> Vec<Line<'static>> {
        [
            "可用命令",
            "",
            "/select  进入搜索页面。",
            "/home    返回封面首页。",
            "/help    显示所有 TUI 命令说明。",
            "/files   展示当前索引中的知识库目录树。",
            "/terms   按词频展示高频关键词。",
            "/stats   展示索引统计信息。",
            "/update  从当前知识库根目录重新构建索引并保存。",
            "/clear   清空当前搜索关键词和结果。",
            "/quit    退出 TUI。",
            "",
            "提示：这些页面都基于当前索引。修改知识库文件后，请输入 /update 刷新。",
        ]
        .into_iter()
        .map(Line::from)
        .chain(std::iter::once(Line::from(
            "返回上一页：按 Alt+Left，或在输入框为空时按 Backspace。",
        )))
        .collect()
    }

    fn file_tree_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(format!(
                "知识库根目录: {}",
                self.index.metadata.root.display()
            )),
            Line::from("以下目录基于当前索引生成。修改文件后请输入 /update 刷新。"),
            Line::from(""),
        ];
        lines.extend(build_file_tree(&self.index).into_iter().map(Line::from));
        lines
    }

    fn term_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(format!("词频最高的前 {TERMS_LIMIT} 个关键词")),
            Line::from("以下结果基于当前索引生成。修改文件后请输入 /update 刷新。"),
            Line::from(""),
        ];
        for (rank, term) in analysis::top_terms(&self.index, TERMS_LIMIT)
            .iter()
            .enumerate()
        {
            lines.push(Line::from(format!(
                "{:>2}. {:<18} freq={:<4} docs={}",
                rank + 1,
                term.term,
                term.total_frequency,
                term.document_frequency
            )));
        }
        lines
    }

    fn stats_lines(&self) -> Vec<Line<'static>> {
        let summary = analysis::summarize_index(&self.index, 5);
        vec![
            Line::from(format!("知识库根目录: {}", summary.root.display())),
            Line::from(format!("知识库根目录: {}", summary.root.display())),
            Line::from(format!("文档数量: {}", summary.document_count)),
            Line::from(format!("词项数量: {}", summary.term_count)),
            Line::from(format!("Token 总数: {}", summary.total_tokens)),
            Line::from(format!(
                "平均每篇文档 Token 数: {:.2}",
                summary.average_tokens_per_document
            )),
            Line::from(format!("索引版本: {}", self.index.metadata.version)),
            Line::from(format!("索引文件: {}", self.index_path.display())),
            Line::from(""),
            Line::from("手动修改 knowledge_base 后，请输入 /update 更新索引。"),
        ]
    }

    fn search_preview_lines(&self, area: Rect) -> Vec<Line<'static>> {
        let text_width = area.width.saturating_sub(2).max(1) as usize;
        self.results
            .get(self.selected)
            .map(|result| {
                let file_name = result_file_name(result);
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("命中词: ", Style::default().fg(Color::Green)),
                        Span::raw(result.matched_terms.join(", ")),
                    ]),
                    Line::from(vec![
                        Span::styled("文件: ", Style::default().fg(Color::Green)),
                        Span::raw(file_name),
                    ]),
                    Line::from(""),
                ];

                if result.matches.is_empty() {
                    append_wrapped_highlighted_lines(
                        &mut lines,
                        &result.snippet,
                        &result.matched_terms,
                        text_width,
                    );
                } else {
                    for (idx, item) in result.matches.iter().enumerate() {
                        let prefix = match item.page {
                            Some(page) => format!("{}. p.{}  ", idx + 1, page),
                            None => format!("{}. ", idx + 1),
                        };
                        append_wrapped_highlighted_lines(
                            &mut lines,
                            &format!("{prefix}{}", item.snippet),
                            &result.matched_terms,
                            text_width,
                        );
                        lines.push(Line::from(""));
                    }
                }
                lines
            })
            .unwrap_or_else(|| {
                vec![Line::from(
                    "请输入关键词开始搜索，或输入 /help 查看命令。按 Esc 退出。",
                )]
            })
    }
}

fn result_file_name(result: &SearchResult) -> String {
    result
        .path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| result.path.display().to_string())
}

fn preferred_update_root(current_root: &Path) -> PathBuf {
    let default_root = PathBuf::from(DEFAULT_KNOWLEDGE_BASE);
    if is_legacy_demo_root(current_root) && default_root.exists() {
        default_root
    } else {
        current_root.to_path_buf()
    }
}

fn is_legacy_demo_root(path: &Path) -> bool {
    path.file_name()
        .map(|name| name == LEGACY_DEMO_NOTES)
        .unwrap_or(false)
}

pub fn open_result_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(crate::error::AppError::MissingPath(path.to_path_buf()));
    }

    open::that(path).map_err(|err| crate::error::AppError::Terminal(err.to_string()))?;
    Ok(())
}

pub fn result_index_at(area: Rect, column: u16, row: u16, result_count: usize) -> Option<usize> {
    if !point_in_inner_area(area, column, row) {
        return None;
    }

    let index = (row - area.y.saturating_add(1)) as usize;
    (index < result_count).then_some(index)
}

fn point_in_inner_area(area: Rect, column: u16, row: u16) -> bool {
    let inner_left = area.x.saturating_add(1);
    let inner_right = area.x.saturating_add(area.width.saturating_sub(1));
    let inner_top = area.y.saturating_add(1);
    let inner_bottom = area.y.saturating_add(area.height.saturating_sub(1));

    column >= inner_left && column < inner_right && row >= inner_top && row < inner_bottom
}

fn styled_line(text: impl Into<String>, color: Color, modifier: Modifier) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Style::default().fg(color).add_modifier(modifier),
    ))
}

fn cheese_title_lines(width: usize) -> Vec<Line<'static>> {
    let title_color = Color::Rgb(226, 111, 76);
    [
        " ██████  ██  ██  ██████  ██████  ██████  ██████ ",
        "██       ██  ██  ██      ██      ██      ██     ",
        "██       ██████  █████   █████   █████   █████  ",
        "██       ██  ██  ██      ██          ██  ██     ",
        " ██████  ██  ██  ██████  ██████  ██████  ██████ ",
        "",
        "██████   █████   ██████  ██████ ",
        "██   ██ ██   ██ ██       ██     ",
        "██████  ███████  █████   █████  ",
        "██   ██ ██   ██      ██  ██     ",
        "██████  ██   ██ ██████   ██████ ",
    ]
    .into_iter()
    .map(|line| centered_styled_line(line, width, title_color, Modifier::BOLD))
    .collect()
}

fn centered_styled_line(
    text: &str,
    width: usize,
    color: Color,
    modifier: Modifier,
) -> Line<'static> {
    let text_width = unicode_width::UnicodeWidthStr::width(text);
    let padding = width.saturating_sub(text_width) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(padding)),
        Span::styled(
            text.to_string(),
            Style::default().fg(color).add_modifier(modifier),
        ),
    ])
}

fn right_styled_line(text: &str, width: usize, color: Color, modifier: Modifier) -> Line<'static> {
    let text_width = unicode_width::UnicodeWidthStr::width(text);
    let padding = width.saturating_sub(text_width);
    Line::from(vec![
        Span::raw(" ".repeat(padding)),
        Span::styled(
            text.to_string(),
            Style::default().fg(color).add_modifier(modifier),
        ),
    ])
}

fn pixel_dog_ear_line() -> Line<'static> {
    let fur = Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD);
    Line::from(vec![Span::raw("        "), Span::styled(" / \\__", fur)])
}

fn pixel_dog_face_line() -> Line<'static> {
    let fur = Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::raw("        "),
        Span::styled("(    @\\___", fur),
    ])
}

fn pixel_dog_muzzle_line() -> Line<'static> {
    let fur = Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::raw("        "),
        Span::styled(" /         O", fur),
    ])
}

fn pixel_dog_paw_line() -> Line<'static> {
    Line::from(vec![
        Span::raw("        "),
        Span::styled(
            "/   (_____/",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn pixel_dog_tail_line() -> Line<'static> {
    Line::from(vec![
        Span::raw("        "),
        Span::styled(
            "/_____/   U",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn inspirational_quote(seed: u64) -> &'static str {
    const QUOTES: &[&str] = &[
        "Practice makes perfect.",
        "Small steps compound into mastery.",
        "Read deeply, build steadily.",
        "Search, learn, connect.",
    ];
    QUOTES[seed as usize % QUOTES.len()]
}

fn build_file_tree(index: &InvertedIndex) -> Vec<String> {
    let root_name = index
        .metadata
        .root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| index.metadata.root.display().to_string());
    let mut tree = TreeNode::default();

    for document in &index.documents {
        let relative = document
            .path
            .strip_prefix(&index.metadata.root)
            .unwrap_or(&document.path);
        let parts = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        tree.insert(&parts);
    }

    let mut lines = vec![root_name];
    tree.render("", &mut lines);
    lines
}

#[derive(Debug, Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    fn insert(&mut self, parts: &[String]) {
        let Some((first, rest)) = parts.split_first() else {
            return;
        };
        self.children.entry(first.clone()).or_default().insert(rest);
    }

    fn render(&self, prefix: &str, lines: &mut Vec<String>) {
        let total = self.children.len();
        for (idx, (name, child)) in self.children.iter().enumerate() {
            let is_last = idx + 1 == total;
            let marker = if is_last { "`-- " } else { "|-- " };
            lines.push(format!("{prefix}{marker}{name}"));
            let child_prefix = if is_last { "    " } else { "|   " };
            child.render(&format!("{prefix}{child_prefix}"), lines);
        }
    }
}

fn highlighted_snippet_line(snippet: &str, terms: &[String]) -> Line<'static> {
    let Some((start, end)) = first_match_range(snippet, terms) else {
        return Line::from(snippet.to_string());
    };

    let spans = vec![
        Span::raw(snippet[..start].to_string()),
        Span::styled(
            snippet[start..end].to_string(),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(snippet[end..].to_string()),
    ];
    Line::from(spans)
}

fn append_wrapped_highlighted_lines(
    lines: &mut Vec<Line<'static>>,
    text: &str,
    terms: &[String],
    width: usize,
) {
    for wrapped in wrap_text_by_width(text, width) {
        lines.push(highlighted_snippet_line(&wrapped, terms));
    }
}

fn append_plain_wrapped_lines(lines: &mut Vec<Line<'static>>, text: &str, width: usize) {
    for wrapped in wrap_text_by_width(text, width) {
        lines.push(Line::from(wrapped));
    }
}

fn wrap_text_by_width(text: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for ch in text.chars() {
        let width = ch.width().unwrap_or(0).max(1);
        if current_width > 0 && current_width + width > max_width {
            lines.push(current.trim_end().to_string());
            current.clear();
            current_width = 0;
            if ch.is_whitespace() {
                continue;
            }
        }

        current.push(ch);
        current_width += width;
    }

    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn result_page_summary(result: &SearchResult) -> String {
    let mut pages = result
        .matches
        .iter()
        .filter_map(|item| item.page)
        .collect::<Vec<_>>();
    pages.sort_unstable();
    pages.dedup();

    if pages.is_empty() {
        return "pages: -".to_string();
    }

    let rendered = pages
        .iter()
        .take(3)
        .map(|page| format!("p.{page}"))
        .collect::<Vec<_>>()
        .join(",");
    if pages.len() > 3 {
        format!("pages: {rendered}+")
    } else {
        format!("pages: {rendered}")
    }
}

fn first_match_range(snippet: &str, terms: &[String]) -> Option<(usize, usize)> {
    let lower = snippet.to_lowercase();
    terms
        .iter()
        .filter(|term| !term.is_empty())
        .filter_map(|term| {
            let needle = term.to_lowercase();
            lower
                .find(&needle)
                .map(|start| (start, start + needle.len()))
        })
        .min_by_key(|(start, _)| *start)
}

#[derive(Debug, Clone, Default)]
struct InputLine {
    text: String,
    cursor: usize,
}

impl InputLine {
    fn text(&self) -> &str {
        &self.text
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.char_len();
    }

    fn insert(&mut self, ch: char) {
        let byte_index = self.byte_index();
        self.text.insert(byte_index, ch);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        self.cursor -= 1;
        let byte_index = self.byte_index();
        self.text.remove(byte_index);
    }

    fn delete(&mut self) {
        if self.cursor >= self.char_len() {
            return;
        }

        let byte_index = self.byte_index();
        self.text.remove(byte_index);
    }

    fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.char_len());
    }

    fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    fn move_to_end(&mut self) {
        self.cursor = self.char_len();
    }

    fn prefix_width(&self) -> usize {
        self.text
            .chars()
            .take(self.cursor)
            .map(|ch| ch.width().unwrap_or(0))
            .sum()
    }

    fn byte_index(&self) -> usize {
        self.text
            .char_indices()
            .nth(self.cursor)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| self.text.len())
    }

    fn char_len(&self) -> usize {
        self.text.chars().count()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::index::IndexBuilder;
    use crate::parser::SimpleTokenizer;

    use super::*;

    #[test]
    fn input_line_edits_at_cursor() {
        let mut input = InputLine::default();
        input.insert('r');
        input.insert('s');
        input.move_left();
        input.insert('u');

        assert_eq!(input.text(), "rus");
    }

    #[test]
    fn input_line_handles_chinese_width_and_delete() {
        let mut input = InputLine::default();
        input.insert('所');
        input.insert('有');
        input.insert('权');
        input.move_left();
        input.delete();

        assert_eq!(input.text(), "所有");
        assert_eq!(
            input.prefix_width(),
            "所有"
                .chars()
                .map(|ch| ch.width().unwrap_or(0))
                .sum::<usize>()
        );
    }

    #[test]
    fn maps_mouse_click_to_result_index() {
        let area = Rect::new(10, 5, 30, 10);

        assert_eq!(result_index_at(area, 11, 6, 3), Some(0));
        assert_eq!(result_index_at(area, 11, 8, 3), Some(2));
        assert_eq!(result_index_at(area, 11, 9, 3), None);
        assert_eq!(result_index_at(area, 9, 6, 3), None);
    }

    #[test]
    fn opening_missing_file_returns_error() {
        let missing = Path::new("definitely-missing-file.md");

        assert!(open_result_path(missing).is_err());
    }

    #[test]
    fn highlighted_snippet_supports_chinese_terms() {
        let line = highlighted_snippet_line("支持所有权模型", &["所有权".to_string()]);

        assert_eq!(line.spans.len(), 3);
    }

    #[test]
    fn preview_text_wraps_to_multiple_rows() {
        let lines = wrap_text_by_width("ownership appears in a long preview line", 12);

        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|line| unicode_width::UnicodeWidthStr::width(line.as_str()) <= 12)
        );
    }

    #[test]
    fn file_tree_uses_index_paths() {
        let temp = tempdir().expect("tempdir");
        let nested = temp.path().join("数据库").join("MySQL");
        fs::create_dir_all(&nested).expect("mkdir");
        fs::write(nested.join("index.md"), "# MySQL\ntransaction").expect("write");

        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");
        let tree = build_file_tree(&index).join("\n");

        assert!(tree.contains("数据库"));
        assert!(tree.contains("MySQL"));
        assert!(tree.contains("index.md"));
    }

    #[test]
    fn slash_select_enters_search_view() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("note.md"), "# Note\nRust").expect("write");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");
        let mut app = TuiApp::new(
            index,
            temp.path().join("index.json"),
            SimpleTokenizer::default(),
        );

        app.input.set_text("/select");
        let should_quit = app.execute_command().expect("command");

        assert!(!should_quit);
        assert_eq!(app.view, TuiView::Search);
    }

    #[test]
    fn back_navigation_returns_to_previous_view() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("note.md"), "# Note\nRust").expect("write");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");
        let mut app = TuiApp::new(
            index,
            temp.path().join("index.json"),
            SimpleTokenizer::default(),
        );

        app.input.set_text("/select");
        app.execute_command().expect("select");
        app.go_back().expect("back");

        assert_eq!(app.view, TuiView::Home);
    }

    #[test]
    fn update_root_migrates_legacy_demo_notes_to_knowledge_base() {
        let root = preferred_update_root(Path::new("demo_notes"));

        if Path::new("knowledge_base").exists() {
            assert_eq!(root, PathBuf::from("knowledge_base"));
        } else {
            assert_eq!(root, PathBuf::from("demo_notes"));
        }
    }

    #[test]
    fn update_root_keeps_custom_knowledge_base() {
        let custom = PathBuf::from("my_notes");

        assert_eq!(preferred_update_root(&custom), custom);
    }
}
