---
# The default id is the same as the one defined below. so not needed
title: Getting Started
---

## Install Clarinet

Hiro has developed Clarinet to be environment-agnostic. Follow this guide to install and build Clarinet. 

*Topics covered in this guide*:

* [Install Clarinet](#install-clarinet)
  * [Install on MacOS](#install-on-macos-homebrew)
  * [Install on Windows](#install-on-windows)
  * [Install from Pre-built library](#install-from-pre-built-binary)
* [Build Clarinet from source using Cargo](#build-clarinet)
* [Use Clarinet to generate shell completion scripts](#setup-shell-completions)

You may choose to install Clarinet in any of the following operating systems:

- macOS
- Windows
- Linux

To install Clarinet, you may choose from the following:
- [pre-built binary](https://github.com/hirosystems/clarinet#install-from-a-pre-built-binary)
- [build source from Cargo](https://github.com/hirosystems/clarinet#install-from-source-using-cargo)

> **_NOTE:_**
>
> There is no difference in Clarinet functionality based on the environment you select.

### Install on macOS (Homebrew)

To install Clarinet using macOs, you must first install Homebrew on your system. If you do not already have Homebrew already installed, 
please refer to the [Homebrew](https://brew.sh/) documentation for detailed information on installing Homebrew.

Once you have Homebrew installed, run the below command in your terminal.

```bash
brew install clarinet
```

For more information on how to install Clarinet on macOS, please see the [Setting Up Your Clarity Development Environment (Mac)](https://www.youtube.com/watch?v=dpPopuvYU90) video walkthrough.

### Install on Windows

The easiest way to install Clarinet on Windows is to use the MSI installer, 
which you can download from the [Hiro releases page](https://github.com/hirosystems/clarinet/releases).

You may also install Clarinet on Winget. Microsoft created this package manager, which includes the latest Windows updates.

Enter the command below.

```PowerShell
winget install clarinet
```

For more information on how to install Clarinet on Windows, please see the [Setting Up Your Clarity Environment (Windows)](https://www.youtube.com/watch?v=r5LY1J5oACs) video walkthrough.

### Install from a pre-built binary

If you would like to install Clarinet from pre-built binaries, you must first download the latest release from the 
[Hiro releases page](https://github.com/hirosystems/clarinet/releases). When you have downloaded the latest release,
unzip the binary and copy it to a location already in your path, such as `/usr/local/bin`, using the command shown below.

```sh
# note: you can change the v0.27.0 version that is available on the releases page.
wget -nv https://github.com/hirosystems/clarinet/releases/download/v0.27.0/clarinet-linux-x64-glibc.tar.gz -O clarinet-linux-x64.tar.gz
tar -xf clarinet-linux-x64.tar.gz
chmod +x ./clarinet
mv ./clarinet /usr/local/bin
```

>**_NOTE:_**
>
>If you use macOS, you may receive security errors when running the pre-compiled binary. 
> To resolve the security warning, use the command below and replace the path `/usr/local/bin/clarinet` with your local binary file.

```sh
xattr -d com.apple.quarantine /usr/local/bin/clarinet
```

### Install from source using Cargo

You may also install Clarinet using Cargo. If you choose this option, please be aware that you must first install Rust.
For more information on installing Rust, please see the [Install Rust](https://www.rust-lang.org/tools/install) page for access 
to `cargo`, the Rust package manager.

If you are using Debian or Ubuntu-based distributions, you must also install the following package to build Clarinet:
```bash
sudo apt install build-essential pkg-config libssl-dev
```
### Build Clarinet

Once you have installed Clarinet using Cargo, you can build Clarinet from the source using Cargo with the following commands:

```bash
git clone https://github.com/hirosystems/clarinet.git --recursive
cd clarinet
cargo clarinet-install
```

By default, you will be placed in our development branch, `develop`, with code that has not yet been released.

- If you plan to submit any code changes, this is the right branch for you. 
- If you prefer the latest stable version, switch to the main branch by entering the command below.

```bash
git checkout main
```

If you have previously checked out the source, ensure you have the latest code (including submodules) before building using this command:

```
git pull
git submodule update --recursive
```

Now that you have installed and built Clarinet, you can [create a new project](how-to-guides/how-to-create-new-project.md) and then [populate the project with smart contracts](how-to-guides/how-to-add-contract.md).

Clarinet also provides tools for interacting with your contracts in a Read, Evaluate, Print, Loop (REPL) console and perform automated [testing of contracts](how-to-guides/how-to-test-contract.md).

## Setup shell completions

Clarinet already has many different commands built in. Therefore, enabling tab completion in your shell may be useful. 
Using the command below, you can use `clarinet` to generate the shell completion scripts for many common shells.

```sh
clarinet completions (bash|elvish|fish|powershell|zsh)
```

After generating the file, you can refer to the documentation for your shell to determine where this file should be moved to and what other steps may be necessary to enable tab completion for `clarinet`.
