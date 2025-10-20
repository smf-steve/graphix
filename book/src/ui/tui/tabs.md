# tabs

The `tabs` widget creates a tabbed interface for organizing content into multiple switchable panels. Each tab has a title displayed in the tab bar and associated content that's shown when the tab is selected.

## Function Signature

```
/// Creates a tabbed interface from an array of (title, content) tuples
val tabs: fn(
    ?#selected: &i64,
    ?#highlight_style: &Style,
    ?#style: &Style,
    ?#divider: &[string, Span],
    Array<(Line, Widget)>
) -> Widget;
```

## Parameters

- **selected** - Index of the currently selected tab (0-indexed)
- **highlight_style** - Style for the selected tab title
- **style** - Base style for unselected tab titles
- **divider** - String or span separating tab titles

## Examples

### Basic Usage

```graphix
use tui;
use tui::tabs;
use tui::paragraph;

let tab1 = paragraph(&"This is tab 1");
let tab2 = paragraph(&"This is tab 2");
let tab3 = paragraph(&"This is tab 3");

tabs(
    #selected: &0,
    &[
        (line("One"), tab1),
        (line("Two"), tab2),
        (line("Three"), tab3)
    ]
)
```

### Navigation Between Tabs

```graphix
let selected_tab = 0;

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Left if selected_tab > 0 => {
                selected_tab <- (k ~ selected_tab) - 1;
                `Stop
            },
            k@`Right if selected_tab < 2 => {
                selected_tab <- (k ~ selected_tab) + 1;
                `Stop
            },
            k@`Tab => {
                selected_tab <- ((k ~ selected_tab) + 1) % 3;
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
        #title: &line("Application (←/→ to navigate)"),
        &tabs(
            #highlight_style: &style(#fg: `Yellow, #add_modifier: `Bold),
            #style: &style(#fg: `Gray),
            #selected: &selected_tab,
            &[(line("Overview"), overview), (line("Items"), items), (line("Settings"), settings)]
        )
    )
)
```

### Styled Tab Titles

```graphix
let tab_list = [
    (line([
        span(#style: style(#fg: `Green), "✓ "),
        span("Completed")
    ]), completed_content),
    (line([
        span(#style: style(#fg: `Yellow), "⚠ "),
        span("In Progress")
    ]), progress_content),
    (line([
        span(#style: style(#fg: `Red), "✗ "),
        span("Failed")
    ]), failed_content)
];

tabs(&tab_list)
```

### Tab with Badge

```graphix
let unread_count = 5;

let messages_tab = line([
    span("Messages"),
    span(#style: style(#fg: `Red, #add_modifier: `Bold), " ([unread_count])")
]);

tabs(&[
    (line("Home"), home_content),
    (messages_tab, messages_content),
    (line("Settings"), settings_content)
])
```

## Keyboard Navigation

Common patterns:
- `Left`/`Right` - Switch to previous/next tab
- `Tab` - Cycle forward through tabs
- Number keys - Jump directly to tab

## See Also

- [block](block.md) - For containing tabs with borders
- [list](list.md) - Common content for tabs
- [table](table.md) - For tabular content in tabs
