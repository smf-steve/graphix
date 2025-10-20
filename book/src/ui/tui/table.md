# table

The `table` widget displays structured data in rows and columns with support for selection, scrolling, and custom styling. It's ideal for data grids, process monitors, file listings, and any tabular data display.

## Function Signatures

```
type HighlightSpacing = [`Always, `WhenSelected, `Never];

/// Creates a table widget from an array of row references
val table: fn(
    ?#header: &Row,
    ?#selected: &i64,
    ?#row_highlight_style: &Style,
    ?#highlight_symbol: &string,
    ?#highlight_spacing: &HighlightSpacing,
    ?#widths: &Array<Constraint>,
    ?#column_spacing: &i64,
    ?#style: &Style,
    Array<&Row>
) -> Widget;

/// Creates a table row from cells
val row: fn(?#style: Style, Array<Cell>) -> Row;

/// Creates a table cell from a line
val cell: fn(?#style: Style, Line) -> Cell;
```

## Parameters

- **header** - Row object for the table header
- **selected** - Index of the currently selected row
- **row_highlight_style** - Style for the selected row
- **highlight_symbol** - String before selected row
- **highlight_spacing** - When to show highlight symbol: `Always`, `WhenSelected`, `Never`
- **widths** - Array of column width constraints
- **column_spacing** - Number of spaces between columns
- **style** - Base style for the table

## Examples

### Basic Usage

```graphix
use tui;
use tui::table;

let header = row([
    cell(line("Name")),
    cell(line("Age")),
    cell(line("City"))
]);

let row1 = row([
    cell(line("Alice")),
    cell(line("28")),
    cell(line("New York"))
]);

let row2 = row([
    cell(line("Bob")),
    cell(line("32")),
    cell(line("San Francisco"))
]);

table(
    #header: &header,
    #selected: &0,
    &[&row1, &row2]
)
```

### Interactive Table

```graphix
type User = {name: string, age: i64, city: string};

let users: Array<User> = [
    {name: "Alice", age: 28, city: "New York"},
    {name: "Bob", age: 32, city: "San Francisco"},
    {name: "Charlie", age: 25, city: "Chicago"}
];

let header = row(
    #style: style(#fg: `Yellow, #add_modifier: `Bold),
    [cell(line("Name")), cell(line("Age")), cell(line("City"))]
);

let rows: Array<&Row> = array::map(users, |u: User| -> &Row {
    &row([
        cell(line(u.name)),
        cell(line("[u.age]")),
        cell(line(u.city))
    ])
});

let selected = 0;

let handle_event = |e: Event| -> [`Stop, `Continue] select e {
    `Key(k) => select k.kind {
        `Press => select k.code {
            k@`Up if selected > 0 => {
                selected <- (k ~ selected) - 1;
                `Stop
            },
            k@`Down if selected < 2 => {
                selected <- (k ~ selected) + 1;
                `Stop
            },
            _ => `Continue
        },
        _ => `Continue
    },
    _ => `Continue
};

input_handler(
    #handle: &handle_event,
    &block(
        #border: &`All,
        #title: &line("User Directory"),
        &table(
            #header: &header,
            #row_highlight_style: &style(#bg: `Yellow, #fg: `Black),
            #selected: &selected,
            #column_spacing: &2,
            #widths: &[`Percentage(30), `Percentage(20), `Percentage(50)],
            &rows
        )
    )
)
```

### Conditional Cell Styling

```graphix
let make_cpu_cell = |cpu: i64| -> Cell {
    let style = select cpu {
        c if c > 80 => style(#fg: `Red),
        c if c > 50 => style(#fg: `Yellow),
        _ => style(#fg: `Green)
    };
    cell(#style, line("[cpu]%"))
};

let row = row([
    cell(line("process-1")),
    make_cpu_cell(85)  // Red
]);
```

### Real-time Updates

```graphix
type Process = {pid: i64, name: string, cpu: i64};

let processes: Array<Process> = [...];
let clock = time::timer(duration:1.s, true);

let rows: Array<&Row> = array::map(processes, |p: Process| -> &Row {
    let cpu_val = p.cpu;
    cpu_val <- {
        let v = clock ~ cpu_val;
        v + rand::rand(#clock, #start: -5, #end: 5)
    };
    
    &row([
        cell(line("[p.pid]")),
        cell(line(p.name)),
        cell(line("[cpu_val]%"))
    ])
});

table(#header: &header, &rows)
```

## See Also

- [list](list.md) - For simpler single-column selection
- [scrollbar](scroll.md) - For adding scrollbars
- [block](block.md) - For containing tables with borders
