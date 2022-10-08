# Clarity for Visual Studio Code

Clarity is a **decidable** smart contract language that optimizes for predictability and security. Developed by Hiro, Clarity enables smart contract developers to encode essential business logic on a blockchain.

A programming language is referred to as **decidable** if you can know with certainty, from the code itself, what the program will do. Clarity is was designed to be intentionally Turing incomplete, thereby avoiding the problem of `Turing complexity` (the number of steps it takes for a turing machine to run an algorithm on a given input size). This allows for complete static analysis of the entire call graph of a given smart contract, while Hiro's support for types and type checkers can eliminate whole classes of bugs like unintended casts, reentrancy bugs, and reads of uninitialized values.

The Language Server Protocol (LSP) defines the protocol used between an editor or IDE, and a language server that provides language features such as auto-complete, go-to-definition, find-all-references, and others.

This Clarity-LSP project aims at leveraging the decidability quality of Clarity and the LSP to provide insights about your code, without publishing your smart contracts to a blockchain, while also enabling you to run the Clarity for Visual Studio Code extenstion in-browser.

![screenshot](images/screenshot.png)

## Clarity for Visual Studio Installation

You can install the latest release of the plugin from the [marketplace](https://marketplace.visualstudio.com/items?itemName=hirosystems.clarity-lsp).
## Features

The features described below are available when you install the Clarity for Visual Studio plugin.

### Auto Complete Native Functions

This feature enables you to start typing a function name, and then have the editor automatically suggest auto-completion with the documentation related to the suggestion.

When you select a function, the extension adds the necessary parentheses around it and puts placeholders in the arguments of the function.

![autocomplete gif](images/autocomplete.gif)

### Check Contract on Save and Display Errors Inline

When a contract is opened or saved, the extension will notify you if errors are found (syntax, unknown keyword, etc), or warnings (such as unsafe code). This helps you to ensure that you write safe and clean code.

![display errors gif](images/errors.gif)

### VS-Code Support

Visual Studio Code is natively supported, but this extension provides additional support for [vscode.dev](https://vscode.dev/), [github.dev](https://github.dev/github/dev) and similar tools.

For more information, please see the [Clarinet GitHub VS Code documentation page](https://github.com/hirosystems/clarinet/blob/develop/components/clarity-vscode/README.md)

### Auto Complete User-Defined Functions

This feature is similar to the auto-complete feature, but does not provide documentation for functions when writing a contract.
### Resolve Contract-Call Targeting Local Contracts

This feature is similar to the auto-complete feature, but enables auto-completion over multiple files.

### Support for Multiple Errors

Although not considered a native feature for the Visual Studio Code extension, support for multiple errors is now supported in this version.

![multiple error support](images/multicontract.gif)
### Support for Traits

When a contract implements a trait (such as the NFT of FT trait – SIPs 009 and 010), the extensions will show and errors if the trait implementation is not satisfied (for example, if the trait expects a function which is not implemented, or if the function signature does not match the trait definition).
### Documentation

This feature extends the capabilities of the native auto-complete function by providing documentation for a function when writing a smart contract.

### Debugger

The debugging feature allows you to run Clarity code, line-by-line, so you can better understand what will happen when run.

For more information on how debugging works, and how you can debug smart contracts, please see the [How to Debug Your Smart Contracts With Clarinet](https://www.hiro.so/blog/how-to-debug-your-smart-contracts-with-clarinet) blog post.

### Handle Requirements

If your Clarity project relies on specific requirements for [interacting with contracts on mainnet](https://github.com/hirosystems/clarinet#interacting-with-contracts-deployed-on-mainnet),this extension will automatically detect the requirement 
on download, and then cache the required contracts. In some cases, you may need to require contracts to have
defining traits (such as SIPs 009 and 010); however, this may only concern contracts deployed on mainnet.
