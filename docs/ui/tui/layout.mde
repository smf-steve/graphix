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
    &Array<Child>
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
{{#include ../../examples/tui/layout_basic.gx}}
```

![Basic Layout](./media/layout_basic.png)

### Three-Pane Layout with Focus

```graphix
{{#include ../../examples/tui/layout_focus.gx}}
```

![Layout With Focus](./media/layout_focus.gif)

### Nested Layouts

```graphix
{{#include ../../examples/tui/layout_nested.gx}}
```

![Nested Layout](./media/layout_nested.png)

### Header/Content/Footer

```graphix
{{#include ../../examples/tui/layout_header_footer.gx}}
```

![Layout With Header and Footer](./media/layout_header_footer.png)

## See Also

- [block](block.md) - Common child widget for layouts
- [input_handler](../overview.md#input-handling) - For handling focus changes
