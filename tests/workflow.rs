use std::fs;

use rust_note_search::index::IndexBuilder;
use rust_note_search::parser::SimpleTokenizer;
use rust_note_search::search::SearchEngine;
use rust_note_search::storage::{load_index, save_index};
use tempfile::tempdir;

#[test]
fn full_index_save_load_search_workflow() {
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join("rust.md"),
        "# Rust\nOwnership and borrowing are central Rust ideas.",
    )
    .expect("write rust");
    fs::write(
        temp.path().join("cn.md"),
        "# 中文\n本地知识库搜索支持所有权和模块化设计。",
    )
    .expect("write cn");

    let builder = IndexBuilder::new(SimpleTokenizer::default());
    let index = builder.build(temp.path()).expect("build");
    let index_path = temp.path().join("index.json");
    save_index(&index_path, &index).expect("save");

    let loaded = load_index(&index_path).expect("load");
    let engine = SearchEngine::new(&loaded, SimpleTokenizer::default());
    let english_results = engine.search("ownership", 5).expect("english search");
    let chinese_results = engine.search("所有权", 5).expect("chinese search");

    assert!(english_results.iter().any(|item| item.title == "Rust"));
    assert!(chinese_results.iter().any(|item| item.title == "中文"));
}

#[test]
fn missing_index_file_returns_error() {
    let temp = tempdir().expect("tempdir");
    let missing = temp.path().join("missing.json");

    let result = load_index(&missing);

    assert!(result.is_err());
}
