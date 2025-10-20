# text

The `text` widget renders styled text in the terminal. It's a fundamental building block for displaying formatted content with colors, modifiers, and multiple lines. Text is built from `Line` objects, which are in turn composed of `Span` objects.

## FunctionFunction SignaturesSignatures

```
type Alignment = [`Left, `Center, `Right];
type Modifier = [`Bold, `Italic, `Underlined, `SlowBlink, `RapidBlink, `Reversed, `Hidden, `CrossedOut];
type Color = [
    `Red, `Green, `Yellow, `Blue, `Magenta, `Cyan, `Gray, `DarkGray,
    `LightRed, `LightGreen, `LightYellow, `LightBlue, `LightMagenta, `LightCyan,
    `White, `Black,
    `Indexed(i64),
    `Rgb({r: i64, g: i64, b: i64})
];

/// Creates styled text from a string or array of lines
val text: fn([string, Array<Line>]) -> Widget;

/// Creates a line of text from a string or array of spans
val line: fn(?#style: Style, ?#alignment: Alignment, [string, Array<Span>]) -> Line;

/// Creates a styled text span
val span: fn(?#style: Style, string) -> Span;

/// Creates a text style
val style: fn(?#fg: Color, ?#bg: Color, ?#add_modifier: Modifier) -> Style;
```

## Text Hierarchy
```
type Alignment = [`Left, `Center, `Right];
type Modifier = [`Bold, `Italic, `Underlined, `SlowBlink, `RapidBlink, `Reversed, `Hidden, `CrossedOut];
type Color = [
    `Red, `Green, `Yellow, `Blue, `Magenta, `Cyan, `Gray, `DarkGray,
    `LightRed, `LightGreen, `LightYellow, `LightBlue, `LightMagenta, `LightCyan,
    `White, `Black,
    `Indexed(i64),
    `Rgb({r: i64, g: i64, b: i64})
];

/// Creates styled text from a string or array of lines
val text: fn([string, Array<Line>]) -> Widget;

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

## BasicExamplesBasicUsage

```graphix
use tui;
use tui::text;

// Simple text
text(&"Hello, World!")

//// Multi-line
text(&[
    line("First line"),
    line("Second line")
    line("Second line")
])

// Styled
let error = spanlet error = span(#style: style(#fg: `Red, #add_modifier: `BoldRed, #add_modifier: `Bold),  "Error:Error:");
text(&[line([error, span(" Something went wrong")])])
```

### Status Messages

```graphix
let make_status = |level, msg| select level {
    `Error => line([
        span(#style: style(#fg: `Red, #add_modifier: `Bold), "ERROR: "),
        span(msg)
    ]),
    `Warning => line([
        span(#style: style(#fg: `Yellow, #add_modifier: `Bold), "WARNING: "),
        span(msg)
    ]),
    `Info => line([
        span(#style: style(#fg: `Cyan), "INFO: "),
        span(msg)
    ])
};

text(&[
    make_status(`Info, "Application started"),
    make_status(`Warning, "Cache miss"),
    make_status(`Error, "Connection failed")
])
```

### DynamicDynamic ColorsColors

```graphix
let countcount = 00;
let timer = time::timer(duration:1.s, true);
countcount <- timer ~ (count(count + 1);

let colorscolors = [`Red, `Green, `Yellow, `Blue, `Magenta, `Cyan];
let color =[`Red, `Green, `Yellow, `Blue, `Magenta, `Cyan];
let color = colors[count % array::len(colors)count % array::len(colors)]$;

line([
    span(#style: style(#fg: `White)style(#fg: `White), "Count: Count: "),
    span(#style:#style: style(#fg: color, #add_modifier: `Bold), "[count]style(#fg: color, #add_modifier: `Bold), "[count]")
])
```

### Alignment

```graphix
text(&[
    line(#alignment: `Left, "Left aligned"),
    line(#alignment: `Center, "Centered"),
    line(#alignment: `Right, "Right aligned")
])
```

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
