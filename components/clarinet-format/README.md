# Clarity Formatter

Clarity format provides a consistent, opinionated, and readable formatting for your clarity smart contracts and is accessible from CLI (`clarinet fmt`) and VSCode/LSP.

For examples of well-formatted contracts, check `components/clarinet-format/tests/golden-intended`

By default lines will be wrapped at 80 characters and indents will be 2 spaces but both are configurable.

### LSP / VSCode integration

From VSCode or using LSP you can configure formatting on-demand or on-save. There is also "Format Section" which formats only the highlighted section. Format-on-save can be enabled in VSCode settings and is off by default.

![Screenshot 2025-03-31 at 8 45 24 AM](https://github.com/user-attachments/assets/85a9544e-cc1b-4aee-8d73-81c57dcb2c91)

### CLI

The cli provides everything the LSP does with the added benefit of allowing you to format all the contracts within a project based on the manifest (configurable if you have a custom location with `--manifest-path`)

To format a specific contract with tab indenting and print out the results without changing the file:

```
clarinet fmt -f contracts/traits.clar --tabs --dry-run
```

To overwrite the contents of a file with a format using 120 maximum characters per line, and 4 spaces per indent:

```
clarinet fmt -f contracts/traits.clar -i 4 --max-line-length 120 --in-place

```

You can use `fmt` or `format`, they're aliases.

```
Usage: clarinet format [OPTIONS]

Options:
  -m, --manifest-path <MANIFEST_PATH>
  -f, --file <FILE>                        If specified, format only this file
  -l, --max-line-length <MAX_LINE_LENGTH>
  -i, --indent <INDENTATION>               indentation size, e.g. 2
  -t, --tabs                               use tabs instead of spaces
      --dry-run                            Only echo the result of formatting
      --in-place                           Replace the contents of a file with the formatted code
```
