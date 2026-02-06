//! Date and time utility for ebooks.
//!
//! # Parsing
//! Date and time parsing is best-effort
//! to handle various datetime strings found in ebook metadata.
//!
//! It follows [**ISO 8601-1**](https://www.iso.org/iso-8601-date-and-time-format.html)
//! (`YYYY-MM-DD T HH:mm:ss [Z|±HH:mm]`) where possible, although handles different varieties,
//! such as spacing, different separators, and omitted components.
//!
//! Absent components default to the earliest valid value.
//! For example:
//! - `2024` → `2024-01-01 00:00:00`
//! - `2025.12` → `2025-12-01 00:00:00`
//! - `2026.1.26`
//! - `2024/1/15`
//! - `2023-01-25 10:11:35Z`
//! - `2020-10-12T09:05:01+08:21`
//! - `20250525T121521Z`

use std::fmt::Display;
use std::iter::Peekable;

/// The [date](Date) and [time](Time) components.
///
/// # Examples
/// - Retrieving the modification date and time:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let modified_date = epub.metadata().modified().unwrap();
/// let date = modified_date.date();
/// let time = modified_date.time();
///
/// assert_eq!("2023-01-25T10:11:35Z", modified_date.to_string());
/// assert_eq!((2023, 1, 25), (date.year(), date.month(), date.day()));
/// assert_eq!((10, 11, 35), (time.hour(), time.minute(), time.second()));
/// assert_eq!(Some(0), time.offset()); // UTC offset in minutes
/// assert!(time.is_utc());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    date: Date,
    time: Time,
}

impl DateTime {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();

        let (date_str, time_str) = raw
            .split_once(['T', ' '])
            .map(|(date, time)| (date, Some(time)))
            .unwrap_or((raw, None));

        Some(DateTime {
            date: Date::parse(date_str)?,
            // If `None`, default the time to `00:00:00`
            time: time_str.and_then(Time::parse).unwrap_or(Time::EMPTY),
        })
    }

    /// The [date](Date) (`2025-12-31`).
    ///
    /// # Examples
    /// - Retrieving the publication date:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let published = epub.metadata().published().unwrap();
    /// let date = published.date();
    ///
    /// assert_eq!("2023-01-25", date.to_string());
    /// assert_eq!(2023, date.year());
    /// assert_eq!(1, date.month());
    /// assert_eq!(25, date.day());
    /// # Ok(())
    /// # }
    /// ```
    pub fn date(&self) -> Date {
        self.date
    }

    /// The [time](Time) (`16:52:20Z`).
    ///
    /// # Examples
    /// - Retrieving the modification time:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;///
    /// let modified_date = epub.metadata().modified().unwrap();
    /// let time = modified_date.time();
    ///
    /// assert_eq!("10:11:35Z", time.to_string());
    /// assert_eq!(10, time.hour());
    /// assert_eq!(11, time.minute());
    /// assert_eq!(35, time.second());
    /// assert_eq!(Some(0), time.offset()); // UTC offset in minutes
    /// # Ok(())
    /// # }
    /// ```
    pub fn time(&self) -> Time {
        self.time
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

/// The date, encompassing the [year](Self::year),
/// [month](Self::month), and [day](Self::day).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i16,
    month: u8,
    day: u8,
}

impl Date {
    fn new_clamped(year: i16, month: u8, day: u8) -> Self {
        Self {
            year: year.clamp(-9999, 9999),
            month: month.clamp(1, 12),
            day: day.clamp(1, 31),
        }
    }

    /// If no month or day is provided, the 1st is assumed.
    ///
    /// Supported formats with any separator (`/`, `-`):
    /// - `YYYYMMDD`
    /// - `YYYYMM`
    /// - `YYYY`
    fn parse(raw: &str) -> Option<Self> {
        fn take_date_num(
            chars: &mut Peekable<impl Iterator<Item = char>>,
            max_count: usize,
        ) -> Option<u32> {
            // Iterate to valid ASCII digit
            while let Some(c) = chars.peek() {
                match c {
                    c if c.is_ascii_digit() => break,
                    _ => chars.next(),
                };
            }
            take_num(chars, max_count)
        }

        let mut chars = raw.chars().peekable();

        // Extract the year
        let year = take_date_num(&mut chars, 4)? as i16;
        // Default month/day to the 1st
        let mut month = 1;
        let mut day = 1;

        // Optional month
        if let Some(m) = take_date_num(&mut chars, 2) {
            month = m as u8;

            // Optional day (paired with month)
            if let Some(d) = take_date_num(&mut chars, 2) {
                day = d as u8;
            }
        }

        Some(Date::new_clamped(year, month, day))
    }

