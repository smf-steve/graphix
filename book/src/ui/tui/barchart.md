# The Bar Chart Widget

The `barchart` widget displays categorical data as vertical bars, supporting grouped bars, custom styling, and dynamic updates. It's ideal for comparing values across categories, showing rankings, or displaying resource usage.

## APIs

```
mod barchart: sig {
    /// Creates a bar chart from bar groups
    val bar_chart: fn(
        ?#max: &i64,
        ?#bar_width: &i64,
        ?#bar_gap: &i64,
        ?#group_gap: &i64,
        ?#style: &Style,
        &Array<BarGroup>
    ) -> Widget;

    /// Creates a group of bars
    val bar_group: fn(?#label: Line, Array<Bar>) -> BarGroup;

    /// Creates an individual bar
    val bar: fn(
        ?#style: &Style,
        ?#label: &Line,
        ?#text_value: &Line,
        &i64
    ) -> Bar;
}
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
{{#include ../../examples/tui/barchart_basic.gx}}
```

![Basic Bar Chart](./media/barchart_basic.png)

### Grouped Bars with Dynamic Data

```graphix
{{#include ../../examples/tui/barchart_grouped.gx}}
```

![Grouped Bar Chart](./media/barchart_grouped.png)

### Color-coded Values

```graphix
{{#include ../../examples/tui/barchart_colored.gx}}
```

![Colored Bar Chart](./media/barchart_colored.png)

## See Also

- [chart](chart.md) - For continuous data visualization
- [sparkline](sparkline.md) - For compact trend display
- [gauge](gauge.md) - For single value display
