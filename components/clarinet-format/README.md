# Clarity Formatter

Clarity format provides a consistent, opinionated, and readable formatting for your clarity smart contracts and is accessible from CLI (`clarinet fmt`) and VSCode/LSP.

For examples of well-formatted contracts, check `components/clarinet-format/tests/golden-intended`

By default lines will be wrapped at 80 characters and indents will be 2 spaces but both are configurable.

### LSP / VSCode integration

From VSCode or using LSP you can configure formatting on-demand or on-save. There is also "Format Section" which formats only the highlighted section. Format-on-save can be enabled in VSCode settings and is off by default.

### CLI

The cli provides everything the LSP does with the added benefit of allowing you to format all the contracts within a project based on the manifest (configurable if you have a custom location with `--manifest-path`)

```
  -m, --manifest-path <MANIFEST_PATH>
  -f, --file <FILE>                        If specified, format only this file
  -l, --max-line-length <MAX_LINE_LENGTH>
  -i, --indent <INDENTATION>               indentation size, e.g. 2
  -t, --tabs                               use tabs instead of spaces
      --dry-run                            Only echo the result of formatting
      --in-place                           Replace the contents of a file with the formatted code
```