    /// The year (typically `0000-9999`).
    pub fn year(&self) -> i16 {
        self.year
    }

    /// The month (`1-12`).
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day (`1-31`).
    ///
    /// # Note
    /// The day does not account for the number of days in a specific month.
    pub fn day(&self) -> u8 {
        self.day
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0>4}-{:0>2}-{:0>2}", self.year, self.month, self.day)
    }
}

/// The time, encompassing the [hour](Self::hour),
/// [minute](Self::minute), [second](Self::second), and [offset](Self::offset).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    offset: Option<i16>,
}

impl Time {
    const EMPTY: Time = Time {
        hour: 0,
        minute: 0,
        second: 0,
        offset: None,
    };

    fn new_clamped(hour: u8, minute: u8, second: u8, offset: Option<i16>) -> Self {
        Self {
            hour: hour.min(23),
            minute: minute.min(59),
            second: second.min(59),
            // Clamp the offset between -12:00 (-720) and +14:00 (840)
            offset: offset.map(|minutes| minutes.clamp(-12 * 60, 14 * 60)),
        }
    }

    /// If no minute or second is provided, 0 is assumed.
    ///
    /// Supported formats with any separator (`:`):
    /// - `hhmmss`
    /// - `hhmm`
    /// - `hh`
    ///
    /// Supported offsets:
    /// - `Z`
    /// - `+hhmm`
    /// - `+hh`
    /// - `-hhmm`
    /// - `-mm`
    fn parse(raw: &str) -> Option<Self> {
        fn take_time_num(
            chars: &mut Peekable<impl Iterator<Item = char>>,
            max_count: usize,
        ) -> Option<u32> {
            // Iterate to valid ASCII digit
            while let Some(c) = chars.peek() {
                match c {
                    // Characters used to denote the time offset
                    'Z' | '-' | '+' => return None,
                    c if c.is_ascii_digit() => break,
                    _ => chars.next(),
                };
            }
            take_num(chars, max_count)
        }

        let mut chars = raw.chars().peekable();

        // Extract the hour
        let hour = take_time_num(&mut chars, 2)? as u8;
        // Default month/day to the 1st
        let mut minute = 0;
        let mut second = 0;
        let mut offset = None;

        // Optional minute
        if let Some(m) = take_time_num(&mut chars, 2) {
            minute = m as u8;

            // Optional second (paired with minute)
            if let Some(s) = take_time_num(&mut chars, 2) {
                second = s as u8;
            }
        }

        // Check if there is an offset
        while let Some(c) = chars.next() {
            if c == 'Z' {
                offset = Some(0);
                break;
            } else if matches!(c, '-' | '+') {
                let sign = if c == '+' { 1 } else { -1 };
                // Need to parse the numbers following the sign
                let hours = take_time_num(&mut chars, 2).unwrap_or(0) as i16;
                let minutes = take_time_num(&mut chars, 2).unwrap_or(0) as i16;
                offset = Some(sign * (hours * 60 + minutes));
                break;
            }
        }

        Some(Time::new_clamped(hour, minute, second, offset))
    }

    /// The number of hours (`0-23`).
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The number of minutes (`0-59`).
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The number of seconds (`0-59`).
    pub fn second(&self) -> u8 {
        self.second
    }

    /// The total UTC offset in minutes (e.g., `+08:30` → `510`).
    ///
    /// # Note
    /// This value is a combination of `HH:mm`:
    ///
    /// | Offset | ±HH:mm   |
    /// |--------|----------|
    /// | `510`  | `+08:30` |
    /// | `-238` | `-03:58` |
    ///
    /// # See Also
    /// - [`Self::offset_hour`] to retrieve the offset in hours (`510` → `8`).
    /// - [`Self::offset_minute`] to retrieve the offset in minutes (`510` → `30`).
    pub fn offset(&self) -> Option<i16> {
        self.offset
    }

