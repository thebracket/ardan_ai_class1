use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use nalgebra::{DMatrix, DVector, SymmetricEigen};
use plotters::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

// The E5 family is trained to see two prefixes: "query: " for the thing you are
// searching for, and "passage: " for the things you search over. We're only
// making a picture here, so everything is a query.
const MODEL_INPUT_PREFIX: &str = "query: ";

const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordGroup {
    Cat,
    Dog,
    House,
    Greeting,
}

impl WordGroup {
    fn color(self) -> RGBColor {
        match self {
            Self::Cat => RGBColor(220, 20, 60),
            Self::Dog => RGBColor(30, 144, 255),
            Self::House => RGBColor(34, 139, 34),
            Self::Greeting => RGBColor(255, 140, 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    text: &'static str,
    language: &'static str,
    group: WordGroup,
}

fn samples() -> Vec<Sample> {
    let mut samples = Vec::new();
    // Four ideas, each expressed in four languages. If embeddings work the way
    // we hope, the language matters less than the meaning.
    for (group, words) in [
        (WordGroup::Cat, ["cat", "gato", "chat", "猫"]),
        (WordGroup::Dog, ["dog", "perro", "chien", "犬"]),
        (WordGroup::House, ["house", "casa", "maison", "家"]),
        (
            WordGroup::Greeting,
            ["good morning", "buenos días", "bonjour", "おはよう"],
        ),
    ] {
        for text in words {
            samples.push(Sample {
                text,
                language: language_of(text),
                group,
            });
        }
    }
    samples
}

fn language_of(text: &str) -> &'static str {
    match text {
        "cat" | "dog" | "house" | "good morning" => "English",
        "gato" | "perro" | "casa" | "buenos días" => "Spanish",
        "chat" | "chien" | "maison" | "bonjour" => "French",
        _ => "Japanese",
    }
}

#[derive(Debug)]
struct Point {
    sample: Sample,
    embedding: Vec<f32>,
    x: f64,
    y: f64,
}

fn model_input(text: &str) -> String {
    format!("{MODEL_INPUT_PREFIX}{text}")
}

fn cache_dir() -> PathBuf {
    // The model is only downloaded once, but it is a chunk of disk. Keep it in
    // the OS cache directory rather than in the repository.
    if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("ai_workshop").join("embeddings");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".cache")
        .join("ai_workshop")
        .join("embeddings")
}

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output")
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum();
    let left_length: f64 = a
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    let right_length: f64 = b
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    dot / (left_length * right_length)
}

// A 384-dimensional vector is no use to an eyeball, so we squash it down to two
// dimensions with PCA. Nearby points on the plot are usually related, but the
// distances are distorted - this is a teaching aid, not the truth.
fn project_pca_2d(vectors: &[Vec<f32>]) -> Vec<(f64, f64)> {
    let rows = vectors.len();
    let cols = vectors[0].len();

    let data = vectors
        .iter()
        .flat_map(|vector| vector.iter().map(|value| f64::from(*value)))
        .collect::<Vec<_>>();
    let mut matrix = DMatrix::from_row_slice(rows, cols, &data);

    // Centre each column on its mean before asking which directions vary most.
    let mut mean = DVector::zeros(cols);
    for col in 0..cols {
        let column_sum: f64 = (0..rows).map(|row| matrix[(row, col)]).sum();
        mean[col] = column_sum / rows as f64;
    }
    for row in 0..rows {
        for col in 0..cols {
            matrix[(row, col)] -= mean[col];
        }
    }

    let covariance = (&matrix.transpose() * &matrix) / (rows as f64 - 1.0);
    let eigen = SymmetricEigen::new(covariance);

    // The eigenvalues tell us which directions carry the most variance; we keep
    // the two biggest, which become our x and y axes.
    let mut indices = (0..eigen.eigenvalues.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        eigen.eigenvalues[*right]
            .partial_cmp(&eigen.eigenvalues[*left])
            .unwrap()
    });

    let components = DMatrix::from_columns(&[
        eigen.eigenvectors.column(indices[0]).into_owned(),
        eigen.eigenvectors.column(indices[1]).into_owned(),
    ]);

    let projected = matrix * components;
    (0..rows)
        .map(|row| (projected[(row, 0)], projected[(row, 1)]))
        .collect()
}

