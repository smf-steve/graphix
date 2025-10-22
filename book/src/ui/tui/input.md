# The Input Widget

The `input_handler` widget intercepts terminal input events (keyboard, mouse, resize, etc.) and allows you to handle them before they reach child widgets. This is essential for creating interactive TUI applications with keyboard navigation, mouse interactions, and custom event handling.

## API

```graphix
type Event = [
  `FocusGained,
  `FocusLost,
  `Key(KeyEvent),
  `Mouse(MouseEvent),
  `Paste(string),
  `Resize(i64, i64)
];

type KeyEvent = {
  code: KeyCode,
  kind: KeyEventKind,
  modifiers: Array<KeyModifier>,
  state: Array<KeyEventState>
};

type MouseEvent = {
  column: i64,
  kind: MouseEventKind,
  modifiers: Array<KeyModifier>,
  row: i64
};

/// Creates an input handler that intercepts events before they reach the child
val input_handler: fn(
    ?#enabled: &[bool, null],
    #handle: &fn(Event) -> [`Stop, `Continue],
    &Tui
) -> Tui;
```

## How It Works

The `input_handler` widget sits between the terminal and your UI, intercepting all input events. Your handler function receives each event and returns either:

- **`Stop**: Event was handled, don't pass it to child widgets
- **`Continue**: Event wasn't handled, pass it down to children

This creates a hierarchical event system where parents can consume events before children see them, similar to event bubbling in web browsers.

## Event Types

### Keyboard Events

Keyboard events include the key code, event kind (press/release/repeat), active modifiers, and keyboard state:

```graphix
type KeyCode = [
  `Backspace, `Enter, `Left, `Right, `Up, `Down,
  `Home, `End, `PageUp, `PageDown, `Tab, `BackTab,
  `Delete, `Insert, `Esc,
  `F(i64),              // Function keys: F(1), F(2), etc.
  `Char(string),        // Character input
  // ... and more
];

type KeyEventKind = [`Press, `Repeat, `Release];

type KeyModifier = [`Shift, `Control, `Alt, `Super, `Hyper, `Meta];
```

### Mouse Events

Mouse events include position (row/column), event kind, and modifiers:

```graphix
type MouseEventKind = [
  `Down(MouseButton),
  `Up(MouseButton),
  `Drag(MouseButton),
  `Moved,
  `ScrollDown, `ScrollUp,
  `ScrollLeft, `ScrollRight
];

type MouseButton = [`Left, `Right, `Middle];
```

### Other Events

- **`FocusGained`/`FocusLost`**: Terminal window focus changes
- **`Paste(string)`**: Text pasted into terminal
- **`Resize(i64, i64)`**: Terminal window resized (width, height)

## Examples

### Basic Keyboard Navigation

```graphix
{{#include ../../examples/tui/list_interactive.gx}}
```

This example shows keyboard navigation in a list. Arrow keys move the selection, and the viewport automatically scrolls to keep the selection visible.

![Interactive List](./media/list_interactive.gif)

### Focus Management

```graphix
{{#include ../../examples/tui/layout_focus.gx}}
```

Tab cycles focus between three panes, with visual feedback via border styling.

![Layout Focus](./media/layout_focus.gif)

### Scrollable Content

```graphix
{{#include ../../examples/tui/scroll_paragraph.gx}}
```

Arrow keys and page up/down navigate through long text content.

![Scrollable Paragraph](./media/scroll_paragraph.gif)

## Event Handler Pattern

A typical event handler uses nested `select` expressions to pattern match on event types:

```graphix
let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Up if condition => {
                // Handle up arrow
                `Stop
            },
            k@`Down if condition => {
                // Handle down arrow
                `Stop
            },
            _ => `Continue  // Pass unhandled keys to children
        },
        _ => `Continue
    },
    `Mouse(m) => select m.kind {
        // Handle mouse events
        _ => `Continue
    },
    _ => `Continue  // Pass other events through
};
```

The `k@` syntax binds the key event to `k`, allowing you to access it within the handler while the event flows through the reactive graph.

## Disabling Input

Use the optional `enabled` parameter to temporarily disable event handling:

```graphix
let input_enabled = true;

input_handler(
    #enabled: &input_enabled,
    #handle: &handle_event,
    &child_widget
)
```

When `enabled` is `false` or `null`, events pass through without calling the handler.

## Nesting Input Handlers

Input handlers can be nested to create hierarchical event handling:

```graphix
input_handler(
    #handle: &global_handler,  // Handles global shortcuts
    &layout([
        input_handler(
            #handle: &left_pane_handler,  // Handles left pane input
            &left_widget
        ),
        input_handler(
            #handle: &right_pane_handler,  // Handles right pane input
            &right_widget
        )
    ])
)
```

Parents process events first. If they return `Continue`, the event flows down to children.

## Common Patterns

### Arrow Key Navigation

```graphix
select k.code {
    k@`Up if index > 0 => {
        index <- (k ~ index) - 1;
        `Stop
    },
    k@`Down if index < max => {
        index <- (k ~ index) + 1;
        `Stop
    },
    _ => `Continue
}
```

### Modal Keybindings

```graphix
let mode = `Normal;

select k.code {
    k@`Char("i") if mode == `Normal => {
        mode <- k ~ `Insert;
        `Stop
    },
    k@`Esc if mode == `Insert => {
        mode <- k ~ `Normal;
        `Stop
    },
    _ => `Continue
}
```

### Modifier Keys

```graphix
select e {
    `Key(k) => {
        let has_ctrl = array::member(k.modifiers, `Control);
        select k.code {
            k@`Char("c") if has_ctrl => {
                // Handle Ctrl+C
                `Stop
            },
            _ => `Continue
        }
    },
    _ => `Continue
}
```

## See Also

- [layout](layout.md) - Focus management with layouts
- [list](list.md) - Interactive list widget
- [table](table.md) - Interactive table widget
- [scroll](scroll.md) - Scrollbar widget for scrollable content
