# paragraph

The `paragraph` widget displays multi-line text with automatic word wrapping and scrolling support. It's ideal for displaying long text content, logs, or any content that needs to flow across multiple lines.

## Function Signature

```
type ScrollPosition = {x: i64, y: i64};

/// Creates a paragraph widget with text content
val paragraph: fn(
    ?#scroll: &ScrollPosition,
    ?#alignment: &Alignment,
    ?#wrap: &bool,
    [string, Text]
) -> Widget;
```

## Parameters

- **scroll** - Record with `x` and `y` fields for scroll position
- **alignment** - `Left`, `Center`, or `Right`
- **wrap** - Enable/disable word wrapping (default: true)

## Examples

### Basic Usage

```graphix
use tui;
use tui::paragraph;

paragraph(&"This is a simple paragraph. It will automatically wrap to fit the available width.")
```

### Scrollable Content

```graphix
let long_text = "I've got a lovely bunch of coconuts... [very long text continues]";
let scroll_y = 0;

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Up if scroll_y > 0 => {
                scroll_y <- (k ~ scroll_y) - 1;
                `Stop
            },
            k@`Down if scroll_y < 100 => {
                scroll_y <- (k ~ scroll_y) + 1;
                `Stop
            },
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
        #title: &line("Scrollable Text"),
        &paragraph(
            #scroll: &{x: 0, y: scroll_y},
            &long_text
        )
    )
)
```

### Live Log Viewer

Display real-time updating content:

```graphix
let log_entries = [];
let new_entry = net::subscribe("/logs/application")?;

log_entries <- array::window(
    #n: 100,
    new_entry ~ log_entries,
    cast<string>(new_entry)?
);

let log_text = array::fold(log_entries, "", |acc, entry| "[acc]\n[entry]");

paragraph(&log_text)
```

### Centered Message

```graphix
paragraph(
    #alignment: &`Center,
    &text(&[
        line(""),
        line(#style: style(#fg: `Yellow, #add_modifier: `Bold), "Welcome"),
        line(""),
        line("Press any key to continue")
    ])
)
```

## Word Wrapping

The paragraph widget automatically wraps long lines to fit the available width. Word boundaries are respected, so words won't be split in the middle unless they're longer than the available width.

## See Also

- [text](text.md) - For creating styled text content
- [scrollbar](scroll.md) - For adding scrollbars
- [block](block.md) - For containing paragraphs with borders
- [list](list.md) - For line-by-line selectable content
