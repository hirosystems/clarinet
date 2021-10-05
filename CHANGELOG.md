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
