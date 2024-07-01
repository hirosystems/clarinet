# Clarity SDK WASM

This component exposes Clarinet features to a JS interface through wasm-bindgen.
It's built with wasm-pack.  
It powers [@hirosystems/clarinet-sdk](https://npmjs.com/package/@hirosystems/clarinet-sdk) and
[@hirosystems/clarinet-sdk-browser](https://npmjs.com/package/@hirosystems/clarinet-sdk-browser).

## Contributing

### Build package

Install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

**Build for node**

```sh
wasm-pack build --release --scope hirosystems --out-dir pkg-node --target nodejs
```

**Build for the browser**

```sh
wasm-pack build --release --scope hirosystems --out-dir pkg-browser --target web
```

Run this script to build **both versions**:

```sh
node build.mjs
```

### Use the local version of the package

#### NPM overrides

In most of the situations, your project won't directly depend on this package, but instead on
`@hirosystems/clarinet-sdk` or `@hirosystems/clarinet-sdk-browser`. If you want to use a local or
a different version of `@hirosystems/clarinet-sdk-wasm` or `@hirosystems/clarinet-sdk-wasm-browser`,
you can use the `overrides` setting in your package.json:

```json
  "overrides": {
    "@hirosystems/clarinet-sdk": {
      "@hirosystems/clarinet-sdk-wasm": "file:/<absolue-path-to>/clarinet/components/clarinet-sdk-wasm/pkg-node"
    }
  }
```

Or for the browser:

```json
  "overrides": {
    "@hirosystems/clarinet-sdk-browser": {
      "@hirosystems/clarinet-sdk-wasm-browser": "file:/<absolue-path-to>/clarinet/components/clarinet-sdk-wasm/pkg-browser"
    }
  }
```

#### NPM link

The command `npm link` can be useful to run the unit tests in the `clarinet-sdk`.
See the contribution section of `@hirosystems/clarinet-sdk` (`../clarinet-sdk/README.md).

### Release

The package is built twice with `wasm-pack` as it can't target `node` and `web` at the same time.
The following script will build for both target, it will also rename the package name for the
browser build.

```sh
cd components/clarinet-sdk-wasm
node build.mjs
```

Once built, the packages can be released by running the following commands. Note that by default we
release with the beta tag. 

```sh
cd pkg-node && npm publish --tag beta && cd ..
cd pkg-browser && npm publish --tag beta && cd ..
```
