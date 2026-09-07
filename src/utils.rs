use convert_case::{Boundary, Converter};

pub const VARIABLE_PREFIX: &str = "argc_";
pub const BEFORE_HOOK: &str = "_argc_before";
pub const AFTER_HOOK: &str = "_argc_after";
pub const ROOT_NAME: &str = "prog";
pub const MAIN_NAME: &str = "main";

pub(crate) const META_VERSION: &str = "version";
pub(crate) const META_BINNAME: &str = "binname";
pub(crate) const META_DOTENV: &str = "dotenv";
pub(crate) const META_DEFAULT_SUBCOMMAND: &str = "default-subcommand";
pub(crate) const META_INHERIT_FLAG_OPTIONS: &str = "inherit-flag-options";
pub(crate) const META_SYMBOL: &str = "symbol";
pub(crate) const META_COMBINE_SHORTS: &str = "combine-shorts";
pub(crate) const META_EXTERNAL_SUBCOMMANDS: &str = "external-subcommands";
pub(crate) const META_MAN_SECTION: &str = "man-section";
pub(crate) const META_REQUIRE_TOOLS: &str = "require-tools";

pub(crate) const MAX_ARGS: usize = 32767;

#[cfg(any(feature = "build", feature = "eval-bash"))]
pub const ARGC_REQUIRE_TOOLS: &str = include_str!("template/require_tools.sh");

#[cfg(any(feature = "build", feature = "eval-bash"))]
pub const ARGC_REQUIRE_PARAMS: &str = include_str!("template/require_params.sh");

#[cfg(any(feature = "build", feature = "eval-bash"))]
pub const ARGC_LOAD_DOTENV: &str = include_str!("template/load_dotenv.sh");

#[cfg(feature = "build")]
pub const ARGC_EXPAND_SHELL_VALUE: &str = include_str!("template/expand_shell_value.sh");

pub fn to_cobol_case(value: &str) -> String {
    Converter::new()
        .set_pattern(convert_case::Pattern::Uppercase)
        .set_delimiter("-")
        .set_boundaries(&[Boundary::Underscore, Boundary::LowerUpper, Boundary::Hyphen])
        .convert(value)
}

pub fn escape_shell_words(value: &str) -> String {
    shell_words::quote(value).to_string()
}

pub fn is_quote_char(c: char) -> bool {
    c == '\'' || c == '"'
}

pub fn unbalance_quote(value: &str) -> Option<(char, usize)> {
    let mut balance = None;
    for (i, c) in value.chars().enumerate() {
        match balance {
            Some((c_, _)) => {
                if c == c_ {
                    balance = None
                }
            }
            None => {
                if is_quote_char(c) {
                    balance = Some((c, i))
                }
            }
        }
    }
    balance
}

pub fn is_windows_path(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    ('a'..='z').any(|v| {
        if value.len() == 2 {
            value == format!("{v}:")
        } else {
            value.starts_with(&format!("{v}:/"))
        }
    })
}

pub fn is_special_var_char(c: char) -> bool {
    matches!(c, '-' | '.' | ':' | '@')
}

pub fn sanitize_var_name(id: &str) -> String {
    id.replace(is_special_var_char, "_")
}

pub fn argc_var_name(id: &str) -> String {
    format!("{VARIABLE_PREFIX}{}", sanitize_var_name(id))
}

pub fn is_true_value(value: &str) -> bool {
    matches!(value, "true" | "1")
}

/// Expand the shell forms an `@env` default is written in: a leading `~`, and
/// `$NAME`, `${NAME}`, `${NAME:-fallback}` or `${NAME-fallback}` anywhere.
///
/// The value is emitted single-quoted, so the shell never sees these forms.
/// A default that names a path has to be written with them all the same, since
/// no other spelling of `$XDG_STATE_HOME` works on two machines, and expanding
/// here is what keeps the two facts from contradicting each other. Nothing else
/// a shell does to a word — splitting, globbing, command substitution — is
/// expanded, and an undefined variable with no fallback becomes empty, as it
/// does in a shell.
pub fn expand_shell_value(value: &str, get_env: &dyn Fn(&str) -> Option<String>) -> String {
    let mut output = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;

    if chars.first() == Some(&'~') && matches!(chars.get(1), None | Some('/')) {
        if let Some(home) = get_env("HOME") {
            output.push_str(&home);
            i = 1;
        }
    }

    while i < chars.len() {
        if chars[i] != '$' {
            output.push(chars[i]);
            i += 1;
            continue;
        }
        if chars.get(i + 1) == Some(&'{') {
            let Some(end) = (i + 2..chars.len()).find(|j| chars[*j] == '}') else {
                output.push(chars[i]);
                i += 1;
                continue;
            };
            let body: String = chars[i + 2..end].iter().collect();
            let (name, fallback) = match body.find(":-").or_else(|| body.find('-')) {
                Some(at) => {
                    let skip = if body[at..].starts_with(":-") { 2 } else { 1 };
                    (&body[..at], Some(body[at + skip..].to_string()))
                }
                None => (body.as_str(), None),
            };
            if !name.is_empty() && name.chars().all(is_env_name_char) {
                match get_env(name).filter(|v| !v.is_empty()) {
                    Some(v) => output.push_str(&v),
                    None => {
                        output.push_str(&expand_shell_value(&fallback.unwrap_or_default(), get_env))
                    }
                }
                i = end + 1;
                continue;
            }
            output.push(chars[i]);
            i += 1;
            continue;
        }
        let end = (i + 1..chars.len())
            .find(|j| !is_env_name_char(chars[*j]))
            .unwrap_or(chars.len());
        if end == i + 1 {
            output.push(chars[i]);
            i += 1;
            continue;
        }
        let name: String = chars[i + 1..end].iter().collect();
        output.push_str(&get_env(&name).unwrap_or_default());
        i = end;
    }
    output
}

fn is_env_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        match name {
            "HOME" => Some("/home/me".into()),
            "XDG_STATE_HOME" => Some("/var/state".into()),
            "EMPTY" => Some("".into()),
            _ => None,
        }
    }

    #[test]
    fn test_expand_shell_value() {
        let expand = |v: &str| expand_shell_value(v, &env);
        assert_eq!(expand("plain"), "plain");
        assert_eq!(expand("~/app"), "/home/me/app");
        assert_eq!(expand("~"), "/home/me");
        assert_eq!(expand("a~/b"), "a~/b");
        assert_eq!(expand("$HOME/x"), "/home/me/x");
        assert_eq!(expand("${HOME}x"), "/home/mex");
        assert_eq!(expand("${MISSING}/tail"), "/tail");
        assert_eq!(expand("${MISSING:-fallback}"), "fallback");
        assert_eq!(expand("${EMPTY:-fallback}"), "fallback");
        assert_eq!(expand("${EMPTY-fallback}"), "fallback");
        assert_eq!(
            expand("${XDG_STATE_HOME:-$HOME/.local/state}/app"),
            "/var/state/app"
        );
        assert_eq!(
            expand("${MISSING:-$HOME/.local/state}/app"),
            "/home/me/.local/state/app"
        );
        assert_eq!(expand("100$"), "100$");
        assert_eq!(expand("${UNCLOSED/x"), "${UNCLOSED/x");
        assert_eq!(expand("$(date)"), "$(date)");
    }

    #[test]
    fn test_cobol() {
        assert_eq!("FOO-BAR".to_string(), to_cobol_case("fooBar"));
        assert_eq!("FOO-BAR".to_string(), to_cobol_case("foo-bar"));
        assert_eq!("FOO1".to_string(), to_cobol_case("foo1"));
    }
}
