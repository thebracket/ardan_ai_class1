use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

/// The team a support ticket should be routed to. This is *our* type. Jev is
/// constrained to answer with one of the labels we give it, so it can never
/// invent a category we don't have a branch for.
#[derive(Debug)]
enum Team {
    Returns,
    Shipping,
    Billing,
    Other,
}

impl Team {
    fn from_label(label: &str) -> Team {
        match label {
            "returns" => Team::Returns,
            "shipping" => Team::Shipping,
            "billing" => Team::Billing,
            _ => Team::Other,
        }
    }
}

#[derive(Deserialize, Debug)]
struct SystemOneResponse {
    model: String,
    answers: Answers,
    usage: Usage,
}

/// Jev answers each question under the id we chose. We asked two questions, so
/// there are two typed answers - a Choice and a Noul. There is no prose to
/// parse and no retry loop to write.
#[derive(Deserialize, Debug)]
struct Answers {
    department: ChoiceAnswer,
    wants_human: NoulAnswer,
}

#[derive(Deserialize, Debug)]
struct ChoiceAnswer {
    choice: String,
    confidence: f32,
    probabilities: HashMap<String, f32>,
}

#[derive(Deserialize, Debug)]
struct NoulAnswer {
    noul: f32,
}

#[derive(Deserialize, Debug)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
    // OpenRouter reports the cost of the call; TypeSafe's own endpoint doesn't,
    // so this is optional.
    cost: Option<f64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Jev is served on OpenRouter, so we use the same OPENROUTER_KEY as every
    // other example - no separate TypeSafe account needed.
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // The ticket to triage. In a real system this would come from a queue; here
    // you can pass it on the command line, otherwise we use a clear-cut default.
    let ticket = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let ticket = if ticket.trim().is_empty() {
        "I've been charged twice for my subscription this month, and I have \
         asked three times now - can I please just talk to a real person?"
            .to_string()
    } else {
        ticket
    };

    // Try a murkier ticket to watch the confidence gate fire instead, e.g. one
    // that mentions a wrong size *and* duplicate charges, where no single team
    // is obviously right.
    println!("ticket: {ticket}");

    // The request. Notice how much of the "prompting" lives in the *structure*
    // rather than in free text:
    //
    // * Each question is atomic - one judgement, not several bundled together.
    // * A Choice gives every option a description, so the model can tell them
    //   apart. There is always an "other", so it is never forced to guess.
    // * The Noul is phrased so that a high value means "yes".
    // * Both questions go in one call; Jev evaluates them in parallel.
    //
    // The question ids ("department", "wants_human") are ours - the model never
    // sees them. They're just the keys the answers come back under.
    let request = json!({
        "model": "typesafe/jev-1.13",
        "state": ticket,
        "questions": {
            "department": {
                "type": "choice",
                "instructions": "Which team should handle this?",
                "criteria": {
                    "returns": "Exchanges, wrong or damaged items",
                    "shipping": "Delivery status, delays, lost packages",
                    "billing": "Charges, invoices, payment problems",
                    "other": "Fits none of the above"
                }
            },
            "wants_human": {
                "type": "noul",
                "instructions": "Is the customer asking to speak to a human?",
                "criteria": {
                    "true": "Asks for a person, an agent, or someone real",
                    "false": "No request to talk to a person"
                }
            }
        }
    });

    // OpenRouter's System One endpoint takes the same request shape as
    // TypeSafe's own /v1/systemone, so only the host and the key change. (The
    // alpha /api/alpha/decisions endpoint works too.)
    let response = reqwest::Client::new()
        .post("https://openrouter.ai/api/v1/systemone")
        .bearer_auth(&api_key)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Jev returned {}: {}",
            response.status(),
            response.text().await?
        );
    }

    // Because the output is typed, it deserializes straight into our structs.
    // No chance of a Markdown fence, trailing advice, or an invented label.
    let response: SystemOneResponse = response.json().await?;

    // Everything below is ordinary, deterministic code. The model gave us
    // probabilities; *we* decide what to do with them.
    let cost = response
        .usage
        .cost
        .map_or("n/a".to_string(), |cost| format!("${cost:.6}"));
    println!(
        "model: {} ({} in / {} out tokens, cost {})",
        response.model, response.usage.input_tokens, response.usage.output_tokens, cost
    );

    let department = &response.answers.department;
    println!(
        "\ndepartment: {} (confidence {:.2})",
        department.choice, department.confidence
    );
    let mut probabilities: Vec<(&String, &f32)> = department.probabilities.iter().collect();
    probabilities.sort_by(|a, b| b.1.total_cmp(a.1));
    for (label, probability) in probabilities {
        println!("  {label}: {probability:.2}");
    }

    let wants_human = response.answers.wants_human.noul;
    println!("\nwants_human: {wants_human:.2}");

    // A low-confidence answer is a reason to ask a person, not to guess. This
    // is the "route the uncertain tail to a human" pattern.
    if department.confidence < 0.5 {
        println!("\n-> confidence too low; sending to manual triage");
        return Ok(());
    }

    match Team::from_label(&department.choice) {
        Team::Returns => println!("\n-> route to the returns team"),
        Team::Shipping => println!("\n-> route to the shipping team"),
        Team::Billing => println!("\n-> route to the billing team"),
        Team::Other => println!("\n-> route to general support"),
    }

    // A Noul is a probability, not a boolean. The threshold is ours to choose:
    // act only when we are sure, and let the uncertain middle go to a person.
    if wants_human > 0.8 {
        println!("-> customer asked for a human; escalate to an agent");
    } else if wants_human > 0.2 {
        println!("-> unclear whether they want a human; flag for review");
    }

    Ok(())
}
