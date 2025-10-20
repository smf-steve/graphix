# Terminal User Interfaces (TUIs)

Graphix includes a comprehensive TUI library built on top of the popular Rust `ratatui` crate. This allows you to build rich, interactive terminal applications with:

### Core Components

The TUI library provides all the essential building blocks:

- **Layout**: Flexible container system with horizontal/vertical arrangement and various sizing constraints
- **Block**: Wrapper component that adds borders, titles, and styling to other components
- **Text**: Rich text rendering with styling, colors, and formatting
- **Paragraph**: Multi-line text with scrolling and word wrapping

### Interactive Widgets

- **Table**: Sortable, selectable data tables with custom styling
- **List**: Scrollable lists with selection and highlighting
- **Tabs**: Tabbed interface for organizing content
- **Browser**: Netidx browser component
- **Calendar**: Date picker and event display

### Data Visualization

- **Chart**: Line charts with multiple datasets, custom axes, and styling
- **Bar Chart**: Grouped and individual bar charts with labels
- **Sparkline**: Compact inline charts perfect for dashboards
- **Gauge**: Progress indicators and meters
- **Canvas**: Low-level drawing surface for custom graphics

### Input Handling

Interactive components use Graphix's event system for keyboard and mouse input:

```graphix/book/src/ui/overview.md#L45-55
let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            `Up => { position <- position - 1; `Stop },
            `Down => { position <- position + 1; `Stop },
            _ => `Continue
        },
        _ => `Continue
    },
    _ => `Continue
};
```

Events flow through the component tree, allowing parent components to handle unprocessed events from children.

## Building Your First TUI

Here's a simple example that demonstrates the core concepts:

```graphix/book/src/ui/overview.md#L65-85
use tui;
use tui::block;
use tui::text;
use tui::layout;

let counter = 0;
let clock = time::timer(duration:1.s, true);
counter <- clock ~ (counter + 1);

let content = text(&"Counter: [counter]");

block(
  #border: &`All,
  #title: &line("My First TUI"),
  #style: &style(#fg: `Green),
  &content
)
```

This creates a bordered block with a counter that increments every second. The key insight is that when `counter` changes, the text automatically updates because of Graphix's reactive nature.

## Styling and Theming

Graphix TUIs support rich styling with:

- **Colors**: Named colors (`Red`, `Green`, `Blue`), indexed colors (`Indexed(202)`), and RGB (`Rgb({r: 255, g: 100, b: 50})`)
- **Text Effects**: Bold, italic, underline, strikethrough
- **Background Colors**: Set background colors for any component
- **Conditional Styling**: Use `select` expressions to change styles based on state

```graphix/book/src/ui/overview.md#L91-98
let style = style(
  #fg: select is_selected { true => `Yellow, false => `White },
  #bg: `DarkGray,
  #add_modifier: `Bold
);
```

## Layout System

The layout system provides flexible component arrangement:

- **Direction**: `Horizontal` or `Vertical`
- **Constraints**: `Percentage(50)`, `Length(20)`, `Min(10)`, `Max(100)`
- **Alignment**: `Left`, `Center`, `Right` for horizontal; `Top`, `Center`, `Bottom` for vertical
- **Focus Management**: Built-in focus handling for interactive components

```graphix/book/src/ui/overview.md#L106-115
layout(
  #direction: &`Horizontal,
  #focused: &selected_pane,
  &[
    child(#constraint: `Percentage(30), sidebar),
    child(#constraint: `Percentage(70), main_content)
  ]
)
```

## State Management

In Graphix, UI state is just regular program state. Use variables to track:

- Selection states in lists and tables
- Input field contents
- Window/pane focus
- Application modes (normal, edit, command)

State changes automatically trigger UI updates:

```graphix/book/src/ui/overview.md#L122-130
let selected_item = 0;
let items = ["Item 1", "Item 2", "Item 3"];

// When user presses down arrow assume the event is handled as
// shown above and arrow_pressed is set using connect
selected_item <- arrow_pressed ~ ((selected_item + 1) % array::len(items));

// UI automatically reflects the change
list(#selected: &selected_item, &items)
```

## Real-time Data Integration

Graphix TUIs excel at displaying real-time data. Connect to data sources via netidx and the UI updates automatically:

```graphix/book/src/ui/overview.md#L135-145
// Subscribe to live data
let temperature = cast<f64>(net::subscribe("/sensors/temperature")?)?;

// Display automatically updates when data changes
gauge(
  #title: &line("Temperature"),
  #ratio: &(temperature / 100.0),
  #style: &style(#fg: `Red)
)
```