    /// The hour component of the UTC [offset](Self::offset) with the sign retained.
    ///
    /// | Offset (±HH:mm) | Hour |
    /// |-----------------|------|
    /// | `00:23`         | `0`  |
    /// | `-00:23`        | `0`  |
    /// | `05:23`         | `5`  |
    /// | `-05:23`        | `-5` |
    pub fn offset_hour(&self) -> Option<i16> {
        self.offset.map(|offset| offset / 60)
    }

    /// The minute component of the UTC [offset](Self::offset) with the sign retained.
    ///
    /// | Offset (±HH:mm) | Minute |
    /// |-----------------|--------|
    /// | `00:23`         | `23`   |
    /// | `-00:23`        | `-23`  |
    pub fn offset_minute(&self) -> Option<i16> {
        self.offset.map(|offset| offset % 60)
    }

    /// Returns `true` if the [offset](Self::offset) is [`None`]
    /// (no specified UTC offset).
    pub fn is_local(&self) -> bool {
        self.offset.is_none()
    }

    /// Returns `true` if the [offset](Self::offset) is [`Some`].
    pub fn is_offset(&self) -> bool {
        self.offset.is_some()
    }

    /// Returns `true` if the [offset](Self::offset) is `0`.
    pub fn is_utc(&self) -> bool {
        self.offset.is_some_and(|offset| offset == 0)
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:0>2}:{:0>2}:{:0>2}",
            self.hour, self.minute, self.second,
        )?;

        // Write offset
        match self.offset {
            Some(0) => write!(f, "Z"),
            Some(offset) => {
                let sign = if offset < 0 { '-' } else { '+' };
                let offset = offset.abs();

                write!(f, "{sign}{:0>2}:{:0>2}", offset / 60, offset % 60)
            }
            _ => Ok(()),
        }
    }
}

fn take_num(chars: &mut Peekable<impl Iterator<Item = char>>, max_count: usize) -> Option<u32> {
    // Extract the number
    let mut num = 0;
    let mut found = false;
    for _ in 0..max_count {
        if let Some(&c) = chars.peek()
            && let Some(digit) = c.to_digit(10)
        {
            num = num * 10 + digit;
            found = true;
            chars.next();
            continue;
        }
        // Break early if the character encountered is not a digit
        break;
    }

    found.then_some(num)
}

#[cfg(feature = "write")]
mod write {
    use crate::ebook::metadata::datetime::{Date, DateTime, Time};

    impl DateTime {
        /// Construct a datetime from the given [`Date`] and [`Time`].
        ///
        /// # See Also
        /// - [`Date::at`] to create a datetime instance.
        ///
        /// # Examples
        /// ```
        /// # use rbook::ebook::metadata::datetime::{Date, DateTime, Time};
        /// let datetime = DateTime::new(
        ///     Date::new(2020, 2, 20),
        ///     Time::new(0, 14, 5, Some(-339)),
        /// );
        ///
        /// assert_eq!("2020-02-20T00:14:05-05:39", datetime.to_string());
        /// assert_eq!(20, datetime.date().day());
        /// assert_eq!(Some(-5), datetime.time().offset_hour());
        /// assert!(datetime.time().is_offset());
        /// ```
        pub fn new(date: Date, time: Time) -> Self {
            Self { date, time }
        }

        /// Returns the current date and UTC time.
        ///
        /// # Panics
        /// Panics on targets where [`std::time::SystemTime`] is unsupported,
        /// such as `wasm32-unknown-unknown`.
        ///
        /// # See Also
        /// - [`Self::try_now`] for a non-panicking variant of this method.
        /// - [`Self::from_unix`] to convert from a UNIX timestamp.
        ///
        /// # Examples
        /// - Retrieving today's date and time:
        /// ```
        /// # use rbook::ebook::metadata::datetime::DateTime;
        /// let today = DateTime::now();
        /// let year = today.date().year();
        /// let hour = today.time().hour();
        ///
        /// println!("Today is {today}");
        /// println!("The year is {year}");
        /// println!("The hour is {hour}");
        /// ```
        pub fn now() -> Self {
            Self::try_now().expect("rbook: std::time::SystemTime is not supported on this platform")
        }

