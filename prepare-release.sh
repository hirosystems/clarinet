#!/bin/bash

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 1.2.3"
    exit 1
fi

NEW_VERSION=$1
ROOT_DIR=$(pwd)

get_package_version() {
    local file=$1
    local version=$(grep '"version":' "$file" | cut -d'"' -f4)
    echo "$version"
}

# Function to update version in package.json
update_package_json() {
    local file=$1
    local version=$2
    local dir=$(dirname "$file")

    # Update package version
    sed -i'' -e "s/\"version\": \".*\"/\"version\": \"$version\"/" "$file"

    # If this is a SDK package, also update the wasm dependency
    if [[ $file == *"clarinet-sdk"* ]]; then
        sed -i'' -e "s/\"@clarinet\/sdk-wasm\": \".*\"/\"@clarinet\/sdk-wasm\": \"$version\"/" "$file"
    fi

    echo "Updated version in $file to $version"
}

# Function to update version in Cargo.toml
update_cargo_toml() {
    local file=$1
    local version=$2

    sed -i'' -e "s/^version = \".*\"/version = \"$version\"/" "$file"
    echo "Updated version in $file to $version"
}

echo "Checking out release branch..."
git checkout -b release/next

echo "Starting version updates to $NEW_VERSION..."

# Update root Cargo.toml and package.json
echo "Updating root files..."
update_cargo_toml "./Cargo.toml" "$NEW_VERSION"

# Build SDK WASM
echo "Building SDK WASM..."
npm run build:sdk-wasm

# Update Clarinet SDK packages
echo "Updating Clarinet SDK packages..."
SDK_DIRS=("components/clarinet-sdk/node" "components/clarinet-sdk/browser")
for dir in "${SDK_DIRS[@]}"; do
    echo "Processing $dir..."
    update_package_json "$dir/package.json" "$NEW_VERSION"
    cd "$dir"
    npm i
    cd "$ROOT_DIR"
done

# Update stacks-devnet-js
echo "Updating stacks-devnet-js..."
cd components/stacks-devnet-js
update_package_json "package.json" "$NEW_VERSION"
npm i
cd "$ROOT_DIR"

# Handle clarity-vscode version separately
echo "Updating clarity-vscode..."
cd components/clarity-vscode
CURRENT_VERSION=$(get_package_version "package.json")
echo "The current version is $CURRENT_VERSION"
read -p "What would you like to update it to? " VSCODE_VERSION
update_package_json "package.json" "$VSCODE_VERSION"
npm i
cd "$ROOT_DIR"

echo "All updates completed successfully!"

echo "Adding git changes"
git commit -am "chore: release $NEW_VERSION"
git push origin release/next

# Clean up any backup files created by sed
find . -name "*-e" -delete
find . -name "*.bak" -delete
