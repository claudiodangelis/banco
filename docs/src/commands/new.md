# new

Creates a new item in a module that supports the `new` capability.

```sh
banco new note -l 'label=some/nested/path' -n 'My note'
```

Pass `-n` for the item name and `-l key=value` for each label. Run without flags to use the
interactive TUI, which prompts for all required fields and offers to open the new item in
`$EDITOR` when done.

When passing a value for an `enum` label via `-l`, the value must be one of the allowed strings
defined by the module. Passing an invalid value will cause the command to fail with an error.