        /// Attempts to return the current date and UTC time.
        ///
        /// Returns `None` on targets where [`std::time::SystemTime`] is unsupported,
        /// such as `wasm32-unknown-unknown`.
        pub fn try_now() -> Option<Self> {
            // Unsupported environments that do not support SystemTime
            // (e.g., wasm32-unknown-unknown)
            #[cfg(all(target_family = "wasm", target_os = "unknown"))]
            {
                None
            }
            // Supported environments (e.g., Linux, Windows, macOS, WASI)
            #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
            {
                let epoch_time = match std::time::UNIX_EPOCH.elapsed() {
                    Ok(after_epoch) => after_epoch.as_secs() as i64,
                    // Handle times before UNIX epoch (1970-1-1T00:00:00Z)
                    Err(before_epoch) => -(before_epoch.duration().as_secs() as i64),
                };

                Some(Self::from_unix(epoch_time))
            }
        }

        /// Returns the date and UTC time from the given UNIX Epoch timestamp.
        ///
        /// # Examples
        /// - Converting from a timestamp:
        /// ```
        /// # use rbook::ebook::metadata::datetime::DateTime;
        /// let datetime = DateTime::from_unix(-58016463);
        ///
        /// assert_eq!("1968-02-29T12:18:57Z", datetime.to_string());
        /// assert_eq!(1968, datetime.date().year());
        /// assert!(datetime.time().is_utc());
        /// ```
        pub fn from_unix(secs: i64) -> Self {
            unix_timestamp_to_utc_calendar(secs)
        }
    }

    impl Date {
        /// Construct a date from the given parts.
        ///
        /// # Clamping
        /// - year: `[-9999, 9999]`
        /// - Month: `[1, 12]`
        /// - Day: `[1, 31]`
        ///
        /// # Examples
        /// - Creating a date:
        /// ```
        /// # use rbook::ebook::metadata::datetime::Date;
        /// let date = Date::new(2012, 7, 8);
        ///
        /// assert_eq!("2012-07-08", date.to_string());
        /// assert_eq!(2012, date.year());
        /// assert_eq!(7, date.month());
        /// assert_eq!(8, date.day());
        /// ```
        pub fn new(year: i16, month: u8, day: u8) -> Self {
            Self::new_clamped(year, month, day)
        }

        /// Create a [`DateTime`] with the given [`Time`].
        ///
        /// # Examples
        /// - Creating a datetime:
        /// ```
        /// # use rbook::ebook::metadata::datetime::{Date, Time};
        /// let date = Date::new(2026, 2, 28);
        /// let time = Time::utc(9, 45, 0);
        /// let datetime = date.at(time);
        ///
        /// assert_eq!("2026-02-28T09:45:00Z", datetime.to_string());
        /// assert_eq!(2026, datetime.date().year());
        /// assert_eq!(45, datetime.time().minute());
        /// ```
        pub fn at(self, time: Time) -> DateTime {
            DateTime { date: self, time }
        }
    }

    impl Time {
        /// Construct a time from the given parts.
        ///
        /// # Clamping
        /// - Hour: `[0, 23]`
        /// - Minute: `[0, 59]`
        /// - Second: `[0, 59]`
        /// - UTC Offset: `[-720, 840]` \[-12:00, +14:00]
        ///
        /// # UTC Offset
        /// The given UTC offset is the total number of minutes (e.g., `+08:30` → `510`).
        /// Passing [`None`] as the UTC offset indicates local time on any system.
        ///
        /// # See Also
        /// - [`Self::utc`] to create a UTC time.
        ///
        /// # Examples
        /// - Creating a time with a UTC offset:
        /// ```
        /// # use rbook::ebook::metadata::datetime::Time;
        /// let time = Time::new(5, 45, 39, Some(205));
        ///
        /// assert_eq!("05:45:39+03:25", time.to_string());
        /// assert!(!time.is_utc());
        /// assert!(time.is_offset());
        /// assert_eq!(Some(205), time.offset());
        /// assert_eq!(Some(3), time.offset_hour());
        /// assert_eq!(Some(25), time.offset_minute());
        /// assert_eq!(5, time.hour());
        /// assert_eq!(45, time.minute());
        /// assert_eq!(39, time.second());
        /// ```
        pub fn new(hour: u8, minute: u8, second: u8, utc_offset: Option<i16>) -> Self {
            Time::new_clamped(hour, minute, second, utc_offset)
        }

