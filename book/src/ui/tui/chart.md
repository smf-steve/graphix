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
use tui;
use tui::chart;

let data: Array<(f64, f64)> = [(0.0, 0.0), (1.0, 1.0), (2.0, 4.0), (3.0, 9.0)];

let ds = dataset(
    #style: &style(#fg: `Cyan),
    #graph_type: &`Line,
    #marker: &`Dot,
    &data
);

chart(
    #x_axis: &axis({min: 0.0, max: 3.0}),
    #y_axis: &axis({min: 0.0, max: 9.0}),
    &[ds]
)
```

### Real-time Data Visualization

```graphix
let data: Array<(f64, f64)> = {
    let clock = time::timer(duration:0.5s, true);
    let x = 0.0;
    x <- (clock ~ x) + 1.0;
    let y = rand::rand(#clock, #start: f64:0., #end: f64:100.);
    let a = [];
    a <- array::window(#n: 32, clock ~ a, (x, y));
    a
};

let ds = dataset(
    #style: &style(#fg: `Cyan),
    #graph_type: &`Line,
    #marker: &`Dot,
    &data
);

let label_style = style(#fg: `Yellow);

chart(
    #style: &style(#bg: `Rgb({r: 20, g: 20, b: 20})),
    #x_axis: &axis(
        #title: line(#style: label_style, "Time (s)"),
        #labels: [
            line(#style: label_style, "[(data[0]$).0]"),
            line(#style: label_style, "[(data[-1]$).0]")
        ],
        {min: (data[0]$).0, max: (data[-1]$).0}
    ),
    #y_axis: &axis(
        #title: line(#style: label_style, "Value"),
        #labels: [
            line("0"), line("50"), line("100")
        ],
        {min: 0.0, max: 100.0}
    ),
    &[ds]
)
```

### Multiple Datasets

```graphix
let temp_ds = dataset(
    #style: &style(#fg: `Red),
    #name: &line("Temperature"),
    &temp_data
);

let humidity_ds = dataset(
    #style: &style(#fg: `Blue),
    #name: &line("Humidity"),
    &humidity_data
);

chart(
    #x_axis: &x_axis,
    #y_axis: &y_axis,
    &[temp_ds, humidity_ds]
)
```

### Scatter Plot

```graphix
let scatter_ds = dataset(
    #graph_type: &`Scatter,
    #marker: &`Dot,
    #style: &style(#fg: `Yellow),
    &data
);

chart(#x_axis: &x_axis, #y_axis: &y_axis, &[scatter_ds])
```

## Marker Comparison

- **Dot**: Fastest, lowest resolution, good for dense data
- **Braille**: Smoothest curves, medium performance, best visual quality
- **Block**: High contrast, medium performance

## See Also

- [barchart](barchart.md) - For categorical data visualization
- [sparkline](sparkline.md) - For compact inline charts
- [canvas](canvas.md) - For custom graphics
