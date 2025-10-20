# barchart

The `barchart` widget displays categorical data as vertical bars, supporting grouped bars, custom styling, and dynamic updates. It's ideal for comparing values across categories, showing rankings, or displaying resource usage.

## Function Signatures

```
/// Creates a bar chart from bar groups
val bar_chart: fn(
    ?#max: &i64,
    ?#bar_width: &i64,
    ?#bar_gap: &i64,
    ?#group_gap: &i64,
    ?#style: &Style,
    Array<BarGroup>
) -> Widget;

/// Creates a group of bars
val bar_group: fn(?#label: Line, Array<Bar>) -> BarGroup;

/// Creates an individual bar
val bar: fn(
    ?#style: &Style,
    ?#label: &Line,
    ?#text_value: &Line,
    i64
) -> Bar;
```

## Parameters

### bar_chart
- **max** - Maximum value for chart scale (auto-scales if not specified)
- **bar_width** - Width of each bar in characters
- **bar_gap** - Space between bars within a group
- **group_gap** - Space between bar groups
- **style** - Base style for the chart

### bar_group
- **label** - Line labeling the group (displayed below bars)

### bar
- **style** - Style for the bar
- **label** - Line labeling the bar
- **text_value** - Line displayed above bar (defaults to numeric value)

## Examples

### Basic Usage

```graphix
use tui;
use tui::barchart;

let bar1 = bar(
    #style: &style(#fg: `Cyan),
    #label: &line("Sales"),
    &42
);

bar_chart(&[bar_group(#label: line("Q1"), [bar1])])
```

### Grouped Bars with Dynamic Data

```graphix
let clock = time::timer(duration:0.7s, true);

let group0 = [
    bar(#style: &style(#fg: `Red), #label: &line("CPU"), &rand(#start:0, #end:100, #clock)),
    bar(#style: &style(#fg: `Yellow), #label: &line("Memory"), &rand(#start:25, #end:200, #clock))
];

let group1 = [
    bar(#style: &style(#fg: `Cyan), #label: &line("Network"), &rand(#start:0, #end:50, #clock)),
    bar(#style: &style(#fg: `Magenta), #label: &line("Disk"), &rand(#start:1, #end:25, #clock))
];

let chart = bar_chart(
    #bar_gap: &2,
    #bar_width: &8,
    #max: &200,
    &[
        bar_group(#label: line("Server 1"), group0),
        bar_group(#label: line("Server 2"), group1)
    ]
);

block(#border: &`All, #title: &line("Resource Usage"), &chart)
```

### Color-coded Values

```graphix
let make_colored_bar = |label, value| {
    let color = select value {
        v if v > 80 => `Red,
        v if v > 50 => `Yellow,
        _ => `Green
    };
    bar(#style: &style(#fg: color), #label: &line(label), &value)
};

let bars = [
    make_colored_bar("Service A", 35),
    make_colored_bar("Service B", 65),
    make_colored_bar("Service C", 92)
];

bar_chart(&[bar_group(bars)])
```

### Centered Chart

```graphix
layout(
    #direction: &`Horizontal,
    #flex: &`Center,
    &[child(#constraint: `Max(40), bar_chart(&groups))]
)
```

## See Also

- [chart](chart.md) - For continuous data visualization
- [sparkline](sparkline.md) - For compact trend display
- [gauge](gauge.md) - For single value display
