use std::io::{self, BufRead};
use std::process::{Command, Stdio};

use clap::{App, Arg};
use clipboard::{ClipboardContext, ClipboardProvider};
use langchain_rust::chain::chain_trait::Chain;
use langchain_rust::chain::llm_chain::LLMChainBuilder;
use langchain_rust::llm::openai::{OpenAI, OpenAIModel};
use langchain_rust::prompt::HumanMessagePromptTemplate;
use langchain_rust::{prompt_args, template_jinja2};

#[tokio::main] // This attribute makes your main function asynchronous
async fn main() -> io::Result<()> {
    let matches = App::new("rcommit")
        .version("0.1.0")
        .author("Luis Fernando Miranda")
        .about("Auses AI to Write Commit messages")
        .arg(
            Arg::new("context")
                .short('c')
                .long("context")
                .takes_value(true)
                .default_value("no context")
                .help("Sets a custom context"),
        )
        .arg(
            Arg::new("exclude")
                .short('e')
                .long("exclude")
                .takes_value(true)
                .multiple_values(true)
                .help("List of files to exclude from the git diff"),
        )
        .arg(
            Arg::new("model")
                .short('m')
                .long("model")
                .takes_value(true)
                .possible_values(&["gpt3.5", "gpt4", "gpt4-turbo"])
                .default_value("gpt4")
                .help("Specifies the OpenAI model to use"),
        )
        .get_matches();

    let context = matches.value_of("context").unwrap_or("no context");
    let model_arg = matches.value_of("model").unwrap_or("gpt3.5");
    let model = match model_arg {
        "gpt3.5" => OpenAIModel::Gpt35,
        "gpt4" => OpenAIModel::Gpt4,
        "gpt4-turbo" => OpenAIModel::Gpt4Turbo,
        _ => panic!("Invalid model specified"), // This should never happen due to clap's possible_values constraint
    };
    let excludes = matches
        .values_of("exclude")
        .unwrap_or_default()
        .collect::<Vec<&str>>();

    let exclude_pattern = excludes.iter().fold(String::new(), |acc, &file| {
        if acc.is_empty() {
            format!("grep -vE '^{}$'", file)
        } else {
            format!("{} | grep -vE '^{}$'", acc, file)
        }
    });

    let shell_command = if exclude_pattern.is_empty() {
        "git diff --cached --name-only --diff-filter=ACM | while read -r file; do echo \"\\n---------------------------\\n name:$file\"; git diff --cached \"$file\" | sed 's/^/+/'; done".to_string()
    } else {
        format!(
            "git diff --cached --name-only --diff-filter=ACM | {} | while read -r file; do echo \"\\n---------------------------\\n name:$file\"; git diff --cached \"$file\" | sed 's/^/+/'; done",
            exclude_pattern
        )
    };
    let llm = OpenAI::default().with_model(model);
    let chain = LLMChainBuilder::new()
        .prompt(HumanMessagePromptTemplate::new(template_jinja2!(
            r#"
    Create a conventional commit message for the following changes.

    Some context about the changes: {{context}}

    File changes: 
        {{input}}



    "#,
            "input",
            "context"
        )))
        .llm(llm)
        .build()
        .expect("Failed to build LLMChain");

    let output = Command::new("sh")
        .arg("-c")
        .arg(shell_command)
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not capture stdout."))?;

    let reader = io::BufReader::new(output);

    let complete_changes = reader
        .lines()
        .map(|line| line.unwrap())
        .collect::<Vec<String>>()
        .join("\n");

    let res = chain
        .invoke(prompt_args! {
            "input"=>complete_changes,
            "context"=>context
        })
        .await
        .expect("Failed to invoke chain");

    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    ctx.set_contents(res).unwrap();

    Ok(())
}
