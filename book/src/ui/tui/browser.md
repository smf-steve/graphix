# browser

The `browser` widget provides a specialized interface for browsing and interacting with netidx hierarchies. It displays netidx paths in a tree structure with keyboard navigation, selection, and cursor movement support.

## Function Signatures

```
type MoveCursor = [`Left(i64), `Right(i64), `Up(i64), `Down(i64)];

/// Creates a browser widget for navigating netidx hierarchies
val browser: fn(
    ?#cursor: MoveCursor,
    ?#selected_row: &string,
    #selected_path: &string,
    ?#size: &Size,
    string
) -> Widget;
```

## Parameters

- **cursor** - Programmatic cursor movement: `Left(n)`, `Right(n)`, `Up(n)`, `Down(n)`
- **selected_row** (output) - Display name of the selected row
- **selected_path** (output, required) - Full path of the currently selected item
- **size** (output) - Rendered size of the browser

## Examples

### Basic Usage

```graphix
use tui;
use tui::browser;

let selected_path: string = never();

browser(
    #selected_path: &selected_path,
    "/"  // Start browsing from root
)
```

### Basic Navigation

```graphix
let path = "/";
let selected_path: string = never();
let cursor: MoveCursor = never();

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            e@ `Up => { cursor <- e ~ `Up(1); `Stop },
            e@ `Down => { cursor <- e ~ `Down(1); `Stop },
            e@ `Left => { cursor <- e ~ `Left(1); `Stop },
            e@ `Right => { cursor <- e ~ `Right(1); `Stop },
            e@ `Enter => { path <- e ~ selected_row; `Stop },
            _ => `Continue
        },
        _ => `Continue
    },
    _ => `Continue
};

input_handler(
    #handle: &handle_event,
    &browser(#cursor, #selected_path: &selected_path, path)
)
```

## See Also

- [list](list.md) - For simpler selection interfaces
- [table](table.md) - For tabular data display
- [block](block.md) - For containing browsers with borders
