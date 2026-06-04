use std::fs;

use rust_note_search::index::IndexBuilder;
use rust_note_search::parser::SimpleTokenizer;
use rust_note_search::search::SearchEngine;
use tempfile::tempdir;

#[test]
fn text_pdf_can_be_indexed_and_searched() {
    let temp = tempdir().expect("tempdir");
    write_text_pdf(temp.path().join("paper.pdf"), "Rust ownership pdf search");

    let builder = IndexBuilder::new(SimpleTokenizer::default());
    let index = builder.build(temp.path()).expect("build");
    let engine = SearchEngine::new(&index, SimpleTokenizer::default());
    let results = engine.search("ownership", 5).expect("search");

    assert!(results.iter().any(|result| result.title == "paper"));
    assert!(
        results
            .iter()
            .flat_map(|result| result.matches.iter())
            .any(|item| item.page == Some(1)),
        "PDF search results should keep page numbers for matched content"
    );
}

#[test]
fn invalid_pdf_returns_indexing_error() {
    let temp = tempdir().expect("tempdir");
    fs::write(temp.path().join("broken.pdf"), b"not a real pdf").expect("write pdf");

    let builder = IndexBuilder::new(SimpleTokenizer::default());
    let result = builder.build(temp.path());

    assert!(result.is_err());
}

fn write_text_pdf(path: impl AsRef<std::path::Path>, text: &str) {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let page_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = document.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 18.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal(text)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content.encode().expect("encode pdf content"),
    ));

    document.objects.insert(
        page_id,
        dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Contents" => content_id,
            "Resources" => resources_id,
        }
        .into(),
    );
    document.objects.insert(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }
        .into(),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);
    document.compress();
    document.save(path).expect("save pdf");
}
