mod chatbot;

use anyhow::{bail, Context, Result};
use chatbot::{ChatSession, ModelReply, ToolDefinition, ToolFactory};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use std::fs;
use std::path::{Path, PathBuf};

const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;

// E5 wants a different prefix for the thing you search *with* and the things
// you search *over*.
const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";

const TOP_K: usize = 5;
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

const SYSTEM_PROMPT: &str = "You are a helpful assistant for the Ultimate Rust \
Foundations class notes. Use the read_docs tool to look things up before you \
answer a question about the notes. Answer using only the passages the tool \
returns, and name the source file and heading you used. If the passages do not \
contain the answer, say so rather than guessing.";

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

// The retrieval half of RAG: an embedding model, the chunks, and their vectors.
// It is built once at startup and then just answers queries.
struct Docs {
    model: TextEmbedding,
    chunks: Vec<Chunk>,
    embeddings: Vec<Vec<f32>>,
}

impl Docs {
    fn search(&mut self, query: &str) -> Result<Vec<(f64, usize)>> {
        let embedding = self
            .model
            .embed(vec![format!("{QUERY_PREFIX}{query}")], None)?
            .remove(0);

        let mut scored = self
            .embeddings
            .iter()
            .enumerate()
            .map(|(index, chunk_embedding)| (cosine_similarity(&embedding, chunk_embedding), index))
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
        scored.truncate(TOP_K);
        Ok(scored)
    }
}

impl ToolFactory for Docs {
    async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String) {
        match tool_name {
            "read_docs" => {
                let query = serde_json::from_str::<serde_json::Value>(arguments)
                    .ok()
                    .and_then(|args| args.get("query")?.as_str().map(str::to_string))
                    .unwrap_or_default();

                match self.search(&query) {
                    Ok(hits) => ("results".to_string(), format_results(&self.chunks, &hits)),
                    Err(error) => (String::from("error"), error.to_string()),
                }
            }
            _ => (tool_name.to_string(), "Unknown tool".to_string()),
        }
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "read_docs".to_string(),
            description: "Search the Ultimate Rust Foundations class notes and return the most \
                          relevant passages, each with its source file and heading."
                .to_string(),
            parameters: vec![("query".to_string(), "string".to_string())],
            required: vec!["query".to_string()],
        }]
    }
}

fn format_results(chunks: &[Chunk], hits: &[(f64, usize)]) -> String {
    let mut output = String::new();
    for (rank, (score, index)) in hits.iter().enumerate() {
        let chunk = &chunks[*index];
        let heading = if chunk.heading.is_empty() {
            "Introduction"
        } else {
            &chunk.heading
        };
        output.push_str(&format!(
            "[{}] {} > {} (score {:.3})\n{}\n\n",
            rank + 1,
            chunk.path,
            heading,
            score,
            chunk.body
        ));
    }
    output
}

fn readln() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Build the corpus once, before the chat starts. Embedding is synchronous
    // and can be slow, but it only happens here - not on every question.
    println!("Building corpus...");
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
    let mut chunks = Vec::new();
    for document in &documents {
        chunks.extend(chunk_markdown(&document.path, &document.body));
    }

    let mut model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;

    let passages = chunks.iter().map(passage_text).collect::<Vec<_>>();
    let embeddings = model.embed(passages, None).context("embedding corpus failed")?;
    println!(
        "Corpus ready: {} chunks from {} files",
        chunks.len(),
        documents.len()
    );

    let docs = Docs {
        model,
        chunks,
        embeddings,
    };

    let mut session = ChatSession::new(api_key, "deepseek/deepseek-v4.1-flash", SYSTEM_PROMPT, docs)?;

    // Spawn a consumer for the streamed replies. The SSE stream itself is not
    // `Send`, so it is awaited in the foreground below rather than spawned.
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        use colored::Colorize;
        while let Some(message) = rx.recv().await {
            match message {
                ModelReply::Reasoning(text) => print!("{}", &text.green()),
                ModelReply::Content(text) => print!("{}", &text.white()),
            }
        }
    });

    loop {
        println!("Ask a question about the notes, or /quit to quit");
        let input = readln()?;
        if input == "/quit" {
            break;
        }

        session.send_prompt(input, tx.clone()).await?;
        println!();
    }

    Ok(())
}
