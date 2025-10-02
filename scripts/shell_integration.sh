# VibeRot Shell Hook Integration
# This enables VibeRot to monitor commands executed in your shell

# Include guard to prevent sourcing multiple times
if [ -n "$VIBEROT_ENABLED" ]; then
    return 0
fi

# Check shell support
if [ -z "$ZSH_VERSION" ] && [ -z "$BASH_VERSION" ]; then
    echo "VibeRot error: Unsupported shell. Only bash and zsh are supported." >&2
    return 1
fi
# Check dependencies
if ! command -v base64 >/dev/null 2>&1; then
    echo "VibeRot error: base64 command not found" >&2
    return 1
fi
if ! command -v nc >/dev/null 2>&1 && ! command -v socat >/dev/null 2>&1; then
    echo "VibeRot error: Neither nc nor socat command found" >&2
    return 1
fi
if ! command -v timeout >/dev/null 2>&1; then
    echo "VibeRot error: timeout command not found" >&2
    return 1
fi
if ! command -v printf >/dev/null 2>&1; then
    echo "VibeRot error: printf command not found" >&2
    return 1
fi
if [[ -z "$ZSH_VERSION" && -z "$bash_preexec_imported" ]]; then
    echo "VibeRot error: preexec and precmd not available" >&2
    return 1
fi
# Read socket path from config file
_viberot_config_file="$HOME/.viberot/.socket"
if [[ ! -f "$_viberot_config_file" ]]; then
    # Fail silently because the user may not always want to enable VibeRot
    return 0
fi

# Read the socket path from the config file
_viberot_socket_path=$(cat "$_viberot_config_file" 2>/dev/null | head -n 1 | tr -d '\n\r')
if [[ -z "$_viberot_socket_path" ]]; then
    # Fail silently if socket path is empty
    return 0
fi

readonly VIBEROT_ENABLED=1

# Function to base64 encode strings safely
_viberot_base64_encode() {
    local input="$1"
    if command -v base64 >/dev/null 2>&1; then
        printf '%s' "$input" | base64 -w 0 2>/dev/null || printf '%s' "$input" | base64
    else
        echo "Error: base64 command not found" >&2
        exit 1
    fi
}

# Function to communicate with VibeRot service
_viberot_send_message() {
    local message="$1"
    # Send single-line JSON message terminated with newline
    if command -v nc >/dev/null 2>&1; then
        printf '%s\n' "$message" | nc -w 0 -U "$_viberot_socket_path" 2>/dev/null || true disown
    elif command -v socat >/dev/null 2>&1; then
        printf '%s\n' "$message" | socat - UNIX-CONNECT:"$_viberot_socket_path" 2>/dev/null || true disown
    else
        echo "Error: Neither nc nor socat command found" >&2
        exit 1
    fi
}

# This flag allows precmd to determine if a command is actually executed
_viberot_last_command=""

_viberot_pre_command_hook() {
    if [[ "$1" != _viberot_* ]] && [ -n "$1" ]; then
        _viberot_last_command="$1"
        # Base64 encode values that may contain special characters
        local encoded_command="$(_viberot_base64_encode "$1")"
        local encoded_pwd="$(_viberot_base64_encode "$PWD")"
        local json_msg="{\"session_id\":\"$$\",\"event_type\":\"CommandStart\",\"command_b64\":\"$encoded_command\",\"working_directory_b64\":\"$encoded_pwd\",\"environment\":{}}"
        _viberot_send_message "$json_msg"
    fi
}

_viberot_post_command_hook() {
    if [[ -n "$_viberot_last_command" ]]; then
        local exit_code=$?
        local json_msg="{\"session_id\":\"$$\",\"event_type\":\"CommandEnd\",\"exit_code\":$exit_code}"
        _viberot_send_message "$json_msg"
    fi
    _viberot_last_command=""
}

preexec_functions+=(_viberot_pre_command_hook)
precmd_functions+=(_viberot_post_command_hook)

echo "VibeRot shell integration enabled."
