use anyhow::{bail, Context, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;

// E5 wants a different prefix for the thing you search *with* and the things
// you search *over*. Getting these the wrong way round quietly costs quality.
const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";

const TOP_K: usize = 5;
const DEFAULT_QUERY: &str = "how do I share data between threads?";
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

struct Document {
    path: String,
    body: String,
}

// A chunk is one section of a markdown file, plus the trail of headings that
// led to it - because a section torn out of context often loses the words that
// made it findable.
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

// Returns the heading level and title if this line is a markdown heading.
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

// Split a markdown file on its headings. We track fenced code blocks so a `#`
// inside a code sample isn't mistaken for a heading.
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
                // A new heading closes everything at the same or deeper level.
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

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for value in embedding {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect()
}

fn passage_text(chunk: &Chunk) -> String {
    if chunk.heading.is_empty() {
        format!("{PASSAGE_PREFIX}{}", chunk.body)
    } else {
        format!("{PASSAGE_PREFIX}{}: {}", chunk.heading, chunk.body)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum();
    let length = |vector: &[f32]| {
        vector
            .iter()
            .map(|value| f64::from(*value) * f64::from(*value))
            .sum::<f64>()
            .sqrt()
    };
    dot / (length(a) * length(b))
}

// Build a fresh SQLite database: one row per chunk, with the vector stored as a
// little-endian BLOB.
fn build_index(database: &Path, model: &mut TextEmbedding, chunks: &[Chunk]) -> Result<()> {
    let connection = Connection::open(database)?;
    connection.execute("DROP TABLE IF EXISTS chunks", [])?;
    connection.execute(
        "CREATE TABLE chunks (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            heading TEXT NOT NULL,
            body TEXT NOT NULL,
            embedding BLOB NOT NULL
        )",
        [],
    )?;

    let passages = chunks.iter().map(passage_text).collect::<Vec<_>>();
    let embeddings = model.embed(passages, None).context("embedding chunks failed")?;

    {
        let mut insert = connection
            .prepare("INSERT INTO chunks (path, heading, body, embedding) VALUES (?1, ?2, ?3, ?4)")?;
        for (chunk, embedding) in chunks.iter().zip(embeddings) {
            insert.execute((
                &chunk.path,
                &chunk.heading,
                &chunk.body,
                embedding_to_blob(&embedding),
            ))?;
        }
    }

    Ok(())
}

struct StoredChunk {
    path: String,
    heading: String,
    body: String,
    embedding: Vec<f32>,
}

fn load_chunks(connection: &Connection) -> Result<Vec<StoredChunk>> {
    let mut statement = connection.prepare("SELECT path, heading, body, embedding FROM chunks")?;
    let rows = statement.query_map([], |row| {
        let blob: Vec<u8> = row.get(3)?;
        Ok(StoredChunk {
            path: row.get(0)?,
            heading: row.get(1)?,
            body: row.get(2)?,
            embedding: blob_to_embedding(&blob),
        })
    })?;

    let mut chunks = Vec::new();
    for row in rows {
        chunks.push(row?);
    }
    Ok(chunks)
}

fn search<'a>(chunks: &'a [StoredChunk], query: &[f32], k: usize) -> Vec<(f64, &'a StoredChunk)> {
    let mut scored = chunks
        .iter()
        .map(|chunk| (cosine_similarity(query, &chunk.embedding), chunk))
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
    scored.truncate(k);
    scored
}

fn preview(body: &str, width: usize) -> String {
    let flattened = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flattened.len() <= width {
        flattened
    } else {
        format!("{}...", &flattened[..width])
    }
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

    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output_dir)?;
    let database = output_dir.join("rag_index.sqlite");

    let documents = load_documents(&root)?;
    let mut chunks = Vec::new();
    for document in &documents {
        chunks.extend(chunk_markdown(&document.path, &document.body));
    }
    println!("Loaded {} markdown files from {}", documents.len(), root.display());
    println!("Split them into {} chunks", chunks.len());

    let mut model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;

    build_index(&database, &mut model, &chunks)?;
    println!("Index written to {}", database.display());

    let connection = Connection::open(&database)?;
    let stored = load_chunks(&connection)?;

    let query_embedding = model
        .embed(vec![format!("{QUERY_PREFIX}{query}")], None)?
        .remove(0);

    println!();
    println!("Query: {query:?}");
    println!("Top {TOP_K} chunks:");
    for (rank, (score, chunk)) in search(&stored, &query_embedding, TOP_K).iter().enumerate() {
        let heading = if chunk.heading.is_empty() {
            "(intro)"
        } else {
            &chunk.heading
        };
        println!("  {}. {:.3}  {} > {heading}", rank + 1, score, chunk.path);
        println!("       {}", preview(&chunk.body, 100));
    }

    Ok(())
}
