# Building UIs With Graphix

Graphix excels at building user interfaces thanks to its reactive dataflow nature. Changes in data automatically propagate through the UI graph, updating only the components that need to change. This makes building complex, interactive applications surprisingly straightforward.

## Why Graphix for UIs?

Traditional UI frameworks require you to manually manage state changes, update DOM elements, and coordinate between different parts of your application. Graphix eliminates this complexity by treating your entire UI as a reactive graph where:

- **Data flows automatically**: When underlying data changes, dependent UI components update automatically
- **State is declarative**: You describe what the UI should look like, not how to update it
- **Composition is natural**: Complex UIs are built by composing simple, reusable components
- **Performance is built-in**: Only components that depend on changed data will re-render

## Future UI Targets

While Graphix currently implements supprot for building TUIs, the reactive architecture makes it well-suited for other UI paradigms:

- **Desktop Applications**: Native desktop applications with reactive updates. Support for this is planned.
- **Web UIs**: The dataflow model maps naturally to modern web frameworks
- **Mobile UIs**: Touch-based interfaces with gesture handling

The core concepts of reactive data flow, component composition, and declarative styling will apply across all UI targets.

## Getting Started

The Graphix shell will automatically build a UI if the last value in your module has type `tui::Widget` (or in the future `gui::Widget`). You can try out the examples in this book by pasting them in a file, or even typing (the short ones) into the interactive REPL. Each TUI component has detailed documentation in the following sections, including complete API references and practical examples.

You can also study and run the examples in `graphix-shell/examples/`. Start with simple components like `text.gx` and `block.gx`, then work your way up to more complex examples like `browser.gx` and `table-advanced.gx`.
