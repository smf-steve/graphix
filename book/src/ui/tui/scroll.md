# scrollbar

The `scrollbar` widget adds a visual scrollbar indicator to scrollable content, making it clear when content extends beyond the visible area and showing the current scroll position.

## Function Signature

```
/// Wraps a widget with a scrollbar indicator
val scrollbar: fn(
    #position: &i64,
    ?#content_length: &i64,
    ?#size: &Size,
    Widget
) -> Widget;
```

## Parameters

- **position** (required) - Current scroll position (typically the Y offset)
- **content_length** - Total length of the content (auto-detected if not specified)
- **size** (output) - Rendered size of the scrollbar area

## Examples

### Basic Usage

```graphix
use tui;
use tui::scrollbar;
use tui::paragraph;

let position = 0;
let content = paragraph(
    #scroll: &{x: 0, y: position},
    &long_text
);

scrollbar(
    #position: &position,
    &content
)
```

### Scrollable Paragraph

```graphix
let long_text = "Very long text content...";
let position = 0;
let max_position = 100;

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Up if position > 0 => {
                position <- (k ~ position) - 1;
                `Stop
            },
            k@`Down if position < max_position => {
                position <- (k ~ position) + 1;
                `Stop
            },
            k@`PageUp if position > 10 => {
                position <- (k ~ position) - 10;
                `Stop
            },
            k@`PageDown if position < (max_position - 10) => {
                position <- (k ~ position) + 10;
                `Stop
            },
            k@`Home => { position <- k ~ 0; `Stop },
            k@`End => { position <- k ~ max_position; `Stop },
            _ => `Continue
        },
        _ => `Continue
    },
    _ => `Continue
};

input_handler(
    #handle: &handle_event,
    &block(
        #border: &`All,
        #title: &line("Scrollable Content"),
        &scrollbar(
            #position: &position,
            #content_length: &max_position,
            &paragraph(#scroll: &{x: 0, y: position}, &long_text)
        )
    )
)
```

### Scrollable List

```graphix
let items = [...];  // Large array
let selected = 0;
let scroll_pos = 0;
let visible = 10;

// Auto-scroll to keep selection visible
scroll_pos <- select selected {
    s if s < scroll_pos => s,
    s if s >= (scroll_pos + visible) => s - visible + 1,
    _ => never()
};

scrollbar(
    #position: &scroll_pos,
    &list(
        #scroll: &scroll_pos,
        #selected: &selected,
        &items
    )
)
```

### Auto-scroll to Bottom

```graphix
let lines = [];
let new_line = net::subscribe("/logs/app")?;

lines <- array::window(#n: 100, new_line ~ lines, cast<string>(new_line)?);

let line_count = array::len(lines);
let visible_lines = 20;
let scroll_pos = max(0, line_count - visible_lines);

scrollbar(
    #position: &scroll_pos,
    #content_length: &line_count,
    &paragraph(
        #scroll: &{x: 0, y: scroll_pos},
        &array::fold(lines, "", |acc, l| "[acc]\n[l]")
    )
)
```

## Common Mistake

Remember to apply the scroll position to both the scrollbar and the content:

```graphix
// Wrong - scroll position not applied to content
scrollbar(
    #position: &position,
    &paragraph(&text)  // Missing #scroll parameter!
)

// Correct - scroll position applied to both
scrollbar(
    #position: &position,
    &paragraph(#scroll: &{x: 0, y: position}, &text)
)
```

## Scroll Behavior

Standard scroll keys:
- `Up`/`Down` - Scroll one line
- `PageUp`/`PageDown` - Scroll one page (typically 10-20 lines)
- `Home` - Scroll to top
- `End` - Scroll to bottom

## See Also

- [paragraph](paragraph.md) - For scrollable text content
- [list](list.md) - For scrollable lists
- [table](table.md) - For scrollable tables
- [block](block.md) - For containing scrollable content
