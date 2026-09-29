use anyhow::{bail, Context, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;
const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";
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

// sqlite-vec expects a little-endian f32 blob, which is exactly what we store.
fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for value in embedding {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn build_index(database: &Path, model: &mut TextEmbedding, chunks: &[Chunk]) -> Result<()> {
    let connection = Connection::open(database)?;
    connection.execute("DROP TABLE IF EXISTS chunks", [])?;
    connection.execute("DROP TABLE IF EXISTS vec_chunks", [])?;
    connection.execute(
        "CREATE TABLE chunks (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            heading TEXT NOT NULL,
            body TEXT NOT NULL
        )",
        [],
    )?;

    // The whole point: a real vector index living inside SQLite. The dimension
    // has to match the embedding model.
    connection.execute(
        "CREATE VIRTUAL TABLE vec_chunks USING vec0(embedding float[384])",
        [],
    )?;

    let passages = chunks.iter().map(passage_text).collect::<Vec<_>>();
    let embeddings = model.embed(passages, None).context("embedding chunks failed")?;

    {
        let mut insert_chunk =
            connection.prepare("INSERT INTO chunks (id, path, heading, body) VALUES (?1, ?2, ?3, ?4)")?;
        let mut insert_vector =
            connection.prepare("INSERT INTO vec_chunks (rowid, embedding) VALUES (?1, ?2)")?;
        for (index, (chunk, embedding)) in chunks.iter().zip(&embeddings).enumerate() {
            insert_chunk.execute(params![index as i64, &chunk.path, &chunk.heading, &chunk.body])?;
            insert_vector.execute(params![index as i64, embedding_to_blob(embedding)])?;
        }
    }

    Ok(())
}

fn search(connection: &Connection, query_embedding: &[f32], k: usize) -> Result<Vec<(String, String, String, f64)>> {
    // The whole search is one SQL query. No cosine loop in Rust; the index does
    // the work.
    // The KNN query has to stand on its own so vec0 can see the LIMIT; we join
    // the metadata back on afterwards.
    let mut statement = connection.prepare(
        "SELECT c.path, c.heading, c.body, hits.distance
         FROM (
             SELECT rowid, distance FROM vec_chunks
             WHERE embedding MATCH ?1 ORDER BY distance LIMIT ?2
         ) AS hits
         JOIN chunks c ON c.id = hits.rowid
         ORDER BY hits.distance",
    )?;
    let rows = statement.query_map(params![embedding_to_blob(query_embedding), k as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, f64>(3)?,
        ))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
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

fn main() -> Result<()> {
    // Register the sqlite-vec extension with every connection in this process.
    // This is the one unsafe line you need; after it, `vec0` just exists.
    unsafe {
        use rusqlite::ffi::sqlite3_auto_extension;
        use sqlite_vec::sqlite3_vec_init;
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

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
    let root = fs::canonicalize(&root).unwrap_or(root);

    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output_dir)?;
    let database = output_dir.join("rag_vec.sqlite");

    let chunks = load_chunks(&root)?;
    println!("Corpus: {} chunks from {}", chunks.len(), root.display());

    let mut model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;

    build_index(&database, &mut model, &chunks)?;
    println!("Index written to {}", database.display());

    let connection = Connection::open(&database)?;
    let version: String = connection.query_row("SELECT vec_version()", [], |row| row.get(0))?;
    println!("sqlite-vec version: {version}");

    let query_embedding = model
        .embed(vec![format!("{QUERY_PREFIX}{query}")], None)?
        .remove(0);

    println!();
    println!("Query: {query:?}");
    println!("Top {TOP_K}:");
    for (rank, (path, heading, body, distance)) in search(&connection, &query_embedding, TOP_K)?
        .iter()
        .enumerate()
    {
        let heading = if heading.is_empty() { "-" } else { heading };
        println!("  {}. distance {:.4}  {path} > {heading}", rank + 1, distance);
        println!("       {}", preview(body, 100));
    }

    Ok(())
}