fn write_plot(path: &Path, points: &[Point]) -> Result<()> {
    let root = SVGBackend::new(path, (960, 720)).into_drawing_area();
    root.fill(&WHITE)?;

    let xs = points.iter().map(|point| point.x).collect::<Vec<_>>();
    let ys = points.iter().map(|point| point.y).collect::<Vec<_>>();
    let x_min = xs.iter().copied().fold(f64::INFINITY, f64::min) - 0.2;
    let x_max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.2;
    let y_min = ys.iter().copied().fold(f64::INFINITY, f64::min) - 0.2;
    let y_max = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.2;

    let mut chart = ChartBuilder::on(&root)
        .caption("Similar Meanings Land Near Each Other", ("sans-serif", 28))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)?;

    chart
        .configure_mesh()
        .x_desc("PCA axis 1")
        .y_desc("PCA axis 2")
        .light_line_style(WHITE.mix(0.0))
        .draw()?;

    for point in points {
        chart.draw_series(std::iter::once(Circle::new(
            (point.x, point.y),
            6,
            point.sample.group.color().filled(),
        )))?;
        chart.draw_series(std::iter::once(Text::new(
            format!("{} ({})", point.sample.text, point.sample.language),
            (point.x + 0.02, point.y + 0.02),
            ("sans-serif", 16).into_font().color(&BLACK),
        )))?;
    }

    root.present()?;
    Ok(())
}

fn print_tokenization(model: &TextEmbedding, samples: &[Sample], model_inputs: &[String]) {
    println!("Tokenization examples:");
    for (sample, model_input) in samples.iter().zip(model_inputs).take(4) {
        let encoding = match model.tokenizer.encode(model_input.clone(), true) {
            Ok(encoding) => encoding,
            Err(error) => {
                println!("  (failed to tokenize {model_input:?}: {error})");
                continue;
            }
        };
        println!("  {} ({})", sample.text, sample.language);
        println!("    model input: {model_input:?}");
        println!("    token ids: {:?}", encoding.get_ids());
        println!("    tokens: {:?}", encoding.get_tokens());
    }
}

fn print_similarity_table(points: &[Point]) {
    let pairs = [
        ("cat", "gato"),
        ("cat", "chat"),
        ("dog", "perro"),
        ("house", "casa"),
        ("good morning", "buenos días"),
        ("cat", "house"),
        ("house", "chien"),
        ("good morning", "犬"),
    ];

    println!("Selected cosine similarities:");
    for (left_text, right_text) in pairs {
        let find = |text: &str| {
            points
                .iter()
                .find(|point| point.sample.text == text)
                .expect("sample is always present")
        };
        println!(
            "  {:<14} <-> {:<14} {:.4}",
            left_text,
            right_text,
            cosine_similarity(&find(left_text).embedding, &find(right_text).embedding)
        );
    }
}

fn main() -> Result<()> {
    let output_dir = output_dir();
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    // The first run downloads the model into the cache; later runs are offline.
    let mut model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir())
            .with_show_download_progress(true),
    )
    .context("failed to load the embedding model")?;

    let samples = samples();
    let model_inputs = samples
        .iter()
        .map(|sample| model_input(sample.text))
        .collect::<Vec<_>>();
    let embeddings = model
        .embed(model_inputs.clone(), None)
        .context("embedding generation failed")?;

    let coordinates = project_pca_2d(&embeddings);
    let points = samples
        .iter()
        .zip(embeddings)
        .zip(coordinates)
        .map(|((sample, embedding), (x, y))| Point {
            sample: *sample,
            embedding,
            x,
            y,
        })
        .collect::<Vec<_>>();

    println!("Model: {MODEL:?}");
    println!("Embedding dimension: {}", points[0].embedding.len());
    println!("Model input prefix: {MODEL_INPUT_PREFIX:?}");
    println!();

    print_tokenization(&model, &samples, &model_inputs);
    println!();
    print_similarity_table(&points);

    let plot_path = output_dir.join("embeddings.svg");
    write_plot(&plot_path, &points)?;
    println!();
    println!("Plot written to {}", plot_path.display());

    Ok(())
}
