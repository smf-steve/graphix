# layout

The `layout` widget arranges child widgets in horizontal or vertical layouts with flexible sizing constraints. It's the primary tool for organizing complex TUI interfaces and supports focus management for interactive applications.

## Function Signatures

```
type Direction = [`Horizontal, `Vertical];
type Flex = [`Start, `Center, `End, `SpaceAround, `SpaceBetween];
type Constraint = [
    `Percentage(i64),
    `Length(i64),
    `Min(i64),
    `Max(i64),
    `Ratio(i64, i64),
    `Fill(i64)
];

/// Creates a layout that arranges child widgets
val layout: fn(
    ?#direction: &Direction,
    ?#focused: &i64,
    ?#flex: &Flex,
    Array<Child>
) -> Widget;

/// Creates a child widget with sizing constraints
val child: fn(?#constraint: Constraint, Widget) -> Child;
```

## Parameters

- **direction** - `Horizontal` or `Vertical` (default: `Vertical`)
- **focused** - Index of the currently focused child (0-indexed)
- **flex** - Alignment when children don't fill space: `Start`, `Center`, `End`, `SpaceAround`, `SpaceBetween`

## Constraint Types

- **Percentage(n)** - Allocates n% of available space
- **Length(n)** - Fixed width/height in cells
- **Min(n)** - At least n cells
- **Max(n)** - At most n cells
- **Ratio(num, den)** - Fractional allocation (num/den)
- **Fill(n)** - Takes remaining space after other constraints

## Examples

### Basic Layout

```graphix
use tui;
use tui::layout;
use tui::block;

let sidebar = block(#border: &`All, #title: &line("Sidebar"), &content1);
let main = block(#border: &`All, #title: &line("Main"), &content2);

layout(
    #direction: &`Horizontal,
    &[
        child(#constraint: `Percentage(30), sidebar),
        child(#constraint: `Percentage(70), main)
    ]
)
```

### Three-Pane Layout with Focus

```graphix
let focused = 0;

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Tab => {
                focused <- ((k ~ focused) + 1) % 3;
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
    &layout(
        #direction: &`Horizontal,
        #focused: &focused,
        &[
            child(#constraint: `Percentage(25), left_pane),
            child(#constraint: `Percentage(50), center_pane),
            child(#constraint: `Percentage(25), right_pane)
        ]
    )
)
```

### Nested Layouts

```graphix
let top_row = layout(
    #direction: &`Horizontal,
    &[
        child(#constraint: `Percentage(50), widget1),
        child(#constraint: `Percentage(50), widget2)
    ]
);

layout(
    #direction: &`Vertical,
    &[
        child(#constraint: `Percentage(50), top_row),
        child(#constraint: `Percentage(50), bottom_widget)
    ]
)
```

### Header/Content/Footer

```graphix
layout(
    #direction: &`Vertical,
    &[
        child(#constraint: `Length(3), header),
        child(#constraint: `Fill(1), content),
        child(#constraint: `Length(1), footer)
    ]
)
```

## See Also

- [block](block.md) - Common child widget for layouts
- [input_handler](../overview.md#input-handling) - For handling focus changes
