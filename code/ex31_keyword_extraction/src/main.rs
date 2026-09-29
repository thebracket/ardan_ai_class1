use anyhow::{bail, Context, Result};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

// A small, cheap model is plenty for "list some keywords". Swap it with the
// KEYWORD_MODEL environment variable to try others.
const DEFAULT_KEYWORD_MODEL: &str = "meta-llama/llama-3.2-3b-instruct";
const KEYWORD_MODEL_ENV: &str = "KEYWORD_MODEL";
const CORPUS_ENV: &str = "RUST_NOTES_DIR";

const MAX_KEYWORDS: usize = 5;

struct Chunk {
    path: String,
    heading: String,
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

// Models wrap JSON in prose or a code fence often enough that it is worth
// digging the array out rather than trusting the whole reply to parse.
fn parse_keywords(raw: &str) -> Vec<String> {
    let (Some(start), Some(end)) = (raw.find('['), raw.rfind(']')) else {
        return Vec::new();
    };
    if end < start {
        return Vec::new();
    }

    let parsed = serde_json::from_str::<Vec<String>>(&raw[start..=end]).unwrap_or_default();
    let mut keywords: Vec<String> = Vec::new();
    for keyword in parsed {
        let keyword = keyword.trim().to_lowercase();
        if !keyword.is_empty() && !keywords.contains(&keyword) {
            keywords.push(keyword);
        }
    }
    keywords.truncate(MAX_KEYWORDS);
    keywords
}

fn shortened(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.to_string()
    } else {
        text.chars().take(limit).collect()
    }
}

async fn extract_keywords(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    text: &str,
) -> Result<Vec<String>> {
    // The whole trick: one simple instruction, and a formatted reply we can
    // parse. No schema, no fine-tuning, no ceremony.
    let prompt = format!(
        "Extract 1 to {MAX_KEYWORDS} relevant keywords from the following text. \
         Reply with ONLY a JSON array of short, lowercase strings.\n\n\
         text: {}",
        shortened(text, 1500)
    );

    let response = client
        .post(URL)
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 120,
            "temperature": 0.0
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        bail!("keyword model returned {}", response.status());
    }

    let body: serde_json::Value = response.json().await?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default();
    Ok(parse_keywords(content))
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;
    let model =
        std::env::var(KEYWORD_MODEL_ENV).unwrap_or_else(|_| DEFAULT_KEYWORD_MODEL.to_string());
    let client = reqwest::Client::new();

    // With an argument, treat it as the text to keyword. Otherwise, keyword
    // every chunk of the corpus - the ingest-time use.
    let argument = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if !argument.trim().is_empty() {
        let keywords = extract_keywords(&client, &api_key, &model, &argument).await?;
        println!("{}", keywords.join(", "));
        return Ok(());
    }

    let root = corpus_root()?;
    if !root.is_dir() {
        bail!(
            "corpus directory {} does not exist - set {CORPUS_ENV} to a checkout of the notes",
            root.display()
        );
    }
    let root = fs::canonicalize(&root).unwrap_or(root);

    let chunks = load_chunks(&root)?;
    println!("Corpus: {} chunks from {}", chunks.len(), root.display());
    println!("Keyword model: {model}");
    println!();

    for (index, chunk) in chunks.iter().enumerate() {
        let label = if chunk.heading.is_empty() {
            chunk.path.clone()
        } else {
            format!("{} > {}", chunk.path, chunk.heading)
        };

        match extract_keywords(&client, &api_key, &model, &chunk.body).await {
            Ok(keywords) if !keywords.is_empty() => println!("{label}\n  {}", keywords.join(", ")),
            Ok(_) => println!("{label}\n  (no keywords)"),
            Err(error) => println!("{label}\n  (failed: {error})"),
        }

        if (index + 1) % 10 == 0 {
            println!("... {} / {}", index + 1, chunks.len());
        }
    }

    Ok(())
}
