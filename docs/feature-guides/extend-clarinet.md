---
title: Extend Clarinet
---

Extend Clarinet to integrate clarity contracts with your own tooling and workflow.

*Topics covered in this guide*:

* [Use clarinet run command](#clarinet-run)
* [Standalone plugin deployment](#standalone-plugin)

## Clarinet run

| Name                      | wallet access | disk write | disk read | Deployment                                                            | Description                                                                                                                                       |
| ------------------------- | ------------- | ---------- | --------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| stacksjs-helper-generator | no            | yes        | no        | https://deno.land/x/clarinet@v0.29.0/ext/stacksjs-helper-generator.ts | Facilitates contract integration by generating some typescript constants that can be used with stacks.js. Never hard code a stacks address again! |
|                           |               |            |           |                                                                       |

Extensions are run with the following syntax:

```
clarinet run --allow-write https://deno.land/x/clarinet@v0.29.0/ext/stacksjs-helper-generator.ts
```

## Standalone plugin

An extension can be deployed as a standalone plugin on [Deno](https://deno.land/), or it can also just be a local file if it includes sensitive/private setup information.

As illustrated in the example above, permissions (wallet / disk read / disk write) are declared using command flags. If, at runtime, the Clarinet extension is trying to write to disk, read from disk, or access wallets without permission, the script will end up failing.
