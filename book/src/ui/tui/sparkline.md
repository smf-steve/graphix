# sparkline

The `sparkline` widget renders compact inline charts perfect for dashboards and status displays. It shows data trends in minimal space, with support for color-coded bars based on thresholds.

## Function Signatures

```
type Direction = [`LeftToRight, `RightToLeft];

/// Creates a sparkline widget from data values
val sparkline: fn(
    ?#max: &i64,
    ?#style: &Style,
    ?#direction: &Direction,
    Array<[SparklineBar, f64]>
) -> Widget;

/// Creates a sparkline bar with custom styling
val sparkline_bar: fn(?#style: Style, f64) -> SparklineBar;
```

## Parameters

### sparkline
- **max** - Maximum value for scaling (auto-scales if not specified)
- **style** - Default style for bars
- **direction** - `LeftToRight` (default) or `RightToLeft`

### sparkline_bar
- **style** - Style for this specific bar

## Examples

### Basic Usage

```graphix
use tui;
use tui::sparkline;

let data = [10.0, 25.0, 40.0, 55.0, 70.0, 85.0, 100.0];

sparkline(#max: &100, &data)
```

### Threshold-based Coloring

```graphix
let data: Array<[SparklineBar, f64]> = {
    let clock = time::timer(duration:0.3s, true);
    let v = select rand::rand(#clock, #start:0., #end:100.) {
        x if (x > 50.) && (x < 75.) => sparkline_bar(#style: style(#fg:`Yellow), x),
        x if x > 75. => sparkline_bar(#style: style(#fg:`Red), x),
        x => x  // Use default style
    };
    let d = [];
    d <- array::window(#n:80, clock ~ d, v);
    d
};

block(
    #border: &`All,
    #title: &line("Network Traffic Rate"),
    &sparkline(
        #style: &style(#fg: `Green),
        #max: &100,
        &data
    )
)
```

### Multi-metric Dashboard

```graphix
let cpu_data = [...];
let mem_data = [...];
let net_data = [...];

layout(
    #direction: &`Vertical,
    &[
        child(#constraint: `Length(3), block(
            #title: &line("CPU"),
            &sparkline(#style: &style(#fg: `Red), #max: &100, &cpu_data)
        )),
        child(#constraint: `Length(3), block(
            #title: &line("Memory"),
            &sparkline(#style: &style(#fg: `Yellow), #max: &100, &mem_data)
        )),
        child(#constraint: `Length(3), block(
            #title: &line("Network"),
            &sparkline(#style: &style(#fg: `Cyan), &net_data)
        ))
    ]
)
```

### Rolling Window

```graphix
let data: Array<f64> = [];
let new_value = net::subscribe("/metrics/cpu")?;

data <- array::window(
    #n: 60,
    new_value ~ data,
    cast<f64>(new_value)?
);

sparkline(#max: &100, &data)
```

## Use Cases

Sparklines are ideal for:
- System resource monitoring (CPU, memory, network)
- Real-time metrics dashboards
- Compact data visualization in lists or tables
- Rate of change visualization

## See Also

- [chart](chart.md) - For detailed charts with axes
- [gauge](gauge.md) - For single current value display
- [linegauge](linegauge.md) - For horizontal progress bars
