#!/bin/bash

set -euo pipefail

function confirm() {
    local msg="$1"
    local input=""

    read -rp "$msg (yes/NO): " input
    if [[ $input != "yes" ]]; then
        echo
        echo "Aborted. Type 'yes' to confirm."
        exit 1
    fi
}

echo "=== The following change will be released ==="
cargo release changes

echo
echo "=== Dry-running release ==="
cargo release

confirm "Do you want to release?"
cargo release -x
