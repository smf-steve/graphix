# The Block Widget

The `block` widget is a container that wraps other widgets with optional borders, titles, and styling. It's one of the most commonly used widgets for creating structured layouts and visually separating different sections of your TUI.

## API

```
mod block: sig {
    type Borders = [`All, `None, `Top, `Bottom, `Left, `Right];

    /// Creates a block widget that wraps content with borders and styling
    val block: fn(
        ?#border: &Borders,
        ?#border_style: &Style,
        ?#title: &Line,
        ?#title_bottom: &Line,
        ?#style: &Style,
        ?#size: &Size,
        &Widget
    ) -> Widget;
}
```

## Parameters

- **border** - Border style: `All`, `None`, `Top`, `Bottom`, `Left`, or `Right`
- **border_style** - Style for the border
- **title** - Line displayed at the top of the block
- **title_bottom** - Line displayed at the bottom of the block
- **style** - Style for the block's interior
- **size** (output) - Rendered size of the block

## Examples

### Basic Usage

```graphix
{{#include ../../examples/tui/block_basic.gx}}
```

![Basic Block](./media/block_basic.png)

### Focus Indication

Use dynamic styling to show which block has focus:

```graphix
{{#include ../../examples/tui/block_focus.gx}}
```

![Styled Block](./media/block_focus.png)

### Dynamic Titles

Titles can contain reactive values that update automatically:

```graphix
{{#include ../../examples/tui/block_dynamic_title.gx}}
```

![Block With Dynamic Title](./media/block_dynamic_title.png)

## See Also

- [layout](layout.md) - For arranging multiple blocks
- [paragraph](paragraph.md) - Common content for blocks
- [text](text.md) - For creating styled text content
