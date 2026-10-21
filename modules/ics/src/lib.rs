use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use libcommand::{impl_command, CommandClient, File, TrinityCommand};
use time::{macros::format_description, Date, Time};
use wit_log::{self as log, trace, warn};

struct Component;

impl TrinityCommand for Component {
    fn init(_config: HashMap<String, String>) {
        let _ = log::set_boxed_logger(Box::new(crate::log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
    }

    fn on_help(_topic: Option<&str>) -> String {
        r#"Parse lines like "14/11/2026 17h30 18h30 apéro" and creates an ICS event from them." "#
            .to_owned()
    }

    fn on_msg(client: &mut CommandClient, content: &str) {
        let Some(content) = content.strip_prefix("!ics").map(|rest| rest.trim()) else {
            return;
        };

        if let Some(parsed) = Self::parse_simple_line(content) {
            trace!("found event! {}", parsed.topic);

            let Some(buffer) = Self::render_ics(&parsed) else {
                warn!("couldn't render the ICS event");
                return;
            };

            let name = "event.ics".to_owned();

            client.upload(File {
                buffer: buffer.bytes().collect(),
                name,
                legend: None,
            });
        }
    }
}

struct Event {
    date: Date,
    start: Time,
    end: Time,
    topic: String,
}

impl Component {
    fn escape_text(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace(';', "\\;")
            .replace(',', "\\,")
    }

    fn render_ics(event: &Event) -> Option<String> {
        let date_desc = format_description!("[year][month][day]");
        let time_desc = format_description!("[hour][minute][second]");

        let date = event.date.format(date_desc).ok()?;
        let start = event.start.format(time_desc).ok()?;
        let end = event.end.format(time_desc).ok()?;

        //TODO: add wit_time module?
        //let stamp_desc = format_description!("[year][month][day]T[hour][minute][second]Z");
        //let dtstamp = OffsetDateTime::now_utc().format(stamp_desc).ok()?;
        let dtstamp = "20261021T143042Z";

        // Stable UID, so re-importing the same event doesn't duplicate it.
        let mut hasher = DefaultHasher::new();
        (&date, &start, &end, &event.topic).hash(&mut hasher);
        let uid = format!("{date}T{start}-{:016x}@trinity", hasher.finish());

        let summary = Self::escape_text(&event.topic);

        let lines = [
            "BEGIN:VCALENDAR",
            "VERSION:2.0",
            "PRODID:-//Trinity//ics//EN",
            "CALSCALE:GREGORIAN",
            "METHOD:PUBLISH",
            "BEGIN:VTIMEZONE",
            "TZID:Europe/Paris",
            "BEGIN:DAYLIGHT",
            "TZOFFSETFROM:+0100",
            "TZOFFSETTO:+0200",
            "TZNAME:CEST",
            "DTSTART:19700329T020000",
            "RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU",
            "END:DAYLIGHT",
            "BEGIN:STANDARD",
            "TZOFFSETFROM:+0200",
            "TZOFFSETTO:+0100",
            "TZNAME:CET",
            "DTSTART:19701025T030000",
            "RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU",
            "END:STANDARD",
            "END:VTIMEZONE",
            "BEGIN:VEVENT",
            &format!("UID:{uid}"),
            &format!("DTSTAMP:{dtstamp}"),
            &format!("DTSTART;TZID=Europe/Paris:{date}T{start}"),
            &format!("DTEND;TZID=Europe/Paris:{date}T{end}"),
            &format!("SUMMARY:{summary}"),
            "END:VEVENT",
            "END:VCALENDAR",
        ];

        // RFC 5545 requires CRLF line endings.
        Some(lines.join("\r\n") + "\r\n")
    }

    /// Parse lines like: 19/11/2026 17:30 21:30 Rennes - Lyon Part Dieu.
    fn parse_simple_line(s: &str) -> Option<Event> {
        // Remove markdown formatting.
        let s = s.replace("*", "").replace("_", "");

        let mut split = s.split_whitespace();

        let date_desc = format_description!("[day]/[month]/[year]");
        let candidate = split.next()?;
        let date = Date::parse(candidate, date_desc).ok()?;

        let hour_desc = format_description!("[hour]:[minute]");
        let candidate = split.next()?;
        let start = Time::parse(candidate, hour_desc).ok()?;

        let candidate = split.next()?;
        let end = Time::parse(candidate, hour_desc).ok()?;

        let topic = split.collect::<Vec<_>>().join(" ");

        Some(Event {
            date,
            start,
            end,
            topic,
        })
    }
}

impl_command!();
