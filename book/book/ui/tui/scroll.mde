# scrollbar

The `scrollbar` widget adds a visual scrollbar indicator to scrollable content, making it clear when content extends beyond the visible area and showing the current scroll position.

## Function Signature

```
/// Wraps a widget with a scrollbar indicator
val scrollbar: fn(
    #position: &i64,
    ?#content_length: &i64,
    ?#size: &Size,
    &Widget
) -> Widget;
```

## Parameters

- **position** (required) - Current scroll position (typically the Y offset)
- **content_length** - Total length of the content (auto-detected if not specified)
- **size** (output) - Rendered size of the scrollbar area

## Examples

### Basic Usage

```graphix
{{#include ../../examples/tui/scroll_basic.gx}}
```

![Basic Scrollbar](./media/scroll_basic.png)

### Scrollable Paragraph

```graphix
{{#include ../../examples/tui/scroll_paragraph.gx}}
```

![Scrollable Paragraph](./media/scroll_paragraph.gif)

### Scrollable List

```graphix
{{#include ../../examples/tui/scroll_list.gx}}
```

![Scrollable List](./media/scroll_list.png)


## See Also

- [paragraph](paragraph.md) - For scrollable text content
- [list](list.md) - For scrollable lists
- [table](table.md) - For scrollable tables
- [block](block.md) - For containing scrollable content
