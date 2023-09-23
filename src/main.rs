use std::{error::Error, io::stdout, fmt::Display, fs};

use async_openai::{
    types::{
        ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role,
    },
    Client,
};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use std::io::Write;

#[derive(Parser, Debug)]
enum Commands {
    /// Create a new template
    CreateTemplate {
        template_name: String,
        content: String,
    },
    /// Ask a question
    Ask {
        question: String,
        #[clap(short, long)]
        template: Option<String>,
    },
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

async fn query_chat(question: String, system_prompt: String, model: String) -> Result<(), Box<dyn Error>> {    
    let client = Client::new();
    
    let request = CreateChatCompletionRequestArgs::default()
        .model(&model)
        .max_tokens(512u16)
        .messages([ChatCompletionRequestMessageArgs::default().content(system_prompt).role(Role::System).build()?,ChatCompletionRequestMessageArgs::default()
            .content(question)
            .role(Role::User)
            .build()?])
        .build()?;

    let mut stream = client.chat().create_stream(request).await?;

    // From Rust docs on print: https://doc.rust-lang.org/std/macro.print.html
    //
    //  Note that stdout is frequently line-buffered by default so it may be necessary
    //  to use io::stdout().flush() to ensure the output is emitted immediately.
    //
    //  The print! macro will lock the standard output on each call.
    //  If you call print! within a hot loop, this behavior may be the bottleneck of the loop.
    //  To avoid this, lock stdout with io::stdout().lock():

    let mut lock = stdout().lock();
    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
                    if let Some(ref content) = chat_choice.delta.content {
                        write!(lock, "{}", content).unwrap();
                    }
                });
            }
            Err(err) => {
                writeln!(lock, "error: {err}").unwrap();
            }
        }
        stdout().flush()?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ask{question, template} => {
            let system_prompt = if let Some(template_name) = template {
                fs::read_to_string(format!("templates/{template_name}.txt"))?
            }
            else {
                "You are an AI assistant running as CLI tool.".into()
            };
            query_chat(question, system_prompt, "gpt-3.5-turbo".into()).await?;
        },
        Commands::CreateTemplate { template_name, content } => {
            let target_dir = dirs::home_dir().unwrap().join(".ai-cli-assistant/templates/");
            if !target_dir.exists() {
                fs::create_dir_all(target_dir.clone())?;
            }

            let template_path = target_dir.join(format!("{template_name}.txt"));
            fs::write(template_path, content)?;
            println!("Template created successfuly!");
        }
    }

    Ok(())
}
