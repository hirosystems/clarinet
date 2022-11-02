---
# The default id is the same as the one being defined below. so not needed
title: Getting Started
---

Follow this guide to install and build Clarinet. 

## Install and Build Clarinet

Hiro has developed Clarinet to be environment-agnostic.
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

To install Clarinet using macOs, you must first have Homebrew installed on your system. If you do not already have Homebrew already installed, 
please refer to the [Homebrew](https://brew.sh/)documentation for detailed information on how to install Homebrew.

Once you have Homebrew installed, run the following command shown below in your terminal.

```bash
brew install clarinet
```

For more informaiton on how to install Clarinet on macOS, please see the [Setting Up Your Clarity Development Environment (Mac)](https://www.youtube.com/watch?v=dpPopuvYU90) video walkthrough.

### Install on Windows

The easiest way to install Clarinet on Windows is to use the MSI installer, 
which you can download from the [Hiro releases page](https://github.com/hirosystems/clarinet/releases).

You may also install Clarinet on Winget. This is the package manager that Microsoft created, which includes in the latest Windows updates.
Simply enter the command below.

```powershell
winget install clarinet
```

For more information on how to install Clarinet on Windows, please see the [Setting Up Your Clarity Environment (Windows)](https://www.youtube.com/watch?v=r5LY1J5oACs) video walkthrough.

### Install from a pre-built binary

If you would like to install Clarinet from pre-built binaries, you must first download the latest release from the 
[Hiro releases page](https://github.com/hirosystems/clarinet/releases). When you have downloaded the latest release,
Uuzip the binary and then copy it to a location that is already in your path, such as `/usr/local/bin` using the command shwon below.

```sh
# note: you can change the v0.27.0 with version that are available in the releases page.
wget -nv https://github.com/hirosystems/clarinet/releases/download/v0.27.0/clarinet-linux-x64-glibc.tar.gz -O clarinet-linux-x64.tar.gz
tar -xf clarinet-linux-x64.tar.gz
chmod +x ./clarinet
mv ./clarinet /usr/local/bin
```

>**_NOTE:_**
>
>If you are using macOS, you may receive security errors when trying to run the pre-compiled binary. 
>You can resolve the security warning by using the command below.

```sh
xattr -d com.apple.quarantine /path/to/downloaded/clarinet/binary
```

### Install from source using Cargo

You may also install Clarinet using Cargo. If you choose this option, please be aware that you must first intall Rust.
For more information on installing Rust, please see the [Install Rust](https://www.rust-lang.org/tools/install) page for access 
to `cargo`, the Rust package manager.

If you are using Debian or Ubuntu-based distributions, you must also install the following package to build Clarinet:
```bash
sudo apt install build-essential pkg-config libssl-dev
```
### Build Clarinet

Once you have installed Clarinet using Cargo, you can build Clarinet from source using Cargo with the following commands:

```bash
git clone https://github.com/hirosystems/clarinet.git --recursive
cd clarinet
cargo clarinet-install
```

By default, you will be placed in our development branch, `develop`, with code that has not yet been released.

- If you plan to submit any changes to the code, then this is the right branch for you. 
- If you would prefer to have the latest stable version, then switch to the main branch by entering the command below.

```bash
git checkout main
```

If you have previously checked out the source, ensure you have the latest code (including submodules) before building using this command:

```
git pull
git submodule update --recursive
```


Now that you have installed and built Clarinet, you can [create a new project](how-to-guides/how-to-create-new-project.md), [add a new contract](how-to-guides/how-to-add-contract.md) and then populate the project with smart contracts.

Clarinet also provides tools for interacting with your contracts in a Read, Evaluate, Print, Loop (REPL) console, and perform automated [testing of contracts](how-to-guides/how-to-test-contract.md).

## Setup shell completions

Clarinet already has many different commands built in, therefore, you may find it useful to enable tab-completion in your shell. 
You can use `clarinet` to generate the shell completion scripts for many common shells using the command shown below.

```sh
clarinet completions (bash|elvish|fish|powershell|zsh)
```

After generating the file, you can refer to the documentation for your shell to determine where this file should be moved to, and what other steps may be necessary to enable tab-completion for `clarinet`.

