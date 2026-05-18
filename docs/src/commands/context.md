# context

Outputs a minified JSON summary of the project state to stdout. Intended primarily for agents —
run it to give an AI assistant full context about the project contents.

```sh
banco context           # or: banco ctx
banco context --pretty  # pretty-print the JSON output
```

## Output format

```json
{
  "project": "name of the dir",
  "providers": [
    {
      "name": "local",
      "modules": [
        {
          "name": "notes",
          "parameters": [
            {
              "name": "label",
              "type": "string",
              "description": "Optional nested path used as a tag (e.g. meetings/2026)"
            }
          ],
          "items": [
            {
              "name": "My first note",
              "label": "meetings"
            }
          ]
        }
      ]
    }
  ]
}
```
