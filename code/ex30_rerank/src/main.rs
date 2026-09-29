use anyhow::{bail, Context, Result};
use fastembed::{
    EmbeddingModel, RerankInitOptions, RerankerModel, TextEmbedding, TextInitOptions, TextRerank,
};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const EMBED_MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;
const RERANK_MODEL: RerankerModel = RerankerModel::BGERerankerBase;
const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

const RRF_K: usize = 60;

// How many candidate passages the first stage hands to the reranker. A real
// pipeline uses 50-100; more candidates means better recall but more reranking.
const CANDIDATES: usize = 50;

const WORKED_EXAMPLE: usize = 6;

struct Question {
    text: &'static str,
    keywords: &'static str,
    expected: &'static str,
}

const QUESTIONS: [Question; 10] = [
    Question {
        text: "How do I share data between threads?",
        keywords: "mutex",
        expected: "Mutexes.md",
    },
    Question {
        text: "How do I send data from one thread to another?",
        keywords: "channel",
        expected: "Channels.md",
    },
    Question {
        text: "What are the performance costs of locking a mutex?",
        keywords: "mutex performance",
        expected: "Mutexes.md",
    },
    Question {
        text: "How do I use Rc to avoid lifetime problems?",
        keywords: "Rc lifetimes",
        expected: "Lifetimes.md",
    },
    Question {
        text: "Why does the borrow checker complain about returning references?",
        keywords: "return reference lifetime",
        expected: "Lifetimes.md",
    },
    Question {
        text: "How do I find and fix a memory leak?",
        keywords: "memory leak",
        expected: "MemoryLeaks.md",
    },
    Question {
        text: "What is async/await?",
        keywords: "async await",
        expected: "AsyncAwaitOverview.md",
    },
    Question {
        text: "What is the difference between a sender and a receiver?",
        keywords: "sender receiver",
        expected: "Channels.md",
    },
    Question {
        text: "How do I stop threads from corrupting shared data?",
        keywords: "mutex",
        expected: "Mutexes.md",
    },
    Question {
        text: "How do I create a mutable variable in Rust?",
        keywords: "mutability shadowing",
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

// The reranker gets the heading and the body, without the E5 `passage: ` prefix -
// that prefix is an instruction to the embedding model, not part of the text.
fn rerank_text(chunk: &Chunk) -> String {
    if chunk.heading.is_empty() {
        chunk.body.clone()
    } else {
        format!("{}: {}", chunk.heading, chunk.body)
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

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn build_fts(chunks: &[Chunk]) -> Result<Connection> {
    let connection = Connection::open_in_memory()?;
    connection.execute(
        "CREATE VIRTUAL TABLE chunks_fts USING fts5(id UNINDEXED, path, heading, body)",
        [],
    )?;
    {
        let mut insert = connection
            .prepare("INSERT INTO chunks_fts (id, path, heading, body) VALUES (?1, ?2, ?3, ?4)")?;
        for (index, chunk) in chunks.iter().enumerate() {
            insert.execute((index as i64, &chunk.path, &chunk.heading, &chunk.body))?;
        }
    }
    Ok(connection)
}

fn bm25_chunk_ranking(connection: &Connection, query: &str) -> Result<Vec<usize>> {
    let fts = query_terms(query)
        .iter()
        .map(|term| format!("\"{term}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    if fts.is_empty() {
        return Ok(Vec::new());
    }

    let mut statement =
        connection.prepare("SELECT id FROM chunks_fts WHERE chunks_fts MATCH ?1 ORDER BY bm25(chunks_fts)")?;
    let rows = statement.query_map([fts], |row| row.get::<_, i64>(0))?;

    let mut ranked = Vec::new();
    for row in rows {
        ranked.push(row? as usize);
    }
    Ok(ranked)
}

fn semantic_chunk_ranking(question: &[f32], embeddings: &[Vec<f32>]) -> Vec<usize> {
    let mut scored = embeddings
        .iter()
        .enumerate()
        .map(|(index, embedding)| (index, cosine_similarity(question, embedding)))
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.1.partial_cmp(&left.1).unwrap());
    scored.into_iter().map(|(index, _)| index).collect()
}

fn reciprocal_rank_fusion(rankings: &[Vec<usize>], k: usize) -> Vec<usize> {
    let mut scores: HashMap<usize, f64> = HashMap::new();
    for ranking in rankings {
        for (position, index) in ranking.iter().enumerate() {
            let rank = position + 1;
            *scores.entry(*index).or_insert(0.0) += 1.0 / (k + rank) as f64;
        }
    }

    let mut fused = scores.into_iter().collect::<Vec<_>>();
    fused.sort_by(|left, right| right.1.partial_cmp(&left.1).unwrap());
    fused.into_iter().map(|(index, _)| index).collect()
}

// Walk a chunk ordering and keep the first time we see each file.
fn file_ranking(chunks: &[Chunk], order: &[usize]) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for index in order {
        let path = &chunks[*index].path;
        if !files.contains(path) {
            files.push(path.clone());
        }
    }
    files
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

fn describe(chunks: &[Chunk], order: &[usize], k: usize) -> String {
    order
        .iter()
        .take(k)
        .map(|index| {
            let chunk = &chunks[*index];
            if chunk.heading.is_empty() {
                chunk.path.clone()
            } else {
                format!("{} > {}", chunk.path, chunk.heading)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
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
    println!("Candidates per question: {CANDIDATES}");
    println!();

    let connection = build_fts(&chunks)?;

    let mut embedder = TextEmbedding::try_new(
        TextInitOptions::new(EMBED_MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;
    let mut reranker = TextRerank::try_new(
        RerankInitOptions::new(RERANK_MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the reranking model")?;

    let passages = chunks.iter().map(passage_text).collect::<Vec<_>>();
    let chunk_embeddings = embedder.embed(passages, None)?;

    let query_inputs = QUESTIONS
        .iter()
        .map(|question| format!("{QUERY_PREFIX}{}", question.text))
        .collect::<Vec<_>>();
    let question_embeddings = embedder.embed(query_inputs, None)?;

    let mut metrics = [Metrics::default(), Metrics::default(), Metrics::default()];

    println!(
        "{:<44} {:>9} {:>8} {:>8}",
        "Question", "semantic", "hybrid", "rerank"
    );
    for (index, question) in QUESTIONS.iter().enumerate() {
        // Stage one: cheap retrieval, fused into a candidate pool.
        let bm25 = bm25_chunk_ranking(&connection, question.keywords)?;
        let semantic = semantic_chunk_ranking(&question_embeddings[index], &chunk_embeddings);
        let fused = reciprocal_rank_fusion(&[bm25, semantic.clone()], RRF_K);
        let candidates = fused.iter().take(CANDIDATES).cloned().collect::<Vec<_>>();

        // Stage two: the cross-encoder reads each candidate against the query.
        let documents = candidates
            .iter()
            .map(|&chunk| rerank_text(&chunks[chunk]))
            .collect::<Vec<_>>();
        let reranked = reranker.rerank(question.text.to_string(), documents, false, None)?;
        let reranked_order = reranked
            .iter()
            .map(|result| candidates[result.index])
            .collect::<Vec<_>>();

        let rankings = [
            file_ranking(&chunks, &semantic),
            file_ranking(&chunks, &candidates),
            file_ranking(&chunks, &reranked_order),
        ];
        let mut cells = Vec::new();
        for (method, ranking) in rankings.iter().enumerate() {
            let rank = rank_of(ranking, question.expected);
            metrics[method].record(rank);
            cells.push(format_rank(rank));
        }

        println!(
            "{:<44} {:>9} {:>8} {:>8}",
            truncate(question.text, 44),
            cells[0],
            cells[1],
            cells[2]
        );
    }

    let total = QUESTIONS.len() as f64;
    println!();
    println!("{:<12} {:>6} {:>7} {:>7} {:>7}", "Method", "hit@1", "hit@3", "hit@5", "MRR");
    let names = ["semantic", "hybrid", "rerank"];
    for (name, metrics) in names.iter().zip(&metrics) {
        println!(
            "{:<12} {:>6.2} {:>7.2} {:>7.2} {:>7.2}",
            name,
            metrics.hit1 as f64 / total,
            metrics.hit3 as f64 / total,
            metrics.hit5 as f64 / total,
            metrics.reciprocal_rank / total
        );
    }

    // One question, with the candidate order and the reranked order shown.
    let question = &QUESTIONS[WORKED_EXAMPLE];
    let bm25 = bm25_chunk_ranking(&connection, question.keywords)?;
    let semantic = semantic_chunk_ranking(&question_embeddings[WORKED_EXAMPLE], &chunk_embeddings);
    let fused = reciprocal_rank_fusion(&[bm25, semantic], RRF_K);
    let candidates = fused.iter().take(CANDIDATES).cloned().collect::<Vec<_>>();
    let documents = candidates
        .iter()
        .map(|&chunk| rerank_text(&chunks[chunk]))
        .collect::<Vec<_>>();
    let reranked = reranker.rerank(question.text.to_string(), documents, false, None)?;
    let reranked_order = reranked
        .iter()
        .map(|result| candidates[result.index])
        .collect::<Vec<_>>();

    println!();
    println!("Worked example: {:?}", question.text);
    println!("  expected:   {}", question.expected);
    println!("  retrieval:  {}", describe(&chunks, &candidates, 5));
    println!("  reranked:   {}", describe(&chunks, &reranked_order, 5));

    Ok(())
}
