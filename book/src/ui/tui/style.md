# Styling

The `style` function creates visual styles for TUI widgets, controlling colors, text modifiers, and underline colors. Styles are used throughout the TUI system to customize the appearance of text, widgets, and interactive elements.

## API

```graphix
mod tui: sig {
    type Color = [
      `Reset,
      `Black,
      `Red,
      `Green,
      `Yellow,
      `Blue,
      `Magenta,
      `Cyan,
      `Gray,
      `DarkGray,
      `LightRed,
      `LightGreen,
      `LightYellow,
      `LightBlue,
      `LightMagenta,
      `LightCyan,
      `White,
      `Rgb({ r: i64, g: i64, b: i64 }), // r, g, b range: 0-255
      `Indexed(i64)                      // 0-255 for 256-color palette
    ];

    type Modifier = [
      `Bold,
      `Italic
    ];

    type Style = {
      fg: [Color, null],
      bg: [Color, null],
      underline_color: [Color, null],
      add_modifier: [Array<Modifier>, null],
      sub_modifier: [Array<Modifier>, null]
    };

    /// Creates a style with optional foreground/background colors and modifiers
    val style: fn(
        ?#fg: [Color, null],
        ?#bg: [Color, null],
        ?#underline_color: [Color, null],
        ?#add_modifier: [Array<Modifier>, null],
        ?#sub_modifier: [Array<Modifier>, null]
    ) -> Style;
}
```

## Colors

### Named Colors

Standard terminal colors are available as simple variants:

- **Basic**: `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `Gray`
- **Light variants**: `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`
- **Gray tones**: `DarkGray`, `Gray`
- **Reset**: `Reset` - resets to terminal default

### True Color (RGB)

For precise color control, use RGB values with components from 0-255:

```graphix
style(#fg: `Rgb({r: 255, g: 100, b: 50}))
style(#bg: `Rgb({r: 20, g: 20, b: 20}))
```

### Indexed Colors

Access the 256-color palette with indexed colors (0-255):

```graphix
style(#fg: `Indexed(202))  // Orange
style(#bg: `Indexed(234))  // Dark gray
```

The 256-color palette includes:
- 0-15: Standard terminal colors
- 16-231: 6x6x6 RGB color cube
- 232-255: Grayscale ramp

## Text Modifiers

Modifiers change text appearance. Pass them as an array to `add_modifier`:

```graphix
style(#add_modifier: [`Bold])
style(#add_modifier: [`Italic])
style(#add_modifier: [`Bold, `Italic])
```

Available modifiers:
- **`Bold`**: Bold/bright text
- **`Italic`**: Italic text (terminal support varies)

Use `sub_modifier` to explicitly remove modifiers when inheriting styles:

```graphix
style(#sub_modifier: [`Bold])  // Remove bold from inherited style
```

## Style Parameters

All parameters are optional and nullable:

- **`fg`**: Foreground (text) color
- **`bg`**: Background color
- **`underline_color`**: Color for underlined text
- **`add_modifier`**: Array of modifiers to add
- **`sub_modifier`**: Array of modifiers to remove

Omitted or `null` parameters use the terminal's default or inherit from parent styles.

## Style Inheritance

Styles are typically applied hierarchically:

1. **Widget style**: Base style for the entire widget
2. **Line style**: Style for a line of text
3. **Span style**: Style for individual text segments

More specific styles override less specific ones. For example, a span's foreground color overrides the line's foreground color.

## Terminal Compatibility

Not all terminals support all styling features:

- **Named colors**: Universally supported
- **256-color palette**: Widely supported in modern terminals
- **True color (RGB)**: Supported in most modern terminals (check `COLORTERM=truecolor`)
- **Bold**: Universally supported
- **Italic**: Support varies; some terminals display as inverse or underline

## See Also

- [text](text.md) - Using styles with text, spans, and lines
- [block](block.md) - Styling block borders and titles
- [list](list.md) - Highlight styles for selections
- [table](table.md) - Cell and row styling
- [tabs](tabs.md) - Tab styling
