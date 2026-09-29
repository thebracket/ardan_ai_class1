use anyhow::{Context, Result};
use arrow_array::types::Float32Type;
use arrow_array::{Array, FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;
const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";
const DIM: i32 = 384;
const TOP_K: usize = 5;
const DEFAULT_QUERY: &str = "how do I share data between threads?";
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

struct Chunk {
    path: String,
    heading: String,
    body: String,
}

fn cache_dir() -> PathBuf {
    if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("ai_workshop").join("embeddings");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".cache")
        .join("ai_workshop")
        .join("embeddings")
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

fn load_chunks(root: &Path) -> Result<Vec<Chunk>> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;
    files.sort();

    let mut chunks = Vec::new();
    for file in files {
        let markdown = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        let path = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .display()
            .to_string();
        chunks.extend(chunk_markdown(&path, &markdown));
    }
    Ok(chunks)
}

fn parse_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|character| *character == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let title = trimmed[level..].trim();
    if title.is_empty() {
        return None;
    }
    Some((level, title.to_string()))
}

fn heading_path(stack: &[(usize, String)]) -> String {
    stack
        .iter()
        .map(|(_, title)| title.as_str())
        .collect::<Vec<_>>()
        .join(" > ")
}

fn push_chunk(chunks: &mut Vec<Chunk>, path: &str, stack: &[(usize, String)], body: &mut String) {
    let text = body.trim();
    if !text.is_empty() {
        chunks.push(Chunk {
            path: path.to_string(),
            heading: heading_path(stack),
            body: text.to_string(),
        });
    }
    body.clear();
}

fn chunk_markdown(path: &str, markdown: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut body = String::new();
    let mut in_code_fence = false;

    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_code_fence = !in_code_fence;
            body.push_str(line);
            body.push('\n');
            continue;
        }

        if !in_code_fence {
            if let Some((level, title)) = parse_heading(line) {
                push_chunk(&mut chunks, path, &stack, &mut body);
                stack.truncate(level - 1);
                stack.push((level, title));
                continue;
            }
        }

        body.push_str(line);
        body.push('\n');
    }

    push_chunk(&mut chunks, path, &stack, &mut body);
    chunks
}

fn passage_text(chunk: &Chunk) -> String {
    if chunk.heading.is_empty() {
        format!("{PASSAGE_PREFIX}{}", chunk.body)
    } else {
        format!("{PASSAGE_PREFIX}{}: {}", chunk.heading, chunk.body)
    }
}

fn preview(body: &str, width: usize) -> String {
    let flattened = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flattened.chars().count() <= width {
        flattened
    } else {
        let short = flattened.chars().take(width).collect::<String>();
        format!("{short}...")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_QUERY.to_string());

    let root = corpus_root()?;
    let root = fs::canonicalize(&root).unwrap_or(root);
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    let database = output_dir.join("lancedb");

    let chunks = load_chunks(&root)?;
    println!("Corpus: {} chunks from {}", chunks.len(), root.display());

    let mut model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;

    let passages = chunks.iter().map(passage_text).collect::<Vec<_>>();
    let embeddings = model.embed(passages, None)?;

    // A vector column is just a FixedSizeList<Float32> in an Arrow schema.
    let schema = Arc::new(Schema::new(vec![
        Field::new("path", DataType::Utf8, false),
        Field::new("heading", DataType::Utf8, false),
        Field::new("body", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), DIM),
            false,
        ),
    ]));

    let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        embeddings
            .iter()
            .map(|embedding| Some(embedding.iter().map(|value| Some(*value)).collect::<Vec<_>>())),
        DIM,
    );
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                chunks.iter().map(|chunk| chunk.path.clone()).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                chunks
                    .iter()
                    .map(|chunk| chunk.heading.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                chunks.iter().map(|chunk| chunk.body.clone()).collect::<Vec<_>>(),
            )),
            Arc::new(vectors),
        ],
    )?;

    // Start from a clean database each run.
    let _ = fs::remove_dir_all(&database);
    fs::create_dir_all(&output_dir)?;
    let db = lancedb::connect(database.to_string_lossy().as_ref())
        .execute()
        .await?;
    let table = db.create_table("chunks", batch).execute().await?;
    println!("Table written to {}", database.display());

    let query_embedding = model
        .embed(vec![format!("{QUERY_PREFIX}{query}")], None)?
        .remove(0);

    println!();
    println!("Query: {query:?}");
    println!("Top {TOP_K}:");
    let stream = table.vector_search(query_embedding)?.limit(TOP_K).execute().await?;
    let batches = stream.try_collect::<Vec<_>>().await?;
    let mut rank = 0;
    for batch in &batches {
        let paths = batch
            .column_by_name("path")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let headings = batch
            .column_by_name("heading")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let bodies = batch
            .column_by_name("body")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let distances = batch
            .column_by_name("_distance")
            .unwrap()
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap();

        for row in 0..batch.num_rows() {
            rank += 1;
            let heading = headings.value(row);
            let heading = if heading.is_empty() { "-" } else { heading };
            println!(
                "  {rank}. distance {:.4}  {} > {heading}",
                distances.value(row),
                paths.value(row)
            );
            println!("       {}", preview(bodies.value(row), 100));
        }
    }

    Ok(())
}
