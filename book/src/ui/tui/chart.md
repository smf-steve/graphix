# chart

The `chart` widget renders line charts with multiple datasets, custom axes, labels, and styling. It's ideal for visualizing time series data, trends, sensor readings, and any numeric data relationships.

## Function Signatures

```
type GraphType = [`Line, `Scatter];
type Marker = [`Dot, `Braille, `Block];

/// Creates a chart widget with datasets and axes
val chart: fn(
    ?#style: &Style,
    #x_axis: &Axis,
    #y_axis: &Axis,
    Array<Dataset>
) -> Widget;

/// Creates an axis configuration
val axis: fn(
    ?#title: Line,
    ?#labels: Array<Line>,
    ?#style: Style,
    {min: f64, max: f64}
) -> Axis;

/// Creates a dataset to display on the chart
val dataset: fn(
    ?#style: &Style,
    ?#graph_type: &GraphType,
    ?#marker: &Marker,
    ?#name: &Line,
    Array<(f64, f64)>
) -> Dataset;
```

## Parameters

### chart
- **style** - Background style for the chart area
- **x_axis** - X-axis configuration (required)
- **y_axis** - Y-axis configuration (required)

### axis
- **title** - Line for axis title
- **labels** - Array of lines displayed along axis
- **style** - Style for axis lines and ticks

### dataset
- **style** - Style for the dataset (line and markers)
- **graph_type** - `Line` or `Scatter`
- **marker** - `Dot`, `Braille`, or `Block`
- **name** - Line naming the dataset (for legends)

## Examples

### Basic Usage

```graphix
{{#include ../../examples/tui/chart_basic.gx}}
```

### Real-time Data Visualization

```graphix
{{#include ../../examples/tui/chart_realtime.gx}}
```

### Multiple Datasets

```graphix
{{#include ../../examples/tui/chart_multi.gx}}
```

### Scatter Plot

```graphix
{{#include ../../examples/tui/chart_scatter.gx}}
```

## Marker Comparison

- **Dot**: Fastest, lowest resolution, good for dense data
- **Braille**: Smoothest curves, medium performance, best visual quality
- **Block**: High contrast, medium performance

## See Also

- [barchart](barchart.md) - For categorical data visualization
- [sparkline](sparkline.md) - For compact inline charts
- [canvas](canvas.md) - For custom graphics
