# calendar

The `calendar` widget displays a monthly calendar view with support for highlighting specific dates and displaying events. It's perfect for date pickers, event schedulers, and time-based visualizations.

## Function Signatures

```
/// Creates a calendar widget displaying a month
val calendar: fn(
    ?#show_month: &Style,
    ?#show_weekday: &Style,
    ?#show_surrounding: &Style,
    ?#events: &Array<CalendarEvent>,
    Date
) -> Widget;

/// Creates an event marker for a specific date
val calendar_event: fn(Style, Date) -> CalendarEvent;

/// Creates a date object
val date: fn(i64, i64, i64) -> Date;  // (year, month, day)
```

## Parameters

### calendar
- **show_month** - Style for the month header
- **show_weekday** - Style for weekday headers (Mon, Tue, etc.)
- **show_surrounding** - Style for dates from surrounding months
- **events** - Array of CalendarEvent objects to highlight dates

### calendar_event
Takes a style and a date to create an event marker.

### date
Creates a date with year, month (1-12), and day (1-31).

## Examples

### Basic Usage

```graphix
use tui;
use tui::calendar;

let current_date = date(2024, 5, 15);

calendar(&current_date)
```

### Event Calendar

```graphix
let today = date(2024, 5, 15);

let events = [
    calendar_event(style(#fg: `Red), date(2024, 5, 5)),
    calendar_event(style(#fg: `Green), date(2024, 5, 15)),
    calendar_event(style(#fg: `Yellow), date(2024, 5, 20)),
    calendar_event(style(#fg: `Cyan), date(2024, 5, 28))
];

block(
    #border: &`All,
    #title: &line("May 2024"),
    &calendar(
        #show_month: &style(#fg: `Yellow, #add_modifier: `Bold),
        #show_weekday: &style(#fg: `Cyan),
        #show_surrounding: &style(#fg: `DarkGray),
        #events: &events,
        &today
    )
)
```

### Color-coded Events by Type

```graphix
type EventType = [`Meeting, `Deadline, `Holiday, `Birthday];
type CalendarEntry = {date: Date, event_type: EventType};

let entries = [
    {date: date(2024, 5, 5), event_type: `Meeting},
    {date: date(2024, 5, 10), event_type: `Deadline},
    {date: date(2024, 5, 15), event_type: `Holiday},
    {date: date(2024, 5, 25), event_type: `Birthday}
];

let events = array::map(entries, |e| {
    let color = select e.event_type {
        `Meeting => `Blue,
        `Deadline => `Red,
        `Holiday => `Green,
        `Birthday => `Magenta
    };
    calendar_event(style(#fg: color), e.date)
});

calendar(#events: &events, &date(2024, 5, 1))
```

## See Also

- [table](table.md) - For tabular date-based data
- [list](list.md) - For event lists
- [block](block.md) - For containing calendars with borders
