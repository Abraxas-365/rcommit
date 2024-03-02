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
    let matches = initialize_command_line_interface();
    let context = matches.value_of("context").unwrap_or("no context");
    let model = parse_model_argument(matches.value_of("model").unwrap_or("gpt3.5"));
    let exclude_patterns = matches
        .values_of("exclude")
        .unwrap_or_default()
        .collect::<Vec<&str>>();
    let git_diff_output = execute_git_diff_command(&exclude_patterns)?;
    let commit_message = generate_commit_message(&git_diff_output, &context, model).await;
    let formatter = format!("git commit -m \"{}\"", commit_message.replace("\"", "\\\""));
    if matches.is_present("git") {
        copy_to_clipboard(&formatter).expect("Could not copy to clipboard");
    } else {
        copy_to_clipboard(&commit_message).expect("Could not copy to clipboard");
    }

    Ok(())
}

fn initialize_command_line_interface() -> clap::ArgMatches {
    App::new("rcommit")
        .version("0.1.0")
        .author("Luis Fernando Miranda")
        .about("Uses AI to write commit messages")
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
                .default_value("gpt3.5")
                .help("Specifies the OpenAI model to use"),
        )
        .arg(
            Arg::new("git")
                .short('g')
                .long("git")
                .takes_value(false)
                .help("Saves the formatted commit message to the clipboard"),
        )
        .get_matches()
}

fn parse_model_argument(model_arg: &str) -> OpenAIModel {
    match model_arg {
        "gpt3.5" => OpenAIModel::Gpt35,
        "gpt4" => OpenAIModel::Gpt4,
        "gpt4-turbo" => OpenAIModel::Gpt4Turbo,
        _ => unreachable!("Invalid model specified"), // clap's possible_values constraint prevents reaching here
    }
}

fn execute_git_diff_command(excludes: &[&str]) -> io::Result<String> {
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

    let output = Command::new("sh")
        .arg("-c")
        .arg(shell_command)
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not capture stdout."))?;

    let reader = io::BufReader::new(output);
    reader
        .lines()
        .collect::<Result<Vec<String>, _>>()
        .map(|lines| lines.join("\n"))
}

async fn generate_commit_message(
    git_diff_output: &str,
    context: &str,
    model: OpenAIModel,
) -> String {
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
        .expect("Could not build LLM chain");

    chain
        .invoke(prompt_args! {
            "input" => git_diff_output,
            "context" => context
        })
        .await
        .expect("Error invoking LLMChain")
}

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()?;
    ctx.set_contents(text.to_owned())?;
    Ok(())
}
