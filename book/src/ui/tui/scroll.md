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
{{#include ../../examples/tui/scroll_basic.gx}}
```

### Scrollable Paragraph

```graphix
{{#include ../../examples/tui/scroll_paragraph.gx}}
```

### Scrollable List

```graphix
{{#include ../../examples/tui/scroll_list.gx}}
```

### Auto-scroll to Bottom

```graphix
{{#include ../../examples/tui/scroll_autoscroll.gx}}
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
