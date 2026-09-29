use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

// How many hits to show from each search. The term is explained in the manual.
const TOP_K: usize = 5;

// Used when no query is given on the command line.
const DEFAULT_QUERY: &str = "memory leak";

// Set this to a checkout of the Ultimate Rust Foundations notes to search the
// full corpus instead of the small shared sample in `code/rag_corpus`.
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

struct Document {
    path: String,
    body: String,
}

fn corpus_root() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var(CORPUS_ENV) {
        return Ok(PathBuf::from(dir));
    }
    // The sample corpus lives once, in code/rag_corpus.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("rag_corpus"))
}

// Walk a directory tree, collecting markdown files. We deliberately skip the
// `code/` folders: we want the prose, not the source.
fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if name.starts_with('.') || name == "code" || name == "images" {
                continue;
            }
            collect_markdown_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    Ok(())
}

fn load_documents(root: &Path) -> Result<Vec<Document>> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;
    files.sort();

    let mut documents = Vec::new();
    for file in files {
        let body = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        let path = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .display()
            .to_string();
        documents.push(Document { path, body });
    }
    Ok(documents)
}

// The naive version: does the text contain the words we typed? We score by how
// many times the terms appear, which is crude but surprisingly usable.
fn keyword_search<'a>(
    documents: &'a [Document],
    query: &str,
    k: usize,
) -> Vec<(&'a Document, usize)> {
    let terms = query
        .to_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut hits = Vec::new();
    for document in documents {
        let haystack = document.body.to_lowercase();
        let mut count = 0;
        for term in &terms {
            count += haystack.matches(term.as_str()).count();
        }
        if count > 0 {
            hits.push((document, count));
        }
    }

    hits.sort_by(|left, right| right.1.cmp(&left.1));
    hits.truncate(k);
    hits
}

fn build_full_text_index(documents: &[Document]) -> Result<Connection> {
    // In-memory, so there is nothing to clean up. FTS5 builds the inverted
    // index for us.
    let connection = Connection::open_in_memory()?;
    connection.execute("CREATE VIRTUAL TABLE docs USING fts5(path, body)", [])?;

    {
        let mut insert = connection.prepare("INSERT INTO docs (path, body) VALUES (?1, ?2)")?;
        for document in documents {
            insert.execute((&document.path, &document.body))?;
        }
    }
    Ok(connection)
}

// BM25 (via FTS5) returns a negative score where smaller means better, so we
// flip the sign for display.
fn full_text_search(connection: &Connection, query: &str, k: usize) -> Result<Vec<(String, f64)>> {
    let mut statement = connection.prepare(
        "SELECT path, bm25(docs) AS rank FROM docs WHERE docs MATCH ?1 ORDER BY rank LIMIT ?2",
    )?;
    let rows = statement.query_map((query, k as i64), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        let (path, rank) = row?;
        results.push((path, -rank));
    }
    Ok(results)
}

fn main() -> Result<()> {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_QUERY.to_string());

    let root = corpus_root()?;
    if !root.is_dir() {
        bail!(
            "corpus directory {} does not exist - set {CORPUS_ENV} to a checkout of the notes",
            root.display()
        );
    }

    // Resolve any `..` so the paths we print are tidy.
    let root = fs::canonicalize(&root).unwrap_or(root);

    let documents = load_documents(&root)?;
    println!("Loaded {} markdown files from {}", documents.len(), root.display());
    println!("Query: {query:?}");
    println!();

    println!("Keyword search (top {TOP_K}):");
    for (document, count) in keyword_search(&documents, &query, TOP_K) {
        println!("  {count:>4}  {}", document.path);
    }

    let connection = build_full_text_index(&documents)?;

    println!();
    println!("Full text search (top {TOP_K}):");
    for (path, score) in full_text_search(&connection, &query, TOP_K)? {
        println!("  {score:>6.2}  {path}");
    }

    Ok(())
}
