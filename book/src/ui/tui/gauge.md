# gauge

The `gauge` widget displays a single value as a filled progress indicator, perfect for showing percentages, completion status, or resource usage. It provides a clear visual representation of how full or complete something is.

## Function Signature

```
/// Creates a gauge widget showing progress from 0.0 to 1.0
val gauge: fn(
    ?#gauge_style: &Style,
    ?#label: &Line,
    ?#use_unicode: &bool,
    ?#style: &Style,
    f64
) -> Widget;
```

## Parameters

- **gauge_style** - Style for the filled portion
- **label** - Line or span displayed in the center
- **use_unicode** - Use Unicode block characters for smoother rendering
- **style** - Style for the unfilled portion

## Examples

### Basic Usage

```graphix
{{#include ../../examples/tui/gauge_basic.gx}}
```

### Progress with Color Thresholds

```graphix
{{#include ../../examples/tui/gauge_threshold.gx}}
```

### Resource Usage

```graphix
{{#include ../../examples/tui/gauge_resource.gx}}
```

### Multi-gauge Dashboard

```graphix
layout(
    #direction: &`Vertical,
    &[
        child(#constraint: `Length(1), gauge(
            #gauge_style: &style(#fg: `Red),
            #label: &line("CPU: 45%"),
            &0.45
        )),
        child(#constraint: `Length(1), gauge(
            #gauge_style: &style(#fg: `Yellow),
            #label: &line("Memory: 67%"),
            &0.67
        )),
        child(#constraint: `Length(1), gauge(
            #gauge_style: &style(#fg: `Green),
            #label: &line("Disk: 23%"),
            &0.23
        ))
    ]
)
```

## See Also

- [linegauge](linegauge.md) - For horizontal line-based gauges
- [sparkline](sparkline.md) - For historical trend display
- [barchart](barchart.md) - For comparing multiple values
