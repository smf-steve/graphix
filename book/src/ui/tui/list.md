# list

The `list` widget displays a scrollable, selectable list of items with keyboard navigation support. It's perfect for menus, file browsers, option selectors, and any interface that requires choosing from a list of items.

## Function Signature

```
/// Creates a list widget from an array of lines
val list: fn(
    ?#selected: &i64,
    ?#scroll: &i64,
    ?#highlight_style: &Style,
    ?#highlight_symbol: &string,
    ?#repeat_highlight_symbol: &bool,
    ?#style: &Style,
    Array<Line>
) -> Widget;
```

## Parameters

- **selected** - Index of the currently selected item (0-indexed)
- **scroll** - Scroll position (offset from the top)
- **highlight_style** - Style for the selected item
- **highlight_symbol** - String displayed before selected item (e.g., "▶ ")
- **repeat_highlight_symbol** - Whether to repeat symbol on wrapped lines
- **style** - Base style for all list items

## Examples

### Basic Usage

```graphix
use tui;
use tui::list;

let items = [
    line("Apple"),
    line("Banana"),
    line("Cherry")
];

list(
    #selected: &0,
    &items
)
```

### Interactive List with Navigation

```graphix
let items = [
    line("Apple"), line("Banana"), line("Cherry"),
    line("Date"), line("Elderberry"), line("Fig"), line("Grape")
];

let last = array::len(items) - 1;
let selected = 0;
let scroll_pos = 0;
let visible = 5;

// Auto-scroll to keep selection visible
scroll_pos <- select selected {
    s if s < scroll_pos => s,
    s if s > (scroll_pos + visible - 1) => s - visible + 1,
    _ => never()
};

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Up if selected > 0 => {
                selected <- (k ~ selected) - 1;
                `Stop
            },
            k@`Down if selected < last => {
                selected <- (k ~ selected) + 1;
                `Stop
            },
            k@`Home => { selected <- k ~ 0; `Stop },
            k@`End => { selected <- k ~ last; `Stop },
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
        #title: &line("Fruit Selection"),
        &list(
            #highlight_style: &style(#fg: `Black, #bg: `Yellow),
            #highlight_symbol: &"▶ ",
            #selected: &selected,
            #scroll: &scroll_pos,
            &items
        )
    )
)
```

### Styled Items

```graphix
let make_item = |text, priority| select priority {
    `High => line(#style: style(#fg: `Red, #add_modifier: `Bold), text),
    `Medium => line(#style: style(#fg: `Yellow), text),
    `Low => line(#style: style(#fg: `White), text)
};

let items = [
    make_item("Critical bug", `High),
    make_item("Feature request", `Medium),
    make_item("Documentation", `Low)
];

list(#selected: &0, &items)
```

### Action on Selection

```graphix
let selected = 0;
let action_triggered = never();

let items = [line("Open"), line("Save"), line("Close"), line("Exit")];

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            // ... navigation keys ...
            k@`Enter => {
                action_triggered <- k ~ selected;
                `Stop
            },
            _ => `Continue
        },
        _ => `Continue
    },
    _ => `Continue
};

select action_triggered {
    0 => net::write("/app/action", "open")?,
    1 => net::write("/app/action", "save")?,
    2 => net::write("/app/action", "close")?,
    3 => net::write("/app/action", "exit")?,
    _ => never()
}
```

## Auto-scroll Logic

Keep the selected item visible:

```graphix
let selected = 0;
let scroll_pos = 0;
let visible_lines = 10;

scroll_pos <- select selected {
    s if s < scroll_pos => s,
    s if s >= (scroll_pos + visible_lines) => s - visible_lines + 1,
    _ => never()
};
```

## See Also

- [table](table.md) - For multi-column structured data
- [scrollbar](scroll.md) - For adding scrollbars
- [block](block.md) - For containing lists with borders
- [tabs](tabs.md) - For switching between different lists
