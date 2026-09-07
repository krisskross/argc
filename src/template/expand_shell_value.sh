_argc_expand_shell_value() {
    local value="$1" out="" rest body name fallback
    case "$value" in
    "~" | "~/"*)
        out="$HOME"
        value="${value#\~}"
        ;;
    esac
    while [[ "$value" == *'$'* ]]; do
        out="$out${value%%\$*}"
        rest="${value#*\$}"
        if [[ "$rest" == '{'* ]]; then
            body="${rest#\{}"
            if [[ "$body" != *'}'* ]]; then
                out="$out\$"
                value="$rest"
                continue
            fi
            name="${body%%\}*}"
            value="${body#*\}}"
            fallback=""
            case "$name" in
            *:-*)
                fallback="${name#*:-}"
                name="${name%%:-*}"
                ;;
            *-*)
                fallback="${name#*-}"
                name="${name%%-*}"
                ;;
            esac
            if [[ -n "${!name:-}" ]]; then
                out="$out${!name}"
            else
                out="$out$(_argc_expand_shell_value "$fallback")"
            fi
        else
            name="${rest%%[!A-Za-z0-9_]*}"
            if [[ -z "$name" ]]; then
                out="$out\$"
                value="$rest"
                continue
            fi
            value="${rest#"$name"}"
            out="$out${!name:-}"
        fi
    done
    printf '%s' "$out$value"
}
