# block

The `block` widget is a container that wraps other widgets with optional borders, titles, and styling. It's one of the most commonly used widgets for creating structured layouts and visually separating different sections of your TUI.

## Function Signature

```
type Borders = [`All, `None, `Top, `Bottom, `Left, `Right];

/// Creates a block widget that wraps content with borders and styling
val block: fn(
    ?#border: &Borders,
    ?#border_style: &Style,
    ?#title: &Line,
    ?#title_bottom: &Line,
    ?#style: &Style,
    ?#size: &Size,
    Widget
) -> Widget;
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
use tui;
use tui::block;
use tui::paragraph;

let content = paragraph(&"Hello, World!");

block(
    #border: &`All,
    #title: &line("My Block"),
    &content
)
```

### Focus Indication

Use dynamic styling to show which block has focus:

```graphix
let focused_block = 0;

block(
    #border: &`All,
    #border_style: &style(
        #fg: select focused_block {
            0 => `Red,
            _ => `Yellow
        }
    ),
    #title: &line("Block 1"),
    &content
)
```

### Dynamic Titles

Titles can contain reactive values that update automatically:

```graphix
let count = 0;
let timer = time::timer(duration:1.s, true);
count <- timer ~ (count + 1);

block(
    #border: &`All,
    #title: &line("Counter: [count]"),
    &content
)
```

## See Also

- [layout](layout.md) - For arranging multiple blocks
- [paragraph](paragraph.md) - Common content for blocks
- [text](text.md) - For creating styled text content
