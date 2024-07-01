# Clarinet SDK Workspace

This workspace regroups
`@hirosystems/clarinet-sdk` for node.js and `@hirosystems/clarinet-sdk-browser` for web browsers.  
They respectively rely on `@hirosystems/clarinet-sdk-wasm` and `@hirosystems/clarinet-sdk-browser-wasm`.

Because of the way the wasm packages are build, with wasm-pack, it made sense to have two different
packages for Node.js and the browsers, but it has some caveats. Especially, some of the code is
duplicated in `./browser/src/sdkProxy.ts` and `./node/src/sdkProxy.ts`. In the future, we hope to 
be able to simplify this build, it would require some breaking changes so it could be part of 
Clarinet 3.x.

## Contributing

The clarinet-sdk requires a few steps to be built and tested locally.
We'll look into simplifying this workflow in a future version.

Clone the clarinet repo and `cd` into it:

```sh
git clone git@github.com:hirosystems/clarinet.git
cd clarinet
```

Open the SDK workspace in VSCode, it's especially useful to get rust-analyzer
to consider the right files with the right cargo features.

```sh
code components/clarinet-sdk/clarinet-sdk.code-workspace
```

The SDK mainly relies on two components:

- the Rust component: `components/clarinet-sdk-wasm`
- the TS component: `components/clarinet-sdk`

To work with these two packages locally, the first one needs to be built with
wasm-pack and linked with: [npm link](https://docs.npmjs.com/cli/v8/commands/npm-link).

Install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer) and run:

```sh
cd components/clarinet-sdk-wasm
wasm-pack build --release --scope hirosystems --out-dir pkg-node --target nodejs
cd pkg-node
npm link
```

Go to the `clarinet-sdk` directory and link the package that was just built.
It will tell npm to use it instead of the published version. You don't need to
repeat the steps everytime the `clarinet-sdk-wasm` changes, it only needs to be
rebuilt with wasm-pack and npm will use it.

Built the TS project:

```sh
cd ../../clarinet-sdk
npm link @hirosystems/clarinet-sdk-wasm
```

You can now run `npm test`, it wil be using the local version of `clarinet-sdk-wasm`

### Release

The Node.js and browser versions can be published with this single command.  
Make sure to check the check both packages versions first.

```sh
npm publish -w node -w browser --tag beta
```
