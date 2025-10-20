# canvas

The `canvas` widget provides a low-level drawing surface for custom graphics. You can draw lines, circles, rectangles, points, and text labels at specific coordinates, making it perfect for diagrams, plots, and custom visualizations.

## Function Signatures

```
type Bounds = {min: f64, max: f64};
type Shape = [
    `Line({color: Color, x1: f64, y1: f64, x2: f64, y2: f64}),
    `Circle({color: Color, x: f64, y: f64, radius: f64}),
    `Rectangle({color: Color, x: f64, y: f64, width: f64, height: f64}),
    `Points({color: Color, coords: Array<(f64, f64)>}),
    `Label({line: Line, x: f64, y: f64})
];

/// Creates a canvas widget for custom graphics
val canvas: fn(
    ?#background_color: &Color,
    ?#marker: &Marker,
    #x_bounds: &Bounds,
    #y_bounds: &Bounds,
    Array<&Shape>
) -> Widget;
```

## Parameters

- **background_color** - Background color for the canvas
- **marker** - Marker type: `Dot`, `Braille` (default), or `Block`
- **x_bounds** - X-axis range with `min` and `max` fields (required)
- **y_bounds** - Y-axis range with `min` and `max` fields (required)

## Shape Types

### Line
```graphix
`Line({color: `Red, x1: 0.0, y1: 0.0, x2: 10.0, y2: 5.0})
```

### Circle
```graphix
`Circle({color: `Blue, x: 5.0, y: 5.0, radius: 2.0})
```

### Rectangle
```graphix
`Rectangle({color: `Green, x: 2.0, y: 2.0, width: 3.0, height: 4.0})
```

### Points
```graphix
`Points({color: `Yellow, coords: [(1.0, 1.0), (2.0, 3.0), (3.0, 1.5)]})
```

### Label
```graphix
`Label({line: line("Hello"), x: 5.0, y: 0.5})
```

## Examples

### Basic Usage

```graphix
use tui;
use tui::canvas;

let line = `Line({color: `Red, x1: 0.0, y1: 0.0, x2: 10.0, y2: 5.0});
let circle = `Circle({color: `Blue, x: 5.0, y: 5.0, radius: 2.0});

canvas(
    #x_bounds: &{min: 0.0, max: 10.0},
    #y_bounds: &{min: 0.0, max: 10.0},
    &[&line, &circle]
)
```

### Function Plotting

```graphix
let points = array::range(0, 100);
let coords = array::map(points, |i| {
    let x = cast<f64>(i)? / 10.0;
    let y = math::sin(x);
    (x, y)
});

let plot = `Points({color: `Cyan, coords});

canvas(
    #x_bounds: &{min: 0.0, max: 10.0},
    #y_bounds: &{min: -1.0, max: 1.0},
    &[&plot]
)
```

### Network Diagram

```graphix
type Node = {x: f64, y: f64, label: string};

let nodes = [
    {x: 2.0, y: 5.0, label: "A"},
    {x: 8.0, y: 5.0, label: "B"},
    {x: 5.0, y: 8.0, label: "C"}
];

let circles = array::map(nodes, |n| {
    &`Circle({color: `Blue, x: n.x, y: n.y, radius: 0.5})
});

let labels = array::map(nodes, |n| {
    &`Label({line: line(n.label), x: n.x, y: n.y - 0.8})
});

let line1 = `Line({color: `White, x1: 2.0, y1: 5.0, x2: 8.0, y2: 5.0});
let line2 = `Line({color: `White, x1: 2.0, y1: 5.0, x2: 5.0, y2: 8.0});
let line3 = `Line({color: `White, x1: 8.0, y1: 5.0, x2: 5.0, y2: 8.0});

let all_shapes = array::concat([[&line1, &line2, &line3], circles, labels]);

canvas(#x_bounds: &{min: 0.0, max: 10.0}, #y_bounds: &{min: 0.0, max: 10.0}, &all_shapes)
```

### Animated Graphics

```graphix
let clock = time::timer(duration:0.1s, true);
let x = 0.0;
x <- {
    let new_x = (clock ~ x) + 0.1;
    select new_x > 10.0 { true => 0.0, false => new_x }
};

let moving_circle = `Circle({color: `Red, x, y: 5.0, radius: 1.0});

canvas(#x_bounds: &{min: 0.0, max: 10.0}, #y_bounds: &{min: 0.0, max: 10.0}, &[&moving_circle])
```

## Marker Comparison

- **Braille**: Highest resolution, smoothest curves, best for detailed graphics
- **Dot**: Fast rendering, lower resolution, good for simple shapes
- **Block**: High contrast, blocky appearance, good for filled areas

## Coordinate System

- Origin (0, 0) is at the bottom-left
- X increases to the right
- Y increases upward
- Shapes outside bounds are clipped

## See Also

- [chart](chart.md) - For pre-built line charts
- [barchart](barchart.md) - For bar-based visualizations
