# rcommit

rcommit is a command-line tool built in Rust that leverages AI (specifically, OpenAI's GPT models) to generate conventional commit messages based on the changes made in a git repository. It provides an easy and intuitive way to create meaningful commit messages by analyzing the context and content of your changes.

## Features

- Generate commit messages using AI.
- Customizable context for better understanding of changes.
- Exclude specific files from consideration.
- Easy integration into existing git workflows.

## Prerequisites

Before you begin, ensure you have met the following requirements:

- Rust programming environment (Cargo and Rust compiler).
- Git installed on your system.
- An OpenAI API key (for the AI functionality).

Add the OpenAI key as env variabe

```bash
export OPENAI_API_KEY={{key}}
```

## Installation

To install rcommit, follow these steps:

1. Clone the repository:

```bash
git clone https://github.com/Abraxas-365/rcommit.git

cd rcommit

cargo build --release

./target/release/rcommit
```

## Arguments

-c, --context: Sets a custom context for the commit message.
-e, --exclude: List of files to exclude from the git diff.

```bash
./target/release/rcommit -c "Feature addition" -e "README.md" "LICENSE"
```
