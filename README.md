# Clarinet

Clarinet is the fastest way to build, test, and deploy smart contracts on the Stacks blockchain. It gives you a local devnet, REPL, testing framework, and debugging tools to ship high-quality Clarity code with confidence.

- 🧑‍💻 **Leverage a powerful CLI**
  Create new projects, manage your smart contracts and their dependencies using clarinet requirements, and interact with your code through the built-in REPL.

- 🧪 **Write unit tests with the SDK**
  Use the Clarinet SDK to write unit tests in a familiar JS environment and validate contract behavior.

- 🛠️ **Run a private blockchain environment**
  Spin up a local devnet with nodes, miners, and APIs so you can test and integrate your code.

- 🔍 **VSCode extension**:
  Linter, step by step debugger, helps writing smart contracts (autocompletion, documentation etc)

---

### Documentation

- [Clarinet CLI](https://docs.hiro.so/stacks/clarinet)
- [Clarinet JS SDK and testing framework](https://docs.hiro.so/stacks/clarinet-js-sdk)

---

### Quickstart

```bash
# Install Clarinet
brew install clarinet
```

> To check out more installation methods, click [here](https://docs.hiro.so/stacks/clarinet#installation)

```bash
# Create a new project
clarinet new hello-world
cd hello-world
```

```bash
# Create a new contract
clarinet contract new counter
```

```clarity
;; Add this to the `contracts/counter.clar`

(define-map counters principal uint)

(define-public (count-up)
  (ok (map-set counters tx-sender (+ (get-count tx-sender) u1)))
)

(define-read-only (get-count (who principal))
  (default-to u0 (map-get? counters who))
)
```

```bash
# Then test it out

# Check the contract
clarinet check

# Launch the REPL
clarinet console
```

```bash
# Inside the console
(contract-call? .counter count-up)
(contract-call? .counter get-count tx-sender)
```

### Contributing

Contributions are welcome and appreciated. The following sections provide information on how you can contribute to Clarinet.

#### Prerequisites

Before contributing to Clarinet, please ensure you meet the following requirements:

- rust (>=1.52.0)
- cargo (>=1.52.0)
- node (>=v14.16.0) - Used for git commit hook
- npm (>=7.18.0) - Used for git commit hook

#### Guide

This repo follows the [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/#summary) specification when writing commit messages.

**Note:** It is important that any pull requests you submit have commit messages that follow this standard.

To start contributing:

1. Fork this repo and clone the fork locally.
2. Create a new branch
   ```bash
   git checkout -b <my-branch>
   ```
3. Run `npm i` in the local repo to install and initialize `husky` and `commitlint`.
   ```bash
   npm i
   ```
4. These tools will be used in a `git commit` hook to lint and validate your commit message. If the message is invalid, `commitlint` will alert you to try again and fix it.

Here is an example of a bad message response:

```bash
git commit -m "bad message"
⧗   input: bad message
✖   subject may not be empty [subject-empty]
✖   type may not be empty [type-empty]

✖   found 2 problems, 0 warnings
ⓘ   Get help: https://github.com/conventional-changelog/commitlint/#what-is-commitlint

husky - commit-msg hook exited with code 1 (error)
```

Here is an example of a good message response:

```bash
git commit -m "fix: added missing dependency"
[my-branch 4c028af] fix: added missing dependency
1 file changed, 50 insertions(+)
```

5. After making your changes, ensure the following:
   -  `cargo build` runs successfully.
   -  `cargo tst` runs successfully.
      -  `cargo tst` is an alias declared in `./cargo/config`, it runs [cargo-nextest](https://crates.io/crates/cargo-nextest)
   -  You have formatted your code with `cargo fmt-stacks`
   -  All functional tests in the `examples` directory pass.
      ```bash
      for testdir in $(ls examples); do
          pushd examples/${testdir}
              ../../target/debug/clarinet check .
          popd
      done
      ```
6. Submit a pull request against the `develop` branch for review.

### Code of Conduct
Please read our [Code of conduct](../../../.github/blob/main/CODE_OF_CONDUCT.md) since we expect project participants to adhere to it. 

## Community

Join our community and stay connected with the latest updates and discussions:

- [Join our Discord community chat](https://discord.com/invite/pPwMzMx9k8) to engage with other users, ask questions, and participate in discussions.
- [Visit hiro.so](https://www.hiro.so/) for updates and subscribing to the mailing list.
- Follow [Hiro on X.](https://x.com/hirosystems)
