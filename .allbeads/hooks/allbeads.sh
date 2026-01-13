#!/bin/bash
# AllBeads Shell Integration
# Source this file in your ~/.bashrc or ~/.zshrc to enable Aiki integration
#
# Add to your shell rc file:
#   source /path/to/allbeads/.allbeads/hooks/allbeads.sh
#
# Or if AllBeads is installed:
#   eval "$(ab aiki hook-init)"

# Load active bead environment variable if it exists
ALLBEADS_ENV_FILE="${ALLBEADS_ENV_FILE:-$HOME/.config/allbeads/env}"

if [ -f "$ALLBEADS_ENV_FILE" ]; then
    source "$ALLBEADS_ENV_FILE"
fi

# Wrapper function for bd/ab update to maintain AB_ACTIVE_BEAD
_allbeads_update_wrapper() {
    local cmd="$1"
    shift

    # Run the actual command
    command "$cmd" update "$@"
    local exit_code=$?

    # If successful and status is in_progress, extract bead ID and update env
    if [ $exit_code -eq 0 ]; then
        # Check if --status=in_progress was passed
        for arg in "$@"; do
            if [[ "$arg" == "--status=in_progress" || "$arg" == "--status=in-progress" ]]; then
                # First arg should be the bead ID
                local bead_id="$1"
                if [ -n "$bead_id" ] && [[ ! "$bead_id" == --* ]]; then
                    mkdir -p "$(dirname "$ALLBEADS_ENV_FILE")"
                    echo "export AB_ACTIVE_BEAD=\"$bead_id\"" > "$ALLBEADS_ENV_FILE"
                    export AB_ACTIVE_BEAD="$bead_id"
                    echo "✓ Activated bead: $bead_id (set AB_ACTIVE_BEAD)"
                fi
                break
            fi
        done
    fi

    return $exit_code
}

# Wrapper function for bd/ab close to clear AB_ACTIVE_BEAD
_allbeads_close_wrapper() {
    local cmd="$1"
    shift

    # Run the actual command
    command "$cmd" close "$@"
    local exit_code=$?

    # If successful, check if any of the closed beads is the active one
    if [ $exit_code -eq 0 ] && [ -n "$AB_ACTIVE_BEAD" ]; then
        for arg in "$@"; do
            if [[ "$arg" == "$AB_ACTIVE_BEAD" ]]; then
                if [ -f "$ALLBEADS_ENV_FILE" ]; then
                    rm "$ALLBEADS_ENV_FILE"
                fi
                unset AB_ACTIVE_BEAD
                echo "✓ Deactivated bead: $arg (unset AB_ACTIVE_BEAD)"
                break
            fi
        done
    fi

    return $exit_code
}

# Optional: Create shell functions to wrap bd/ab commands
# Uncomment to enable automatic env var management
# alias bd='_allbeads_bd_wrapper'
# alias ab='_allbeads_ab_wrapper'

# Helper to show current active bead
allbeads-active() {
    if [ -n "$AB_ACTIVE_BEAD" ]; then
        echo "Active bead: $AB_ACTIVE_BEAD"
    else
        echo "No active bead"
    fi
}
