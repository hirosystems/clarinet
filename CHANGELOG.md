#### 1.1.0 (2022-11-17)

##### New Features

*  Support for Stacks 2.1 ([790c14bf](https://github.com/hirosystems/clarinet/commit/790c14bf8fd4a30d1f50c2c4a55873aecac1a076))
*  Display clarity documentation on hover in VSCode ([e605acb4](https://github.com/hirosystems/clarinet/commit/e605acb49f0892cb75b7a16edf37807f29133a20))

##### Chores

*  Better error management in chainhook-node ([353ceb61](https://github.com/hirosystems/clarinet/commit/353ceb617b8a5b710331fe3387b07f6ad48e3f48))

#### 1.0.6 (2022-11-10)

##### New Features

*  Ability for chainhook-node to start with custom config ([473c86ba](https://github.com/hirosystems/clarinet/commit/473c86babe41f0c16ea9c370890d440a698dfa08))
*  Draft bitcoin replay implementation ([8580092e](https://github.com/hirosystems/clarinet/commit/8580092e2e8661c3d6e653be2c3f687774e560fa))

##### Bug Fixes

*  Filter out boot contracts from requirement dependencies ([34fbcf96](https://github.com/hirosystems/clarinet/commit/34fbcf9686a9bdcdf2e11715abbcb9fa301e4dfb))
*  Deno expect events ([8bdcd392](https://github.com/hirosystems/clarinet/commit/8bdcd39254c8fb269245005f30b2f930df8dd7ea))
*  Fix issue with check-checker requiring checking on bools ([ef738fe3](https://github.com/hirosystems/clarinet/commit/ef738fe3f98cbdb87dd8e510d5ed0857817350eb))

##### Refactors

*  Add serverless dependency in cbtc example and upgrade dependencies ([f065f2b](https://github.com/hirosystems/clarinet/pull/660/commits/f065f2b3d5268689b7d2b77bba668f352bb53ca2))
*  Get_bitcoin_proof ([5a3a8ee9](https://github.com/hirosystems/clarinet/commit/5a3a8ee97a4b6db895c048ba9d67ac3423abe5de))

#### 1.0.5 (2022-11-03)

##### New Features

*  Many chainhooks enhancements: event payload augmented, new predicates (segwit, etc) implemented. Documentation coming soon!
*  Introducing clarity-jupyter-kernel ([1c118513](https://github.com/hirosystems/clarinet/commit/1c1185136a1c52248a1b20ba43b5887fbaa4ef4d))
*  Ability to use low/medium/high cost dynamic presets in deployment plans ([86219c4e](https://github.com/hirosystems/clarinet/commit/86219c4e52997d8aad059e871c96f96a5834c616))
*  Ability to send STX in deployment plans ([c9e3bac4](https://github.com/hirosystems/clarinet/commit/c9e3bac44c2fe97f2f6b5f6578cc309f2cc2e38f))

##### Bug Fixes

*  Termination in `clarinet integrate --no-dashboard` mode fixed ([2cdb09a6](https://github.com/hirosystems/clarinet/commit/2cdb09a6aeed631236971f4d4206ff97b742683e))
*  Check print predicate for contained value ([0f5956dc](https://github.com/hirosystems/clarinet/commit/0f5956dc1019f25d14c4204d7cece8923d74ae7b))
*  Improved keyword recognition in VSCode grammar file ([e690b371](https://github.com/hirosystems/clarinet/commit/e690b371331ed8ec4d27cbc609581c9f07e04888), [119dce57](https://github.com/hirosystems/clarinet/commit/119dce577dd654e471a3054c206c593bdf78bb1b))
*  Fixed stacks-js-helper generator ([11562ae7](https://github.com/hirosystems/clarinet/commit/11562ae739170a799620f4a62462219304dafc19))

##### Chores

*  Types improvements clarinet deno library ([3bc5c51c](https://github.com/hirosystems/clarinet/commit/3bc5c51cda35bdc52c8867fe222341680e0e3880))
*  Add tests to the clarinet deno library ([d5b7555d](https://github.com/hirosystems/clarinet/commit/d5b7555d5a3acf4d5d53e32f06e6b80520b93c4e), [062a7144](https://github.com/hirosystems/clarinet/commit/062a7144f25e019dbacd62f8874e2d0a783fd20f))

#### 1.0.4 (2022-10-17)

##### New Features

*  Devnet chainstate now lives in `cache` directory specified in Clarinet.toml ([a6fb383f](https://github.com/hirosystems/clarinet/commit/a6fb383fecb936d27386f3f914f98dda89a67dda))
*  Ability to pass wildcards for chainhook testing ([08f75a2a](https://github.com/hirosystems/clarinet/commit/08f75a2a2abcf1cbc1b2c60115cc7d939d090fbd))
*  Ability to use write-to-file as action (chainhooks) ([fb19e392](https://github.com/hirosystems/clarinet/commit/fb19e392836430d029aa8374a625a49451b38ad9))
*  Introduce stacks-network component ([ace64116](https://github.com/hirosystems/clarinet/commit/ace641164465d7a253375365a2f805a650981d09))

##### Bug Fixes

*  Bump ingestion limits to 5 mb ([e4d539da](https://github.com/hirosystems/clarinet/commit/e4d539da703371d6046a64088e18fb23d0452575))
*  Fix invalid bitcoin txid ([03783a41](https://github.com/hirosystems/clarinet/commit/03783a414837afc78c7688741d42ab4309389abb))
*  Fix coverage tracking ([1a4836d1](https://github.com/hirosystems/clarinet/commit/1a4836d1d6e35e835a76465caf0b3be01a5b2aee))
*  Fix crash occuring on NFTMintEvent ([d5dc3fc0](https://github.com/hirosystems/clarinet/commit/d5dc3fc0454a51a5a9708a66b63cb5ccf58c3b24))
*  Resolve boot contract in LSP ([ad34037c](https://github.com/hirosystems/clarinet/commit/ad34037cbd4d6f30360a08817e497a8c9a9ef2de))
*  Better error ([d42f7ed6](https://github.com/hirosystems/clarinet/commit/d42f7ed680097e672611ab1a581f55f32208dd11))
*  Performance optimisation with parser v2 ([c1712489](https://github.com/hirosystems/clarinet/commit/c171248997a357d8e6a1bc074b79a0452de6235f))

#### 1.0.0 (2022-10-06)

##### New Features

*  Introducing our brand new re-architected VSCode extension ([README](https://github.com/hirosystems/clarinet/tree/develop/components/clarity-vscode))
*  All of our tools (REPL, LSP, Clarinet) are now directly derived from the canonical Clarity VM ([#512](https://github.com/hirosystems/clarinet/pull/512), [#535](https://github.com/hirosystems/clarinet/pull/535), [#544](https://github.com/hirosystems/clarinet/pull/544))
*  Ability to trigger chainhooks from unit tests ([#564](https://github.com/hirosystems/clarinet/pull/564))
*  Deno integration upgraded and revisited ([#511](https://github.com/hirosystems/clarinet/pull/511))
*  Ability to specify Deno import maps ([#511](https://github.com/hirosystems/clarinet/pull/511))
*  Ability to specify TS config files ([#555](https://github.com/hirosystems/clarinet/pull/555/commits/083b498ef4210b50de74a70bc20e4a4e5a64db94))
*  Ability to cache Deno libraries locally ([a2c2ded3](https://github.com/hirosystems/clarinet/commit/a2c2ded391e3110c640d0d3e6b41d3cd0d1b56e5))
*  Bitcoin deployment plans now supports transfers to P2WPKH addresses ([c50a4c27](https://github.com/hirosystems/clarinet/commit/c50a4c27857a41d178393306f97e464bffea9b80))
*  Ability to detect outdated deployment plans and display diffs ([#365](https://github.com/hirosystems/clarinet/issues/365))

##### Bug Fixes

A myriad of issues were addressed in this new version, the most notable being:

*  Cannot make http request from within clarinet test ([#566](https://github.com/hirosystems/clarinet/issues/566))
*  Clarinet CPU usage spiking to 100% when using clarinet integrate ([#545](https://github.com/hirosystems/clarinet/issues/545))
*  Clarinet console crashes when it errors ([#541](https://github.com/hirosystems/clarinet/issues/541))
*  Unhandled Division By Zero exception ([#525](https://github.com/hirosystems/clarinet/issues/525))
*  Handle errors from callReadOnlyFn in tests ([#407](https://github.com/hirosystems/clarinet/issues/407))
*  Arithmetic underflow crashes clarity-repl instead of displaying error ([#471](https://github.com/hirosystems/clarinet/issues/471))
*  Improve debugability of chain.mine_block() ([#91](https://github.com/hirosystems/clarinet/issues/91))


##### Documentation

*  Added new example - How to use Chainhooks for indexing data ([cdeca648](https://github.com/hirosystems/clarinet/commit/cdeca64837e51dd64292ba2f4ddfcdfc3ef77da1))
*  Added OpenAPI spec for Chainhooks ([01e8979c](https://github.com/hirosystems/clarinet/commit/01e8979c815cff701496d25e07dbf6777ff0afd5))

##### Compatibility Issue

Clarinet v1.0.0 is not currently backwards-compatible with older versions of the Clarinet deno library. If you are upgrading Clarinet to Clarinet `v1.0.0`, you will need to enter the following import command in your test files to perform this update.
```ts
import { … } from 'https://deno.land/x/clarinet@v1.0.2/index.ts';
```

*Note* The `v1.0.0` library is not compatible with Clarinet versions <= `0.33.0`. Prior versions of the library also will not be compatible with versions >= `1.0.0` of Clarinet because the layer in charge of the communication between Typescript and Rust was upgraded.
If you are using Clarinet in a *Github Action*, and using the tag `latest` (now pointing to `v1.0.0`), the tests will fail if the import upgrade task is not performed. If you do not want to upgrade, this is possible; however, you will need to specify the docker tag `v0.33.0`, instead of `latest`.

#### 0.33.0 (2022-07-20)

##### Chores

*  migrate to mono-repo layout ([#481](https://github.com/hirosystems/clarinet/pull/481))
* **deps:**
  *  bump crossbeam-utils in /components/stacks-devnet-js ([9a0dedfd](https://github.com/hirosystems/clarinet/commit/9a0dedfd41aeed36ec503bd969e595b7c9dc207d))
  *  bump thread_local in /components/stacks-devnet-js ([a6b5065f](https://github.com/hirosystems/clarinet/commit/a6b5065fa9d59df2701bf7cc968203c0b8f7d30d))
  *  bump nix in /components/stacks-devnet-js ([f453b4aa](https://github.com/hirosystems/clarinet/commit/f453b4aae32dee20f9d4f006e70b2518c2878bb3))

##### Continuous Integration

*  revisit CI and release process ([423c3d36](https://github.com/hirosystems/clarinet/commit/423c3d36c7cb571156bb6553162dbac0b24a2e1c))

##### Documentation Changes

*  README.md. Removed depends_on() field in the clarinet.toml file and added success message for clarinet check command. ([84d0a327](https://github.com/hirosystems/clarinet/commit/84d0a32776b69520228d2e5149a4a3428e970b56))

##### New Features

*  polish hyperchain integration ([#432](https://github.com/hirosystems/clarinet/pull/432), [#480](https://github.com/hirosystems/clarinet/pull/480), [#494](https://github.com/hirosystems/clarinet/pull/494) )
*  display microblocks in clarinet terminal UI ([77535aa6](https://github.com/hirosystems/clarinet/commit/77535aa62637f4f42af8f5316c42ea78688efce7))
*  improve block / microblock fork handling ([#480](https://github.com/hirosystems/clarinet/pull/480))
*  various chainhooks improvements ([#429](https://github.com/hirosystems/clarinet/pull/429))
*  suggest changes to default deployment plans when updates available ([#488](https://github.com/hirosystems/clarinet/pull/488), [#489](https://github.com/hirosystems/clarinet/pull/489))


# [0.32.0](https://github.com/hirosystems/clarinet/compare/v0.31.1...v0.32.0) (2022-06-23)


### Bug Fixes

* add dotenv to cbtc dependencies ([158390a](https://github.com/hirosystems/clarinet/commit/158390aa42f6781a684a3322a2ce960a5c8329ec))
* address auto-review ([61200a1](https://github.com/hirosystems/clarinet/commit/61200a1aafd4592b17524207d99e6ea15cc43d42))
* address feedbacks ([cdf4e6d](https://github.com/hirosystems/clarinet/commit/cdf4e6def986d43734a6dc0b7aff688b7f06f409))
* address feedbacks ([ce82131](https://github.com/hirosystems/clarinet/commit/ce821313ed66b9190826dd147cdcc477cf6908f0))
* address feedbacks ([895ba87](https://github.com/hirosystems/clarinet/commit/895ba873a7a36f7218ce35e4f9e081fb5d4d18fb))
* address remaining feedbacks ([5963383](https://github.com/hirosystems/clarinet/commit/59633832515a2974261eb8e5886e9632d62695b6))
* better termination ([16def80](https://github.com/hirosystems/clarinet/commit/16def803f389a60abed5d51ffab5159438751e69))
* broken termination ([45dff9f](https://github.com/hirosystems/clarinet/commit/45dff9fac5b9b6f791277a1c2cf8116013095f5c))
* build ([9fa504e](https://github.com/hirosystems/clarinet/commit/9fa504e19f290ab68821c7fe77477a59a768847f))
* build issues ([84ec835](https://github.com/hirosystems/clarinet/commit/84ec835d84a0980195ddb230b8e33ee2c73ca461))
* build warning ([b164be4](https://github.com/hirosystems/clarinet/commit/b164be43c05e0c8a856ca8340318756d92e1e2c1))
* build, unused files ([a3acaa9](https://github.com/hirosystems/clarinet/commit/a3acaa98bcce094306ce5ddc4004504ee33b741c))
* cargo audit ([44b60d5](https://github.com/hirosystems/clarinet/commit/44b60d5fdebc2eade932e41acd0f8756eb733752))
* cargo fmt, stacks-devnet ([869916e](https://github.com/hirosystems/clarinet/commit/869916edc50617c5e4245c9af259c5f5f36e06fc))
* cascade changes in Oreo.dockerfile ([9626aed](https://github.com/hirosystems/clarinet/commit/9626aedde7ed3033fefd3ceadd562165f8e44b3c))
* cascade errors ([8cd5d9a](https://github.com/hirosystems/clarinet/commit/8cd5d9a9a1da27dc63befc87e5e31f2fe8a6e879))
* cascade errors from generators ([ef6a64e](https://github.com/hirosystems/clarinet/commit/ef6a64e5bbdd43129478ca6480fb9856f8043806))
* cbtc chainhooks ([27867c6](https://github.com/hirosystems/clarinet/commit/27867c608eb7e82864bc810b887d4ecd02804f56))
* command line deployment generate ([de61f49](https://github.com/hirosystems/clarinet/commit/de61f4956a9678888edcfd1ed2451308ebe7ce9d))
* dead code ([6119ec4](https://github.com/hirosystems/clarinet/commit/6119ec41aba1b9a78db161311e5cb6dd60e762c9))
* display transaction in mempool view ([006a86e](https://github.com/hirosystems/clarinet/commit/006a86ef92a6e69ce8bfc1044750081700275544))
* fix typo "runnner" -> "runner" ([54f5ccc](https://github.com/hirosystems/clarinet/commit/54f5cccef7486f8f32bdc5ff62508db90b736cdc))
* handle warnings ([852014b](https://github.com/hirosystems/clarinet/commit/852014be7bcff19305b78d7191f95a34bd425b17))
* hyperchain tweaks ([369c9da](https://github.com/hirosystems/clarinet/commit/369c9da45723a13e4bb00df3601a9a7d65aa5c13))
* improve error messages about dependencies ([48ea5a0](https://github.com/hirosystems/clarinet/commit/48ea5a0b87a00fd082d4a91bb072b72bb7d785cd)), closes [hirosystems/clarity-repl#188](https://github.com/hirosystems/clarity-repl/issues/188) [#396](https://github.com/hirosystems/clarinet/issues/396)
* lsp and requirements ([334d81f](https://github.com/hirosystems/clarinet/commit/334d81f73d789fe97beeab73ca63e8aa5b9f560a))
* path ([50b7e0d](https://github.com/hirosystems/clarinet/commit/50b7e0ddbcd01d46e7e5a6e52e22b0949e93f222))
* remove ProjectManifest::default() ([cdde911](https://github.com/hirosystems/clarinet/commit/cdde911809224ec53590123434adafbb387bd4c7))
* serialize using relative path ([5ce5970](https://github.com/hirosystems/clarinet/commit/5ce5970cd3fb8dda6493dfc540b1d7edd4d8248a))
* stacks-devnet-js ([983b7be](https://github.com/hirosystems/clarinet/commit/983b7be3139d6b7097a43d0cc6798f4ef2ac271d))
* ts lib ([83337a0](https://github.com/hirosystems/clarinet/commit/83337a02e0fbe17d7b2bb0db2d621f1113d7c51f))
* typo ([e8f1303](https://github.com/hirosystems/clarinet/commit/e8f13032e660cfa1c17dbaa06d0d3d4423555f81))
* typo ([3f5d79a](https://github.com/hirosystems/clarinet/commit/3f5d79ac1e57d88a50f263a7fc95600cf52ba11a))
* typo ([4b4794d](https://github.com/hirosystems/clarinet/commit/4b4794dede064e7dde24ec3e76aae1768af00599))
* typo ([7b11ecd](https://github.com/hirosystems/clarinet/commit/7b11ecdc702333f8e9bc3bc85613d325d451ad93))
* use 0.0.0.0 instead of localhost ([fd92b0b](https://github.com/hirosystems/clarinet/commit/fd92b0b8de0c5f1d10169d2c2216aa348652cca7))
* windows networking issues ([9c080f6](https://github.com/hirosystems/clarinet/commit/9c080f6a96be253458c392eecb0dc0c866edc08e))
* windows ui ([77fc359](https://github.com/hirosystems/clarinet/commit/77fc359503cfa0ab12858aa6a3e5fdbf0c3d429a))


### Features

* ability to add contract-call in deployment plans ([2e27031](https://github.com/hirosystems/clarinet/commit/2e2703105933d3be31a7df329452ced1de2f5e54))
* ability to have bitcoin transactions in deployment plans ([5e343d3](https://github.com/hirosystems/clarinet/commit/5e343d37d597d80d513fc409bb75f5a1d86031f9))
* ability to remap_principals in requirement-publish ops ([ac651f6](https://github.com/hirosystems/clarinet/commit/ac651f6c5b78c55f965ff729ded3c0751a7d1213))
* add chainhooks helpers in cli ([eb7c70e](https://github.com/hirosystems/clarinet/commit/eb7c70ed4b4ddf6414a0dc3ec2ab54308961eefe))
* add openapi spec ([dd9ccaf](https://github.com/hirosystems/clarinet/commit/dd9ccafaa79a8d9ebe6c41f710cb2fd169ec47b7))
* add wasm feature flag to clarinet-deployments ([4c78144](https://github.com/hirosystems/clarinet/commit/4c78144237936c087becd71b8c1614c9d5147a5e))
* add wasm feature flag to clarinet-files ([990dcf1](https://github.com/hirosystems/clarinet/commit/990dcf1c6c5e7da4f907071d85e1b543afddd8fe))
* allow empty console ([7453df5](https://github.com/hirosystems/clarinet/commit/7453df510a6f69a9074fd6381812ff30fbb2799b))
* automatically suggest principal remap ([ef548c9](https://github.com/hirosystems/clarinet/commit/ef548c976c7f5edeba7d2631cc2634c9f4d2f995))
* contract remap for devnet / testnet ([f0ac5b4](https://github.com/hirosystems/clarinet/commit/f0ac5b43bbc93a8322827d9d8e2c7bb29f7cc408))
* faucet + orchestrator adjustments ([681f51c](https://github.com/hirosystems/clarinet/commit/681f51c7f70eff553016cc8bdfe877e6bd051222))
* fix feature flags ([ea8aeea](https://github.com/hirosystems/clarinet/commit/ea8aeea356b93c8dd25b31396fbb07367b087d7e))
* introduce cbtc example ([e195f71](https://github.com/hirosystems/clarinet/commit/e195f71d8b25545f442b2030a8c803e732e41a05))
* show error diagnostics when testing ([f0580fb](https://github.com/hirosystems/clarinet/commit/f0580fb327b0505b03d9af34d9d33ece88b2a36f))
* use oreo as as a dockerized standalone component ([428e7b8](https://github.com/hirosystems/clarinet/commit/428e7b8473437317418458acfe14ad2b6af6ecfd))
* use oreo as library in clarinet ([82057d4](https://github.com/hirosystems/clarinet/commit/82057d431e00032c7d397d524223f935e786cea3))

## [0.31.1](https://github.com/hirosystems/clarinet/compare/v0.31.0...v0.31.1) (2022-05-27)


### Bug Fixes

* code coverage not including initial executions ([6089e86](https://github.com/hirosystems/clarinet/commit/6089e8604384d5e86bb48df47de0404cd2781cf0))
* display errors from parsing ([517a3fa](https://github.com/hirosystems/clarinet/commit/517a3fa7cfcb5a0640c2a0f19cdb16741e8ef970))
* explorer, noneCV ([3dd1142](https://github.com/hirosystems/clarinet/commit/3dd1142fb78433040f54dca6bd1c925e288b57f4))
* usage of default deployment files ([c6b1f8c](https://github.com/hirosystems/clarinet/commit/c6b1f8c757e6654f5ee6e27e821b8df924237bd9))
* use `clarinet@0.31.0` lib in generated tests ([74b1b99](https://github.com/hirosystems/clarinet/commit/74b1b99f8e121b067546645cc73b1104f2b5dc78)), closes [#381](https://github.com/hirosystems/clarinet/issues/381)
* wrong network ([a631268](https://github.com/hirosystems/clarinet/commit/a63126818f00a051ab8f98a3ec7bf6a1ede334c2))

# [0.31.0](https://github.com/hirosystems/clarinet/compare/v0.30.0...v0.31.0) (2022-05-24)


### Bug Fixes

* address feedbacks ([c03549d](https://github.com/hirosystems/clarinet/commit/c03549d1fa3804faa9fbd19f01e0933d0051ae0f))
* bug + associated name ambiguity ([4aaa881](https://github.com/hirosystems/clarinet/commit/4aaa881733af70a3c8856a36be9d504cd756dcba))
* clarinet test --watch ([b985f32](https://github.com/hirosystems/clarinet/commit/b985f3255dddcc2714c605692655f499d0a5794e))
* cost reports ([c26b063](https://github.com/hirosystems/clarinet/commit/c26b0635b30b0256a3cadeb73c647db61648b087))
* deno interface import ([2978301](https://github.com/hirosystems/clarinet/commit/29783019328984f32377aaff4e07ee3f498da96d))
* devnet deployments ([49e1701](https://github.com/hirosystems/clarinet/commit/49e1701e172f9bb916d073ddce236e18178781c0))
* lsp integration tests, streamline deployment plan serde attributes ([8dff7e0](https://github.com/hirosystems/clarinet/commit/8dff7e0ad7b2e671b259db729cb8fe2bab855651))
* node-binding build ([14a651d](https://github.com/hirosystems/clarinet/commit/14a651dd7c0eab677488c37f1d79261364bef3b1))
* test relying on hashmap ([f84f67e](https://github.com/hirosystems/clarinet/commit/f84f67e7a03bb5ca076da66906c5252b8af8f7c8))
* test return code ([50c39b8](https://github.com/hirosystems/clarinet/commit/50c39b87f74a3a66aa2fa78c98701a318fac9826))
* tests ([c3fd59b](https://github.com/hirosystems/clarinet/commit/c3fd59b9db246b1acb6933f6c59becb6523d89ef))
* unable to resolve dependencies in presence of boot contracts ([5df8330](https://github.com/hirosystems/clarinet/commit/5df8330a79fd390903dfedd6a49380885bfa0edc))
* unordered contracts ([bf8f86f](https://github.com/hirosystems/clarinet/commit/bf8f86f2dee334e15890739dd4212a715869812b))
* update doc ([5804c4b](https://github.com/hirosystems/clarinet/commit/5804c4bd3c7d7f34200cb0424ea5edbc16a58ca2))
* using v0.100.0 instead of v1.0.0-beta1 ([1025e32](https://github.com/hirosystems/clarinet/commit/1025e3222150740e357c6f9e908c7bb9653056cf))
* warnings ([cb97106](https://github.com/hirosystems/clarinet/commit/cb9710630cdd0689f11fcaf31e849f1a6bf12f88))
* windows builds ([d020d57](https://github.com/hirosystems/clarinet/commit/d020d57e427c493ef01051acb188cfb8eec1b754))


### Features

* add telemetry for DAP debugger ([c7a29f5](https://github.com/hirosystems/clarinet/commit/c7a29f5dc3962f53ecf1ea9e459f86fbcab3a692))
* cascade changes in clarinet integrate ([0fec1ed](https://github.com/hirosystems/clarinet/commit/0fec1ed7f836b75f4803bcb814482d218a8bf842))
* cascade changes in clarinet test ([b719741](https://github.com/hirosystems/clarinet/commit/b71974156470e5028cc49749bec07c397f68e4ae))
* cascade changes in cli interface ([4ca4024](https://github.com/hirosystems/clarinet/commit/4ca4024a183f1f92cb6313d16f1d787abdaa914a))
* cascade changes in lsp ([56b0322](https://github.com/hirosystems/clarinet/commit/56b03225560481e8a98fb383ea78104ec088db18))
* **dap:** implement DAP debugger ([60b7145](https://github.com/hirosystems/clarinet/commit/60b7145982294c87f45bce1732f30a755d88d9eb))
* **dap:** WIP implementation of DAP interface ([270c5a7](https://github.com/hirosystems/clarinet/commit/270c5a7cf4632122c30ebe336f776f6379465e65))
* enable multithreading ([67b7d1c](https://github.com/hirosystems/clarinet/commit/67b7d1cbcd75f99f75403354373c9e7e68e06c53))
* improve protocol deployment timing on devnet ([b61b726](https://github.com/hirosystems/clarinet/commit/b61b7265013cf130412247a52c8fbf9a600f04f5))
* initial setup for DAP debugger ([8ab5837](https://github.com/hirosystems/clarinet/commit/8ab58371412dc226c5e40c1e7dc2105f0dc58156))
* introduce deployments ([fa83d83](https://github.com/hirosystems/clarinet/commit/fa83d839bb8f583f6ba170b6f115ded03f735243))
* introduce notion of simnet ([c0085ac](https://github.com/hirosystems/clarinet/commit/c0085ac894093360d89e034d24f80ed969a0b122))
* support new interface to dependency checker ([c3f8db4](https://github.com/hirosystems/clarinet/commit/c3f8db4432cb5a6a9e1c6d6ff816e98d892f1abc))
* type updates ([10a5f16](https://github.com/hirosystems/clarinet/commit/10a5f16fd803494542c6ca4b2b84eb3a9cb4f8e7))
* update deno layer ([98de4e8](https://github.com/hirosystems/clarinet/commit/98de4e81b22c56f45a800ed991cd68c43346fd7d))
* update to use repl with DAP support ([472de80](https://github.com/hirosystems/clarinet/commit/472de80d5034cf5f5500652daa372fc22c14cfe3))

# [0.30.0](https://github.com/hirosystems/clarinet/compare/v0.29.1...v0.30.0) (2022-05-13)


### Bug Fixes

* unordered contracts ([4cc54c7](https://github.com/hirosystems/clarinet/commit/4cc54c772299d4d034a87f13b5c20997b59359a4))


### Features

* add telemetry for DAP debugger ([b1511e6](https://github.com/hirosystems/clarinet/commit/b1511e6427abffa14b4dda22b222aa856e575a32))
* **dap:** implement DAP debugger ([6bcec16](https://github.com/hirosystems/clarinet/commit/6bcec165e6029be4074d1350f783a2a6bf7fb852))
* **dap:** WIP implementation of DAP interface ([2600cb1](https://github.com/hirosystems/clarinet/commit/2600cb1c2bf54164af6181312e51cf9288600828))
* initial setup for DAP debugger ([cf352d2](https://github.com/hirosystems/clarinet/commit/cf352d2a0a1c88ba615afb946ba2862ba19a2100))
* support new interface to dependency checker ([223a158](https://github.com/hirosystems/clarinet/commit/223a158885d21f473e305358483d0fc1c05ddd16))
* update to use repl with DAP support ([f937bb2](https://github.com/hirosystems/clarinet/commit/f937bb2b52560c55d4df74113d0a4e83cfd69d62))

## [0.29.1](https://github.com/hirosystems/clarinet/compare/v0.29.0...v0.29.1) (2022-05-03)


### Bug Fixes

* fixed problem with contract ordering in lsp ([12bccc5](https://github.com/hirosystems/clarinet/commit/12bccc5db45a230f7faa91a4ab784a4161c3a135))

# [0.29.0](https://github.com/hirosystems/clarinet/compare/v0.28.1...v0.29.0) (2022-04-21)


### Bug Fixes

* add new costs synthesis table ([d8f5f29](https://github.com/hirosystems/clarinet/commit/d8f5f2939521d56f71db06a0b4ab4ef718b16fcf))
* Fix issue with telemetry prompt on windows ([0af8fe9](https://github.com/hirosystems/clarinet/commit/0af8fe997dbb61fe335fe4bb607d6f0147639a66)), closes [#317](https://github.com/hirosystems/clarinet/issues/317)
* remove caveman dbg statement ([bd830f9](https://github.com/hirosystems/clarinet/commit/bd830f959897d755fe94fb1c94997b942154b91e))
* remove warnings about manifest file ([49edfd4](https://github.com/hirosystems/clarinet/commit/49edfd41136b2389054c47c37808663cbc8faa6d))


### Features

* add boot_contracts config ([c1cab93](https://github.com/hirosystems/clarinet/commit/c1cab93ccffa03366d25956634311a9b886a1957))
* add tx_per_block and improve formatting ([1fc6d4f](https://github.com/hirosystems/clarinet/commit/1fc6d4f56e311edfebc8378b684cedc5155e91a0))

## [0.28.1](https://github.com/hirosystems/clarinet/compare/v0.28.0...v0.28.1) (2022-04-06)


### Bug Fixes

* add `principal` type for completions ([1aa8fb7](https://github.com/hirosystems/clarinet/commit/1aa8fb74d099e06d4a14626588d6daafbf720652)), closes [#303](https://github.com/hirosystems/clarinet/issues/303)

# [0.28.0](https://github.com/hirosystems/clarinet/compare/v0.27.0...v0.28.0) (2022-03-31)


### Bug Fixes

* address https://github.com/hirosystems/clarinet/issues/279 ([69253d3](https://github.com/hirosystems/clarinet/commit/69253d3f8f974e4841b30b25233e63971d38434b))
* adjust some env variables ([6bd3436](https://github.com/hirosystems/clarinet/commit/6bd3436665ad7034e6dac2f707c0ed946b49aacb))
* attempt to repair Test workflow ([cf1d598](https://github.com/hirosystems/clarinet/commit/cf1d5988db214fbba6a943248066e4f59337b4d9))
* handle errors during file creation properly ([88b14b2](https://github.com/hirosystems/clarinet/commit/88b14b28b24877f5de9b61ad6b6290acb7b0beca)), closes [#229](https://github.com/hirosystems/clarinet/issues/229)
* try another nightly ([5694857](https://github.com/hirosystems/clarinet/commit/56948574d5d07f1dcdbe25c603a96991866867f8))
* upgrade @mapbox/node-pre-gyp from 1.0.6 to 1.0.8 ([0eff1a7](https://github.com/hirosystems/clarinet/commit/0eff1a75eb920e462b48d0a595806f73a61a9d56))
* upgrade typescript from 4.5.2 to 4.5.5 ([0129097](https://github.com/hirosystems/clarinet/commit/0129097cb2dfc1b3447edca02885f5f453b78075))


### Features

* **debugger:** add telemetry for debugger ([4438e23](https://github.com/hirosystems/clarinet/commit/4438e23430f3beee04ef0629c144b428adc55cf6))
* **deugger:** add debugger info to README ([3026df7](https://github.com/hirosystems/clarinet/commit/3026df7beee680bd2a4e12cb8a82a89123275e2d))
* improve clarinet integrate, clarinet contracts publish and testing harness reliability ([#240](https://github.com/hirosystems/clarinet/issues/240)) ([b9b6f74](https://github.com/hirosystems/clarinet/commit/b9b6f74a3c36b99bf816067785ca291062f8de20)), closes [#1](https://github.com/hirosystems/clarinet/issues/1) [#231](https://github.com/hirosystems/clarinet/issues/231)
* update interfaces for debugger in REPL ([38a89b5](https://github.com/hirosystems/clarinet/commit/38a89b5c6dd148488e61c9c7c7b63f90d2a0154b))
* use lib v0.28.0 ([354fecf](https://github.com/hirosystems/clarinet/commit/354fecf1dbc9fbc4ef47996b7710a631667596a6))

# [0.27.0](https://github.com/hirosystems/clarinet/compare/v0.26.1...v0.27.0) (2022-02-24)


### Features

* add shell completions files ([e6b1f25](https://github.com/hirosystems/clarinet/commit/e6b1f25683cb2c0e8f031e08fa0843fb0e5af690))
* add subcommand to generate shell completions ([a493b67](https://github.com/hirosystems/clarinet/commit/a493b6792515a6163a401f4c4a25802d98b76882)), closes [#146](https://github.com/hirosystems/clarinet/issues/146)
* clean up commands and improve help docs ([8f18182](https://github.com/hirosystems/clarinet/commit/8f18182ce659b9dd8969af0ac28c9a0a3b4c9278)), closes [#118](https://github.com/hirosystems/clarinet/issues/118)
* stacks-devnet-js support for linux-musl (non-glibc, e.g. alpine) ([6e322f1](https://github.com/hirosystems/clarinet/commit/6e322f1b668b9fcbc547e1053cf8d10807828c60))

## [0.26.1](https://github.com/hirosystems/clarinet/compare/v0.26.0...v0.26.1) (2022-02-13)


### Bug Fixes

* update to clarity-repl 0.22.1 ([644c082](https://github.com/hirosystems/clarinet/commit/644c082da056511ebe3c5d0d9e2eb10411b78c4b))

# [0.26.0](https://github.com/hirosystems/clarinet/compare/v0.25.1...v0.26.0) (2022-02-12)


### Bug Fixes

* fix serialization of REPL settings ([5fc9d08](https://github.com/hirosystems/clarinet/commit/5fc9d080cdb499d61d69498050773b7cbaba72fe))


### Features

* macos-arm64 / Apple Silicon / M1 builds ([da5f1bc](https://github.com/hirosystems/clarinet/commit/da5f1bc43a977156656d3036861c2ff4978d53bf))

## [0.25.1](https://github.com/hirosystems/clarinet/compare/v0.25.0...v0.25.1) (2022-02-11)


### Bug Fixes

* crash on `clarinet new contract` ([d236370](https://github.com/hirosystems/clarinet/commit/d2363700b0a7594ea54802496dd4244343a7e238)), closes [#246](https://github.com/hirosystems/clarinet/issues/246)

# [0.25.0](https://github.com/hirosystems/clarinet/compare/v0.24.0...v0.25.0) (2022-02-10)


### Bug Fixes

* clarity-repl resolve_link adjustment ([f1e8b47](https://github.com/hirosystems/clarinet/commit/f1e8b47814173acb6cd39ed5bf98a987ff8c97cf))
* link title typo ([20b5982](https://github.com/hirosystems/clarinet/commit/20b5982be084b0934a86938e7a67115c94c43168))


### Features

* add analysis settings support ([c7984e3](https://github.com/hirosystems/clarinet/commit/c7984e3036641ac95c65b6ae56f5b954641e282c)), closes [hirosystems/clarity-repl#62](https://github.com/hirosystems/clarity-repl/issues/62)
* add check-checker options to Clarinet.toml ([2f8ad7f](https://github.com/hirosystems/clarinet/commit/2f8ad7fdd962e8f94eb6871074edef86416993a2))
* add option to check single file ([64b0e2f](https://github.com/hirosystems/clarinet/commit/64b0e2fbe3f68c5c5dc412f30bbaba17a8f1c54a))
* add option to select parser version ([470734c](https://github.com/hirosystems/clarinet/commit/470734c634ab4f3dada4797af92a6b7c32155afd))
* consolidate repl settings in config file ([cfe7af3](https://github.com/hirosystems/clarinet/commit/cfe7af3a781ccc93896f0da4f3f5593924618205))
* update to clarity-repl 0.22.0 ([e2d996a](https://github.com/hirosystems/clarinet/commit/e2d996a9a1f6a211c5d867168b09de78349949a7))
* update to work with new parser ([ce8267a](https://github.com/hirosystems/clarinet/commit/ce8267ac94502a43f0bb3e2a2fe27b23c37ac97a))

# [0.24.0](https://github.com/hirosystems/clarinet/compare/v0.23.1...v0.24.0) (2022-01-19)


### Bug Fixes

* fixed broken links ([b0f073a](https://github.com/hirosystems/clarinet/commit/b0f073ac634f480c86b0b788c51057605b202c64))
* generate proper strings from contract results ([6b189c6](https://github.com/hirosystems/clarinet/commit/6b189c61b36f1f1ca755b86225916373565411bb)), closes [#213](https://github.com/hirosystems/clarinet/issues/213)
* only code-sign on release ([e10f3d1](https://github.com/hirosystems/clarinet/commit/e10f3d12eeae10e0fe809df4b2bdfbe97d33a26a))
* resolve bug with windows build in CI ([16ccd00](https://github.com/hirosystems/clarinet/commit/16ccd00110c6abde8f4e27b74ddd93f935028e36))


### Features

* added ignore and only options to clarinet test ([a57cc23](https://github.com/hirosystems/clarinet/commit/a57cc2373cfe25604290c59143469df649337657))

## [0.23.1](https://github.com/hirosystems/clarinet/compare/v0.23.0...v0.23.1) (2022-01-13)


### Bug Fixes

* issue with chunked information in Mempool section ([1dd1e22](https://github.com/hirosystems/clarinet/commit/1dd1e22e1f14f6d23a8cdfe364bebb88489b25cb))

# [0.23.0](https://github.com/hirosystems/clarinet/compare/v0.22.0...v0.23.0) (2022-01-07)


### Bug Fixes

* display warnings and notes ([e0c4e1c](https://github.com/hirosystems/clarinet/commit/e0c4e1c0a1bd41a1814225a2565607523cfb25be))
* off by one spans ([c724911](https://github.com/hirosystems/clarinet/commit/c724911e1f252c123d382947576014fa648638ee))


### Features

* display warning as warning in popups ([0ec64cf](https://github.com/hirosystems/clarinet/commit/0ec64cf4ea53ac169c7f798edca69be24d0c97f9))

# [0.22.0](https://github.com/hirosystems/clarinet/compare/v0.21.2...v0.22.0) (2022-01-05)


### Bug Fixes

* lsp adjustment ([67233f2](https://github.com/hirosystems/clarinet/commit/67233f29fb01a4f3faa192d1a0d62866471605b1))


### Features

* fail gracefully on Clarinet.toml errors ([3023320](https://github.com/hirosystems/clarinet/commit/3023320cfe919f925fbb0710d42811d12100e430))

## [0.21.2](https://github.com/hirosystems/clarinet/compare/v0.21.1...v0.21.2) (2021-12-23)


### Bug Fixes

* rollback to clarity-repl v0.18.0 ([c5db67b](https://github.com/hirosystems/clarinet/commit/c5db67be42915492f3a619156611d19133d5fb82))

## [0.21.1](https://github.com/hirosystems/clarinet/compare/v0.21.0...v0.21.1) (2021-12-23)


### Bug Fixes

* show errors when parsing opts ([2dad960](https://github.com/hirosystems/clarinet/commit/2dad96064c95b763a1df13b2ea56421acc527a27)), closes [#188](https://github.com/hirosystems/clarinet/issues/188)

# [0.21.0](https://github.com/hirosystems/clarinet/compare/v0.20.0...v0.21.0) (2021-12-22)


### Bug Fixes

* build error ([6399169](https://github.com/hirosystems/clarinet/commit/63991693e3f9ce33c7ee010a5419190d0ed2c7cf))
* caveman debug vestige ([a89c631](https://github.com/hirosystems/clarinet/commit/a89c631e3cd283094bd81d51c41379acb9bf28e3))
* clarity integration ([259fb4a](https://github.com/hirosystems/clarinet/commit/259fb4a0669d0c0e58b4192c1a21e187cff8572a))
* comments and doc ([ea269ae](https://github.com/hirosystems/clarinet/commit/ea269aefc980b6aea663ff1115cec4321c13179c))
* disable_telemetry flag ([8dcb0ec](https://github.com/hirosystems/clarinet/commit/8dcb0ec805a66570d8e10d1bb1e05e36e7cae43b))
* doc copy pasta ([8927144](https://github.com/hirosystems/clarinet/commit/8927144e7240049beadbfbc19c2671ba299d523f))
* enable telemetry in Dockerfile ([8b43980](https://github.com/hirosystems/clarinet/commit/8b43980a18ac9d996449b7873384236ae3be5c40))
* iterate on integration ([3a65165](https://github.com/hirosystems/clarinet/commit/3a651655b2aa1a39aafac6a38a83a082a612b48c))
* make tower-lsp optional ([1f83b21](https://github.com/hirosystems/clarinet/commit/1f83b2146205ed3ad8479c865ef8c2e0deb523bf))
* remove reference to Blockstack ([350be75](https://github.com/hirosystems/clarinet/commit/350be75a8368bf27dea36fa14275e24d5f3feca4))
* stacks-devnet-js build ([fe74897](https://github.com/hirosystems/clarinet/commit/fe74897b03b2b5a464e8f6a874f0a2f3a6311373))


### Features

* add `analysis` field into project config ([ce61daf](https://github.com/hirosystems/clarinet/commit/ce61dafb92048e8268cc0082250d838b9289163f))
* add key ([0631e72](https://github.com/hirosystems/clarinet/commit/0631e720d4e4260c94a725b4f78fbb0dad5cf242))
* implement telemetry as a gated feature ([8b04f8b](https://github.com/hirosystems/clarinet/commit/8b04f8ba5f1cb0017f28ab2f050cf68f2af8369b))
* implement telemetry helpers ([b1cbcc6](https://github.com/hirosystems/clarinet/commit/b1cbcc6d264f5dc7f49fef983ef167b330028e06))

# [0.20.0](https://github.com/hirosystems/clarinet/compare/v0.19.1...v0.20.0) (2021-12-09)


### Features

* document CI how-to ([a4079d9](https://github.com/hirosystems/clarinet/commit/a4079d96dd762ae85804bd1d8b116eac75447dac))

## [0.19.1](https://github.com/hirosystems/clarinet/compare/v0.19.0...v0.19.1) (2021-12-06)


### Bug Fixes

* update package name ([6477408](https://github.com/hirosystems/clarinet/commit/647740816577ba7faa978281ac83fc3485e929fa))

# [0.19.0](https://github.com/hirosystems/clarinet/compare/v0.18.3...v0.19.0) (2021-12-06)


### Bug Fixes

* add 128bit numbers support ([a87a99c](https://github.com/hirosystems/clarinet/commit/a87a99c99e8377d7cef0eeb331dd537702dae588))
* address feedbacks ([4a74c51](https://github.com/hirosystems/clarinet/commit/4a74c51f592ac68fd6948d7818fa5ff18f9eb7d2))
* better event management ([bd1e9c3](https://github.com/hirosystems/clarinet/commit/bd1e9c30e87b78af99178f4144a9ae47032d7f9f))
* build ([b8d1ac7](https://github.com/hirosystems/clarinet/commit/b8d1ac727dd6ce4d84032044ca3c6372cfe33774))
* deployment fee rate can be too low ([1fa7564](https://github.com/hirosystems/clarinet/commit/1fa756426f12689aace74b46a23e6bbaec10597d))
* deployment_fee_rate not being used ([940f17d](https://github.com/hirosystems/clarinet/commit/940f17dbb4e33fe315bf9e799503d61974d3a236))
* dropped request ([c777d91](https://github.com/hirosystems/clarinet/commit/c777d9122a5e0323e26be4c0c609fa391a1be5e3))
* error message ([bb1c5c7](https://github.com/hirosystems/clarinet/commit/bb1c5c7de89622fc04fff85172780d676501cfa7))
* fee too low (stacking orders) ([90d46b7](https://github.com/hirosystems/clarinet/commit/90d46b759dd17f22da7c12a3cae954523b7c62a7))
* ignore RUSTSEC-2021-0124 ([b4b570a](https://github.com/hirosystems/clarinet/commit/b4b570a20a7115a8b8ae3b5fe0616777beca2d3d))
* incorrect error message ([ac48d31](https://github.com/hirosystems/clarinet/commit/ac48d318a84bd0515640ec09fab22f8dd50cee3d))
* infinite iter ([1ab975a](https://github.com/hirosystems/clarinet/commit/1ab975aeee561baa6813f06db24f0025d532a3eb))
* nested runtimes / switch to futures ([26d60b7](https://github.com/hirosystems/clarinet/commit/26d60b7d9a04b6c5fcd7971fe0f64feaf7ea3d9f))
* package name ([e8dc71b](https://github.com/hirosystems/clarinet/commit/e8dc71bcb374f02c3c553d3d850ba7b2b68f0ab3))
* return type ([52a37bf](https://github.com/hirosystems/clarinet/commit/52a37bfc2cb23942c96b8696253180de96d81ecd))
* StacksRpc internal improvements ([2bf6c52](https://github.com/hirosystems/clarinet/commit/2bf6c52d44ccf2d4f68bee407fba61074286fa50))
* STXLockEventData unlock_height type ([927acfe](https://github.com/hirosystems/clarinet/commit/927acfe9ed4069725bc1b8313e968883bb6b8b62))
* testnet / mainnet deployment ([b49056b](https://github.com/hirosystems/clarinet/commit/b49056ba4db966c86086f05bf25ec03f4a02e423))
* tsconfig adjustments ([4f5320e](https://github.com/hirosystems/clarinet/commit/4f5320e23030099add3ee12eabbb92c58975d82f))
* typos ([669d94b](https://github.com/hirosystems/clarinet/commit/669d94b2c183f1ae641d77a61ff4125eda5c4a66))
* update lib name ([9041ff8](https://github.com/hirosystems/clarinet/commit/9041ff829f8edff19a50e8c7aa3e8bf017b70c07))
* update node bindings ([cf29e00](https://github.com/hirosystems/clarinet/commit/cf29e00a757a35b603d06b8140b343d3ba843f72))
* update types/node + package.json ([7fdb417](https://github.com/hirosystems/clarinet/commit/7fdb417fbe39c1e0d5d7f7124ceba03c9ecb8250))
* warnings, remove rustdoc ([c81d7a7](https://github.com/hirosystems/clarinet/commit/c81d7a7f0b887b9cb23642ec3232fd11a638d563))


### Features

* add expectFungibleTokenBurnEvent ([0a77641](https://github.com/hirosystems/clarinet/commit/0a77641d685d53eeda2db0e0c79e37415409a3a4))
* add expectNonFungibleTokenBurnEvent ([b33be1b](https://github.com/hirosystems/clarinet/commit/b33be1ba31d57f0845ef2e2c5439b40988dc0bf4))
* add expectNonFungibleTokenMintEvent ([1bcf5b1](https://github.com/hirosystems/clarinet/commit/1bcf5b1aa2eb7a574c68229c89c9cc5a38ad722b))
* add stacks / bitcoin specific metadata for blocks / transactions ([59d66ee](https://github.com/hirosystems/clarinet/commit/59d66eec7fdbc63f29bdf041caaedff28b22969e))
* closing the loop ([e80a6dc](https://github.com/hirosystems/clarinet/commit/e80a6dc434042307375639d9b831b06b79c0f3c8))
* expose get_stacks_node_url for stacks.js ([f1f8bed](https://github.com/hirosystems/clarinet/commit/f1f8bed146add3c0ecbfdaf0ab4db3b3b5b4c21a))
* improve api ([2f72f6d](https://github.com/hirosystems/clarinet/commit/2f72f6dc70142670a2ac0acbf11394d01f34518b))
* introduce new schemas for block / transaction handling ([01501d9](https://github.com/hirosystems/clarinet/commit/01501d97d4da55b64f090365353b3d4e8f2a73d6))
* migrate to typescript ([dd55845](https://github.com/hirosystems/clarinet/commit/dd55845ab554958a0e7b73fc447efdbb473a1c49))
* polish stacks operations schemas ([cb939f4](https://github.com/hirosystems/clarinet/commit/cb939f4b550514dbae5a27aafd5c254063c1dcd0))

# [0.18.0](https://github.com/hirosystems/clarinet/compare/v0.17.0...v0.18.0) (2021-10-18)


### Bug Fixes

* build settings ([2a0cf5f](https://github.com/hirosystems/clarinet/commit/2a0cf5f4365760c5b40ceb9bf8db9d6ebec2a80e))
* cargo build --locked in unit tests ([5f5f428](https://github.com/hirosystems/clarinet/commit/5f5f428078fc3e7d6818ce51b4577242df9b58fe))
* disable audits ([9036ccf](https://github.com/hirosystems/clarinet/commit/9036ccf9e63b6888228c535761e1ba5984f50623))
* disable STACKS_API_ENABLE_NFT_METADATA ([0ce043a](https://github.com/hirosystems/clarinet/commit/0ce043a59ef8bbe014f4d15bbb479c80d8dd6741))
* freeze nightly version ([1a88293](https://github.com/hirosystems/clarinet/commit/1a88293a20db656107528a96e3708f6c7f2d190f))
* freeze nightly version ([ecd58f0](https://github.com/hirosystems/clarinet/commit/ecd58f066fd04d8bede19a682d5af88d25616f31))
* freeze rocket ([bcd1e28](https://github.com/hirosystems/clarinet/commit/bcd1e289c5caaaf7697d3bd54d480e97ab9492fb))
* re-enable audits, ignore RUSTSEC-2020-0159, RUSTSEC-2021-0119, RUSTSEC-2020-0071 ([f853221](https://github.com/hirosystems/clarinet/commit/f853221cf73fe542d309f7b6b83ba62f569e0fe7))


### Features

* add expectPrintEvent function ([e835c5a](https://github.com/hirosystems/clarinet/commit/e835c5a9c7294b420e74abac0d216deb93d1d8c9))
* add hints ([c067cfb](https://github.com/hirosystems/clarinet/commit/c067cfb97da46f60f2b3b9783af8ab0b51a1f3d0))
* enable ft/nft metadata ([08f0da5](https://github.com/hirosystems/clarinet/commit/08f0da5d465feb7b800a41942f42b6e0d31a5b2c))
* introduce deployment fee rate ([225aac2](https://github.com/hirosystems/clarinet/commit/225aac2e428b8cbb025f5f224a171e4f8bfffdc2))
* iterate on feedbacks ([dbf98c2](https://github.com/hirosystems/clarinet/commit/dbf98c214ec02836b7212ac8ac7f0edb30b5b588))
* update colors and messages ([587671f](https://github.com/hirosystems/clarinet/commit/587671f3e40483c61207e561155f1af7864f7c74))

# [0.17.0](https://github.com/hirosystems/clarinet/compare/v0.16.0...v0.17.0) (2021-10-05)


### Features

* ability to cache repl sessions ([5e086cb](https://github.com/hirosystems/clarinet/commit/5e086cbf5bf173db034eea256bf617e3ab5efdee))
* acknowledge check ok ([f6a6143](https://github.com/hirosystems/clarinet/commit/f6a6143f7cb1ce2157ca8d0800588bee7797d428))
* add decrement function ([#109](https://github.com/hirosystems/clarinet/issues/109)) ([56e5477](https://github.com/hirosystems/clarinet/commit/56e54770e1a6965686554e6a872715ee8f0cca85))
* cascade changes ([fb6d9a9](https://github.com/hirosystems/clarinet/commit/fb6d9a9224417d4078c1dbbc46c7a874d623651b))
* cost reporting via tests ([#116](https://github.com/hirosystems/clarinet/issues/116)) ([a0117aa](https://github.com/hirosystems/clarinet/commit/a0117aad7ab17e4c36c0568fb10c0b631d39c263))
* create abi-generator extension ([e5b46c4](https://github.com/hirosystems/clarinet/commit/e5b46c4e6865adb1e6524e7865508ab7a35aa865))
* revamp extension usage ([ec8cdfc](https://github.com/hirosystems/clarinet/commit/ec8cdfc9fb86bbc5bbdb381fa10982dd2ba82581))

## [0.15.1](https://github.com/hirosystems/clarinet/compare/v0.15.0...v0.15.1) (2021-08-18)


### Bug Fixes

* attempt to address compatibility with Linux ([fdae2b2](https://github.com/hirosystems/clarinet/commit/fdae2b2930363fd6765b03ccfef7051270059ca0))
* attempt to fix linux compatibility ([832dd16](https://github.com/hirosystems/clarinet/commit/832dd16dc2ac4b8e51f06df3e11078dfaaa991de))
* hard code host ip address ([e81e711](https://github.com/hirosystems/clarinet/commit/e81e7113dd7fdad2bb439700c19a4e11ad1bd1b1))
* postgres port handling ([c716669](https://github.com/hirosystems/clarinet/commit/c716669a739b99b8fdc78ac951718021885e8891))
* use nightly for tests ([f76098d](https://github.com/hirosystems/clarinet/commit/f76098dbddda254b67953b4e9af5218dee06ea7b))
* use stable in github actions ([31fa5e1](https://github.com/hirosystems/clarinet/commit/31fa5e1348414e03def9b5030c59cd1874374383))

# [0.15.0](https://github.com/hirosystems/clarinet/compare/v0.14.2...v0.15.0) (2021-08-11)


### Bug Fixes

* better process termination ([1feec61](https://github.com/hirosystems/clarinet/commit/1feec612bbdc91e689a1671f86f5774f8b8e504f))
* cross-platform filesystem issues ([05d6d77](https://github.com/hirosystems/clarinet/commit/05d6d77786660ed7f719cd7522c3432e562db348))
* cross-platform networking ([35511e7](https://github.com/hirosystems/clarinet/commit/35511e7ef8792ef42dd474208367fac349ccc255))
* cross-platform UI issues ([1b7f9e8](https://github.com/hirosystems/clarinet/commit/1b7f9e8a11aea44dca99c55f1f38eee32f5d200d))
* docker prune at startup in case of dirty state ([95a7fd1](https://github.com/hirosystems/clarinet/commit/95a7fd1aaa128fef4c94628dec610f1e83d870ea))
* don't crash if /v2/info is unresponsive ([639fc58](https://github.com/hirosystems/clarinet/commit/639fc58a0b31464b232ed29ba13fb3c7f048aea9))
* failing CI steps ([aaeb443](https://github.com/hirosystems/clarinet/commit/aaeb4430011c38a5daf0e6531a71ae42d9419600))
* handle projects with > 25 contracts ([eb4d3ef](https://github.com/hirosystems/clarinet/commit/eb4d3efb4f3deb7449a1817d0bc392106775bcb9))
* pox initialization ([6f8f16a](https://github.com/hirosystems/clarinet/commit/6f8f16a06f1066844aace7e9c970f39f1a452f35))
* tty -> none ([878f6a9](https://github.com/hirosystems/clarinet/commit/878f6a977cc0d8525c58015e7518efc048c6d8dc))
* use nightly ([55cbc77](https://github.com/hirosystems/clarinet/commit/55cbc771ae1f0c881c9fb692340eeb5c3df88e29))
* use nightly in Dockerfile ([12262ea](https://github.com/hirosystems/clarinet/commit/12262eae82893641489117817d7f82d2193b7d97))
* use nightly toolchain ([13ee9bb](https://github.com/hirosystems/clarinet/commit/13ee9bb35acaec12d21a641d33f0ef1c70f3b8e4))
* warnings ([30e7438](https://github.com/hirosystems/clarinet/commit/30e743841af179f7988af67604da39b0319b11a7))
* warnings ([0c9f7a2](https://github.com/hirosystems/clarinet/commit/0c9f7a2af8768688f41b51c0cf2cfa301a1c7099))


### Features

* ability to disable dashboard ([32ccaff](https://github.com/hirosystems/clarinet/commit/32ccaff18d3e1832bd352feb58c12facb5c279ff))
* ability to disable explorers ([483a853](https://github.com/hirosystems/clarinet/commit/483a853ada99976a0bc6a7f1a0be11eb1588d634))
* ability to reset devnet ([1c0e1f2](https://github.com/hirosystems/clarinet/commit/1c0e1f2ab65e1c9db0baf63e79585d4d706aa005))
* add alias for poke -> console ([84aea86](https://github.com/hirosystems/clarinet/commit/84aea86cae825e57950cf69b87b145732c7efb7a))
* add bitcoin explorer ([446a4d4](https://github.com/hirosystems/clarinet/commit/446a4d4976803a042ed7855a2d979d1100b13550))
* better support for Devnet / Testnet / Mainnet ([9f47c73](https://github.com/hirosystems/clarinet/commit/9f47c73da62d99a955e27a575aa3b13b39a43cc4))
* devnet overall stable ([38bcc49](https://github.com/hirosystems/clarinet/commit/38bcc49f2ab8764b2fc32021f6a043c91eb2955a))
* display decoded transactions ([0fcb597](https://github.com/hirosystems/clarinet/commit/0fcb597ae9c539238c4c0e8bae3c4573702324fa))
* draft auto stack-stx ([c93623b](https://github.com/hirosystems/clarinet/commit/c93623b7a9492a9e35c51dc93da3d43be118b734))
* handle microblocks ([c68ac68](https://github.com/hirosystems/clarinet/commit/c68ac68493076e6c7405ab42fb8500523469a32a))
* improve termination sequence ([fbfd2c6](https://github.com/hirosystems/clarinet/commit/fbfd2c64df5c1310ae679b34c81d2d88e84e8f7e))
* integrating pox ([17c521c](https://github.com/hirosystems/clarinet/commit/17c521c26499c80ff81745237b602312f774474d))
* interface prototyped ([d42c05d](https://github.com/hirosystems/clarinet/commit/d42c05de025456989f36062c57f6ceb694a7e6a2))
* log container ids ([ed124a9](https://github.com/hirosystems/clarinet/commit/ed124a9f775b45f7dfa28f1e6648cddab701e0b4))
* update mempool ([adcb1be](https://github.com/hirosystems/clarinet/commit/adcb1bef9d55aa4196d1130c85f77bd8cb7f69fb))
* update services statuses ([f1865e4](https://github.com/hirosystems/clarinet/commit/f1865e47b4fc32c9898069eff40dc537eabbc40e))
* write logs to disk ([fa82cd9](https://github.com/hirosystems/clarinet/commit/fa82cd9aca9c623635dac003b06d91fb40fab784))

## [0.14.2](https://github.com/hirosystems/clarinet/compare/v0.14.1...v0.14.2) (2021-07-20)


### Bug Fixes

* enforce cache eviction ([6ef7d63](https://github.com/hirosystems/clarinet/commit/6ef7d631b9e0661ae0c5bf4e0aa03c5dabfae4d7))
* un-hard code path (clarinet deploy) ([b3e933a](https://github.com/hirosystems/clarinet/commit/b3e933a1aaa58a656eebfb29624c51cf48ba18ab))

## [0.14.1](https://github.com/hirosystems/clarinet/compare/v0.14.0...v0.14.1) (2021-06-28)


### Bug Fixes

* display typescript errors ([e7af34b](https://github.com/hirosystems/clarinet/commit/e7af34b3061f8afa42d0788d5a47a149fb3885d6))
* new contract generator ([b8e39d7](https://github.com/hirosystems/clarinet/commit/b8e39d7aac1e8943f1c1a9ebb93f630688e9067a))
* remove required -- for clarinet test ([0182b07](https://github.com/hirosystems/clarinet/commit/0182b07a051001073f12573d2676e54fc833fca0))

# [0.14.0](https://github.com/hirosystems/clarinet/compare/v0.13.0...v0.14.0) (2021-06-25)


### Bug Fixes

* implement tx.transferSTX ([4974e85](https://github.com/hirosystems/clarinet/commit/4974e8592e208f619e9aca029db42625dffa09bf))
* turn manifest-path into optional argument ([bb44856](https://github.com/hirosystems/clarinet/commit/bb44856fd01abd4c4edbdd3ba92520f993d13772))


### Features

* add allow-wallets option to clarinet run ([2690879](https://github.com/hirosystems/clarinet/commit/269087994e0f4501485a1081fc55d8acfe3d4eea))
* better manifest-path handling ([6ecdfb0](https://github.com/hirosystems/clarinet/commit/6ecdfb063012a885f7ae62385d12a1089b2333a5))
* polish logs (tests vs scripts) ([2810a92](https://github.com/hirosystems/clarinet/commit/2810a92f22f3922b62242c38446d9f831a64177f))
