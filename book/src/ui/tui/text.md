# text

The `text` widget renders styled text in the terminal. It's a fundamental building block for displaying formatted content with colors, modifiers, and multiple lines. Text is built from `Line` objects, which are in turn composed of `Span` objects.

## Function Signatures

```
type Alignment = [`Left, `Center, `Right];
type Modifier = [
    `Bold, 
    `Italic, 
    `Underlined, 
    `SlowBlink, 
    `RapidBlink, 
    `Reversed, 
    `Hidden, 
    `CrossedOut
];
type Color = [
    `Red, `Green, `Yellow, `Blue, `Magenta, `Cyan, `Gray, `DarkGray,
    `LightRed, `LightGreen, `LightYellow, `LightBlue, `LightMagenta, `LightCyan,
    `White, `Black,
    `Indexed(i64),
    `Rgb({r: i64, g: i64, b: i64})
];

/// Creates styled text from a string or array of lines
val text: fn(&[string, Array<Line>]) -> Widget;

/// Creates a line of text from a string or array of spans
val line: fn(?#style: Style, ?#alignment: Alignment, [string, Array<Span>]) -> Line;

/// Creates a styled text span
val span: fn(?#style: Style, string) -> Span;

/// Creates a text style
val style: fn(?#fg: Color, ?#bg: Color, ?#add_modifier: Modifier) -> Style;
```

## Text Hierarchy

- **Span**: A single segment of text with a single style
- **Line**: A collection of spans forming one line
- **Text**: A collection of lines forming multi-line content

## Examples

### Basic Usage

```graphix
{{#include ../../examples/tui/text_basic.gx}}
```

![Basic Text](./media/text_basic.png)

### Status Messages

```graphix
{{#include ../../examples/tui/text_status.gx}}
```

![Multi Line Status](./media/text_status.png)

### Dynamic Colors

```graphix
{{#include ../../examples/tui/text_dynamic.gx}}
```

![Dynamic Text](./media/text_dynamic.gif)

### Alignment

```graphix
{{#include ../../examples/tui/text_alignment.gx}}
```

![Text Alignment](./media/text_alignment.gif)

## Color Support

- **Named colors**: `Red`, `Green`, `Blue`, `Yellow`, `Magenta`, `Cyan`, `White`, `Black`, `Gray`, `DarkGray`, and `Light*` variants
- **Indexed colors**: `Indexed(202)` for 256-color palette
- **RGB colors**: `Rgb({r: 255, g: 100, b: 50})` for true color

## Text Modifiers

- `Bold`, `Italic`, `Underlined`, `CrossedOut`
- `SlowBlink`, `RapidBlink` (terminal support varies)
- `Reversed`, `Hidden`

## See Also

- [paragraph](paragraph.md) - For wrapped and scrollable text
- [block](block.md) - For containing text with borders
- [list](list.md) - For selectable text items