        /// Construct a UTC time from the given parts.
        ///
        /// # See Also
        /// - [`Self::new`] for clamping details.
        ///
        /// # Examples
        /// - Creating a UTC time:
        /// ```
        /// # use rbook::ebook::metadata::datetime::Time;
        /// let time = Time::utc(12, 30, 4);
        ///
        /// assert_eq!("12:30:04Z", time.to_string());
        /// assert!(time.is_utc());
        /// assert_eq!(Some(0), time.offset());
        /// assert_eq!(12, time.hour());
        /// assert_eq!(30, time.minute());
        /// assert_eq!(4, time.second());
        /// ```
        pub fn utc(hour: u8, minute: u8, second: u8) -> Self {
            Self::new(hour, minute, second, Some(0))
        }
    }

    /// UNIX timestamp to UTC Gregorian calendar.
    ///
    /// Based on Howard Hinnant's date algorithms:
    /// <https://howardhinnant.github.io/date_algorithms.html>
    fn unix_timestamp_to_utc_calendar(secs: i64) -> DateTime {
        // Calculate `Date` components (Howard Hinnant's Algorithm)
        let days_since_epoch = secs.div_euclid(86400) as i32;
        let secs_of_day = secs.rem_euclid(86400) as u32;

        // Calculate `Time` components
        let second = (secs_of_day % 60) as u8;
        let minute = ((secs_of_day / 60) % 60) as u8;
        let hour = ((secs_of_day / 3600) % 24) as u8;

        // Calculate `Date` components (Howard Hinnant's Algorithm)
        let z = days_since_epoch + 719468; // Shift epoch from 1970-01-01 to 0000-03-01
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = (z - era * 146097) as u32;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i32 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };

        let year = y + (m <= 2) as i32;
        let month = m as u8;
        let day = d as u8;

        DateTime::new(
            Date::new(year as i16, month, day),
            Time::utc(hour, minute, second),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        #[rustfmt::skip]
        let expected = [
            ((2025, 1, 1, 0, 0, 0, None), "2025"),
            ((2023, 10, 27, 15, 30, 5, Some(0)), "2023-10-27T15:30:05Z"),
            ((2023, 10, 27, 15, 30, 0, Some(120)), "2023-10-27 15:30+02:00"),
            ((2025, 12, 1, 0, 0, 0, None), "2025-13-01"),
            ((2025, 1, 1, 0, 0, 0, None), "2025-00-00"),
            ((2025, 1, 31, 0, 0, 0, None), "2025-01-32"),
            ((2025, 1, 1, 23, 59, 59, Some(0)), "2025-01-01 25:61:99Z"),
            ((2020, 1, 1, 20, 0, 0, None), "2020-01-01T20"),
            ((2020, 5, 20, 0, 0, 0, None), "20200520"),
            ((1999, 12, 31, 23, 59, 0, None), "1999/12/31 23:59"),
            ((2022, 1, 1, 12, 0, 0, Some(-480)), "2022.01.01 12:00:00-0800[T/Z]"),
            ((2021, 1, 1, 0, 0, 0, None), "  2021-01-01  "),
            ((2021, 6, 1, 0, 0, 0, None), "2021-06-unknown"),
            ((5, 1, 1, 0, 0, 0, None), "0005-01-01"),
            ((5, 12, 1, 0, 0, 0, None), "5-50T"),
        ];

        for ((y, m, d, hh, mm, ss, off), raw) in expected {
            let datetime =
                DateTime::parse(raw).unwrap_or_else(|| panic!("Failed to parse: {}", raw));
            let expected_date = Date {
                year: y,
                month: m,
                day: d,
            };
            let expected_time = Time {
                hour: hh,
                minute: mm,
                second: ss,
                offset: off,
            };

            assert_eq!(datetime.date, expected_date, "Date mismatch for: {raw}");
            assert_eq!(datetime.time, expected_time, "Time mismatch for: {raw}");
        }
    }

    #[test]
    fn test_parse_datetime_fail() {
        #[rustfmt::skip]
        let expected = [
            "T12:00:00",
        ];

        for raw in expected {
            assert_eq!(None, DateTime::parse(raw), "{raw}");
        }
    }

