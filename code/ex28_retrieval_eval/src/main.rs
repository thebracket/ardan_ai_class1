use anyhow::{bail, Context, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;
const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

// A golden set: questions we know the answer to, and the file the answer is in.
// The expected value is a path substring, so the eval still works when you
// point RUST_NOTES_DIR at the full notes.
struct Question {
    text: &'static str,
    expected: &'static str,
}

const QUESTIONS: [Question; 10] = [
    Question { text: "How do I share data between threads?", expected: "Mutexes.md" },
    Question { text: "How do I send data from one thread to another?", expected: "Channels.md" },
    Question { text: "What are the performance costs of locking a mutex?", expected: "Mutexes.md" },
    Question { text: "How do I use Rc to avoid lifetime problems?", expected: "Lifetimes.md" },
    Question {
        text: "Why does the borrow checker complain about returning references?",
        expected: "Lifetimes.md",
    },
    Question { text: "How do I find and fix a memory leak?", expected: "MemoryLeaks.md" },
    Question { text: "What is async/await?", expected: "AsyncAwaitOverview.md" },
    Question { text: "What is the difference between a sender and a receiver?", expected: "Channels.md" },
    Question {
        text: "How do I stop threads from corrupting shared data?",
        expected: "Mutexes.md",
    },
    Question {
        text: "What should I learn first when starting with Rust?",
        expected: "RustFundamentals.md",
    },
];

struct Document {
    path: String,
    body: String,
}

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

// Both keyword methods work on the same words: split the question on anything
// that isn't a letter or digit, and lowercase.
fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

// Method one: count how often the words appear. No index, no ranking cleverness.
fn keyword_ranking(documents: &[Document], query: &str) -> Vec<String> {
    let terms = query_terms(query);
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
    hits.into_iter().map(|(document, _)| document.path.clone()).collect()
}

// Method two: SQLite's FTS5 index, ranked by BM25.
fn build_fts(documents: &[Document]) -> Result<Connection> {
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

fn full_text_ranking(connection: &Connection, query: &str) -> Result<Vec<String>> {
    // Quote each word and OR them together: a plain question contains words
    // FTS5 would read as query syntax.
    let fts = query_terms(query)
        .iter()
        .map(|term| format!("\"{term}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    if fts.is_empty() {
        return Ok(Vec::new());
    }

    let mut statement = connection
        .prepare("SELECT path FROM docs WHERE docs MATCH ?1 ORDER BY bm25(docs)")?;
    let rows = statement.query_map([fts], |row| row.get::<_, String>(0))?;

    // Collapse to one entry per file, keeping the best (first) position.
    let mut ranked: Vec<String> = Vec::new();
    for row in rows {
        let path = row?;
        if !ranked.contains(&path) {
            ranked.push(path);
        }
    }
    Ok(ranked)
}

// Method three: embed, cosine, collapse chunks down to their file.
fn semantic_ranking(question: &[f32], chunks: &[Chunk], embeddings: &[Vec<f32>]) -> Vec<String> {
    let mut best_per_file: HashMap<&str, f64> = HashMap::new();
    for (chunk, embedding) in chunks.iter().zip(embeddings) {
        let score = cosine_similarity(question, embedding);
        best_per_file
            .entry(chunk.path.as_str())
            .and_modify(|current| {
                if score > *current {
                    *current = score;
                }
            })
            .or_insert(score);
    }

    let mut ranked = best_per_file.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.partial_cmp(&left.1).unwrap());
    ranked.into_iter().map(|(path, _)| path.to_string()).collect()
}

#[derive(Default)]
struct Metrics {
    hit1: usize,
    hit3: usize,
    hit5: usize,
    reciprocal_rank: f64,
}

impl Metrics {
    fn record(&mut self, rank: Option<usize>) {
        if let Some(rank) = rank {
            if rank == 1 {
                self.hit1 += 1;
            }
            if rank <= 3 {
                self.hit3 += 1;
            }
            if rank <= 5 {
                self.hit5 += 1;
            }
            self.reciprocal_rank += 1.0 / rank as f64;
        }
    }
}

fn rank_of(ranked: &[String], expected: &str) -> Option<usize> {
    ranked
        .iter()
        .position(|path| path.contains(expected))
        .map(|index| index + 1)
}

fn format_rank(rank: Option<usize>) -> String {
    match rank {
        Some(rank) => format!("#{rank}"),
        None => "-".to_string(),
    }
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(width - 3).collect::<String>())
    }
}

fn main() -> Result<()> {
    let root = corpus_root()?;
    if !root.is_dir() {
        bail!(
            "corpus directory {} does not exist - set {CORPUS_ENV} to a checkout of the notes",
            root.display()
        );
    }
    let root = fs::canonicalize(&root).unwrap_or(root);

    let documents = load_documents(&root)?;
    let mut chunks = Vec::new();
    for document in &documents {
        chunks.extend(chunk_markdown(&document.path, &document.body));
    }
    println!(
        "Corpus: {} files, {} chunks from {}",
        documents.len(),
        chunks.len(),
        root.display()
    );
    println!("Questions: {}", QUESTIONS.len());
    println!();

    let connection = build_fts(&documents)?;

    let mut model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;

    let passages = chunks.iter().map(passage_text).collect::<Vec<_>>();
    let chunk_embeddings = model.embed(passages, None)?;

    let query_inputs = QUESTIONS
        .iter()
        .map(|question| format!("{QUERY_PREFIX}{}", question.text))
        .collect::<Vec<_>>();
    let question_embeddings = model.embed(query_inputs, None)?;

    let mut metrics = [
        Metrics::default(),
        Metrics::default(),
        Metrics::default(),
    ];

    println!(
        "{:<50} {:>8} {:>10} {:>9}",
        "Question", "keyword", "full-text", "semantic"
    );
    for (index, question) in QUESTIONS.iter().enumerate() {
        let rankings = [
            keyword_ranking(&documents, question.text),
            full_text_ranking(&connection, question.text)?,
            semantic_ranking(&question_embeddings[index], &chunks, &chunk_embeddings),
        ];

        let mut cells = Vec::new();
        for (method, ranking) in rankings.iter().enumerate() {
            let rank = rank_of(ranking, question.expected);
            metrics[method].record(rank);
            cells.push(format_rank(rank));
        }

        println!(
            "{:<50} {:>8} {:>10} {:>9}",
            truncate(question.text, 50),
            cells[0],
            cells[1],
            cells[2]
        );
    }

    let total = QUESTIONS.len() as f64;
    println!();
    println!("{:<12} {:>6} {:>7} {:>7} {:>7}", "Method", "hit@1", "hit@3", "hit@5", "MRR");
    for (name, metrics) in ["keyword", "full-text", "semantic"].iter().zip(&metrics) {
        println!(
            "{:<12} {:>6.2} {:>7.2} {:>7.2} {:>7.2}",
            name,
            metrics.hit1 as f64 / total,
            metrics.hit3 as f64 / total,
            metrics.hit5 as f64 / total,
            metrics.reciprocal_rank / total
        );
    }

    Ok(())
}
