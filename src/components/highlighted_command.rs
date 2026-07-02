//! Semantic token coloring for shell install commands.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandTokenKind {
    Runner,
    Package,
    Url,
    Flag,
    Operator,
    Arg,
}

struct CommandToken {
    text: String,
    kind: CommandTokenKind,
}

fn token_kind(word: &str, prev_runner: bool) -> CommandTokenKind {
    let lower = word.to_lowercase();
    if matches!(
        lower.as_str(),
        "npx" | "npm" | "pnpm" | "yarn" | "pip" | "pip3" | "cargo" | "curl" | "wget" | "bun" | "go"
    ) {
        return CommandTokenKind::Runner;
    }
    if word.starts_with('-') {
        return CommandTokenKind::Flag;
    }
    if matches!(word, "|" | "&&" | ";" | "||" | ">" | "<" | ">>") {
        return CommandTokenKind::Operator;
    }
    if word.starts_with("http://")
        || word.starts_with("https://")
        || (word.contains('.') && !word.starts_with("./") && !word.starts_with("../"))
    {
        return CommandTokenKind::Url;
    }
    if prev_runner || word.starts_with('@') || word.contains('/') || word.contains(':') {
        return CommandTokenKind::Package;
    }
    CommandTokenKind::Arg
}

fn tokenize_command(command: &str) -> Vec<CommandToken> {
    let mut tokens = Vec::new();
    let mut prev_runner = false;
    for word in command.split_whitespace() {
        let kind = token_kind(word, prev_runner);
        prev_runner = kind == CommandTokenKind::Runner;
        tokens.push(CommandToken {
            text: word.to_string(),
            kind,
        });
    }
    tokens
}

fn token_class(kind: CommandTokenKind) -> &'static str {
    match kind {
        CommandTokenKind::Runner => "cmd-token-runner",
        CommandTokenKind::Package => "cmd-token-package",
        CommandTokenKind::Url => "cmd-token-url",
        CommandTokenKind::Flag => "cmd-token-flag",
        CommandTokenKind::Operator => "cmd-token-op",
        CommandTokenKind::Arg => "cmd-token-arg",
    }
}

#[component]
pub fn HighlightedCommand(
    text: String,
    #[prop(default = true)] show_prefix: bool,
    #[prop(default = "install-cmd")] class: &'static str,
) -> impl IntoView {
    let tokens = tokenize_command(&text);
    view! {
        <code class=class>
            {show_prefix.then(|| view! { <span class="install-prefix">"$ "</span> })}
            {tokens.into_iter().enumerate().map(|(idx, token)| {
                view! {
                    {(idx > 0).then_some(" ")}
                    <span class=token_class(token.kind)>{token.text}</span>
                }
            }).collect_view()}
        </code>
    }
}

#[cfg(test)]
mod tests {
    use super::{tokenize_command, CommandTokenKind};

    #[test]
    fn tokenizes_mcp_remote_command() {
        let tokens = tokenize_command("npx mcp-remote www.onchain-ai.xyz/mcp");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, CommandTokenKind::Runner);
        assert_eq!(tokens[1].kind, CommandTokenKind::Package);
        assert_eq!(tokens[2].kind, CommandTokenKind::Url);
    }

    #[test]
    fn tokenizes_scoped_package() {
        let tokens = tokenize_command("npm i @uniswap/mcp");
        assert_eq!(tokens[0].kind, CommandTokenKind::Runner);
        assert!(tokens
            .iter()
            .any(|t| t.text == "@uniswap/mcp" && t.kind == CommandTokenKind::Package));
    }
}
