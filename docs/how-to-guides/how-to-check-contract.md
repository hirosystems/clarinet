---
title: Check Contracts
---

Clarinet provides syntax and semantics checkers for Clarity. You can verify if the Clarity code in your project is valid with the command listed below.

```bash
$ clarinet check
```

This command uses the `Clarinet.toml` file to locate and analyze all of the contracts in the project.
If the Clarity code is valid, then the command will indicate success with the response below.

```
✔ 2 contracts checked
```

The command may also report warnings indicating the code is valid.

You may also perform a syntax-check on a single file by using the command below.

```bash
$ clarinet check <path/to/file.clar>
```

If there are no syntax errors, the output of the command will be a success message.

```
✔ Syntax of contract successfully checked
```

Any syntactical errors in the Clarity code will be reported, but type-checking and other semantic checks are not performed.
This is because Clarinet is only looking at this one contract, and does not have the full context to perform a complete check.