    #[test]
    fn test_parse_date() {
        #[rustfmt::skip]
        let expected = [
            ((0, 1, 1), "0"),
            ((1902, 2, 11), "- 1902  2  11 "),
            ((2025, 1, 1), "2025"),
            ((2025, 8, 1), "20258"),
            ((2025, 12, 1), "202518"),
            ((2025, 12, 2), "202518/2"),
            ((2003, 10, 23), "2003.10.23"),
            ((2025, 8, 1), "2025/8"),
            ((2021, 12, 1), "2021-16-a"),
            ((2070, 5, 1), "2070-b-5"),
            ((2030, 12, 1), "203012"),
            ((2025, 12, 1), "2025-12"),
            ((2025, 12, 31), "2025-12-31"),
            ((2001, 6, 7),  " 2001 / 06 / 07 "),
            ((2026, 4, 5), "2026/4/5"),
            ((2029, 8, 16), "2029/8/16"),
            ((589, 5, 5), "589/5/05"),
            ((5, 5, 5), "5-5-5"),
        ];

        for ((year, month, day), raw) in expected {
            assert_eq!(
                Some(Date { year, month, day }),
                Date::parse(raw),
                "Date mismatch for: {raw}",
            );
        }
    }

    #[test]
    fn test_parse_date_fail() {
        #[rustfmt::skip]
        let expected = [
            "",
            "abc",
            "---",
            "/",
            "n/a",
        ];

        for raw in expected {
            assert_eq!(None, Date::parse(raw), "{raw}");
        }
    }

    #[test]
    fn test_parse_time() {
        #[rustfmt::skip]
        let expected = [
            ((13, 0, 0, Some(615)), "13+10:15"),
        ];

        for ((hour, minute, second, offset), raw) in expected {
            assert_eq!(
                Some(Time {
                    hour,
                    minute,
                    second,
                    offset,
                }),
                Time::parse(raw),
                "Time mismatch for: {raw}",
            );
        }
    }

    #[test]
    fn test_parse_time_fail() {
        #[rustfmt::skip]
        let expected = [
            "Z",
            "",
            "xyz",
            "---Z",
            "+3",
            "-9",
            "n/a",
            "+00",
            "-10:03",
        ];

        for raw in expected {
            assert_eq!(None, Time::parse(raw));
        }
    }

    #[test]
    #[cfg(feature = "write")]
    fn test_unix_to_datetime() {
        #[rustfmt::skip]
        let cases = [
            (0, 1970, 1, 1, 0, 0, 0), // Unix Epoch
            (1709164800, 2024, 2, 29, 0, 0, 0), // 2024-03-01 (Leap Day)
            (1709251200, 2024, 3, 1, 0, 0, 0), // 2024-03-01 (Day after a leap day)
            (1709251199, 2024, 2, 29, 23, 59, 59), // 2024-02-29 23:59:59 (Leap year divisible by 4)
            (951825600, 2000, 2, 29, 12, 0, 0), // 2000-02-29 12:00:00 (Leap year divisible by 400)
            (4107542399, 2100, 2, 28, 23, 59, 59), // 2100-02-28 23:59:59
            (4107542400, 2100, 3, 1, 0, 0, 0), // 2100-03-01 (Should skip 2/29)
            (2147483647, 2038, 1, 19, 3, 14, 7), // 2038-01-19 03:14:07 (Y2K38)
            (16725225600, 2500, 1, 1, 0, 0, 0), // 2500-01-01
            (253402300799, 9999, 12, 31, 23, 59, 59), // Max 4-digit year (9999)

            // Negative timestamps
            (-1, 1969, 12, 31, 23, 59, 59), // 1 second before Epoch
            (-86400, 1969, 12, 31, 0, 0, 0), // Exactly 1 day before Epoch
            (-58060800, 1968, 2, 29, 0, 0, 0), // 1968-02-29 (Leap Day)
            (-2147483648, 1901, 12, 13, 20, 45, 52),
            (-2208988800, 1900, 1, 1, 0, 0, 0),
            (-2203977600, 1900, 2, 28, 0, 0, 0),
            (-2203891200, 1900, 3, 1, 0, 0, 0),
            (-4952457600, 1813, 1, 23, 21, 20, 0),
            (-62135596800, 1, 1, 1, 0, 0, 0), // Year 1 (Gregorian Projection)
        ];

        for (stamp, y, m, d, hh, mm, ss) in cases {
            let datetime = DateTime::from_unix(stamp);
            let expected_date = Date {
                year: y,
                month: m,
                day: d,
            };
            let expected_time = Time {
                hour: hh,
                minute: mm,
                second: ss,
                offset: Some(0),
            };

            assert_eq!(datetime.date, expected_date, "Date mismatch for: {stamp}");
            assert_eq!(datetime.time, expected_time, "Time mismatch for: {stamp}");
        }
    }
}
