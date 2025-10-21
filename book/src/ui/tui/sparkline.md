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
{{#include ../../examples/tui/sparkline_basic.gx}}
```

### Threshold-based Coloring

```graphix
{{#include ../../examples/tui/sparkline_threshold.gx}}
```

### Multi-metric Dashboard

```graphix
{{#include ../../examples/tui/sparkline_dashboard.gx}}
```

### Rolling Window

```graphix
{{#include ../../examples/tui/sparkline_rolling.gx}}
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
