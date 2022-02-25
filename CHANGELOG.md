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
