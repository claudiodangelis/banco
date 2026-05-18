# Bookmarks

Bookmarks are Markdown files that store a URL along with an optional label for organization.
The URL is stored on the first line of the file; the rest is freeform content.

Bookmarks support the `browse` command — selecting a bookmark opens its URL in the system
browser.

## Parameters

| Parameter | Type   | Required | Description                                               |
| --------- | ------ | -------- | --------------------------------------------------------- |
| `label`   | string | no       | Nested path used as a tag (e.g. `tools/rust`)             |
| `url`     | string | yes      | The URL to bookmark                                       |
