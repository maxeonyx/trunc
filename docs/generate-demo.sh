#!/bin/bash
# Generate demo output for the landing page

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TRUNC="$SCRIPT_DIR/../target/release/trunc"

# Generate sample build output
generate_input() {
    echo "Building project..."
    echo "Compiling src/main.rs"
    echo "Compiling src/lib.rs"
    for i in $(seq 4 96); do
        if [ $i -eq 42 ]; then
            echo "warning: unused variable \`config\` in src/config.rs:23"
        elif [ $i -eq 58 ]; then
            echo "warning: function \`parse_args\` is never used in src/cli.rs:15"
        else
            echo "   Compiling dependency $i..."
        fi
    done
    echo "Linking..."
    echo "Optimizing..."
    echo "Running tests..."
    echo "Finished release build in 12.4s"
}

echo '$ some-long-build-command | trunc'
echo ""
generate_input | $TRUNC -f 5 -l 5
