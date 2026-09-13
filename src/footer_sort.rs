// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::cmp::Ordering;

pub fn compare_values(sort_key: &str, left: &str, right: &str) -> Ordering {
    if sort_key == "date" {
        // The classes are ordered first — every unparseable value before every
        // date — and the string fallback only ever compares within the
        // unparseable class, so the order stays a strict weak ordering when
        // the two mix. Deciding per-pair — dates when both parse, strings
        // otherwise — produced cycles ("9 May 2020" < "10 May 2021" as dates,
        // "10 May 2021" < "5 drafts" < "9 May 2020" as strings), and a
        // comparator with a cycle makes `sort_by` panic. Two spellings of the
        // same date stay `Equal`, as pinned below.
        return match (parse_date(left), parse_date(right)) {
            (Some(left_date), Some(right_date)) => left_date.cmp(&right_date),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => left.cmp(right),
        };
    }

    left.cmp(right)
}

pub(crate) fn parse_date(value: &str) -> Option<(u32, u8, u8)> {
    let text = value.trim();
    if text.is_empty() {
        return None;
    }

    parse_numeric_date(text).or_else(|| parse_named_month_date(text))
}

fn parse_numeric_date(text: &str) -> Option<(u32, u8, u8)> {
    let parts: Vec<&str> = text
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }

    if parts[0].len() == 4 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u8>().ok()?;
        let day = parts[2].parse::<u8>().ok()?;
        return validate_ymd(year, month, day);
    }

    if parts[2].len() == 4 {
        let year = parts[2].parse::<u32>().ok()?;
        let first = parts[0].parse::<u8>().ok()?;
        let second = parts[1].parse::<u8>().ok()?;

        // Prefer month/day/year, then try day/month/year.
        return validate_ymd(year, first, second).or_else(|| validate_ymd(year, second, first));
    }

    None
}

fn parse_named_month_date(text: &str) -> Option<(u32, u8, u8)> {
    let normalized = text.replace(',', " ");
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    if let Some(month) = parse_month_token(parts[0]) {
        let day = parse_u8_prefix(parts[1])?;
        let year = parse_u32_prefix(parts[2])?;
        return validate_ymd(year, month, day);
    }

    let day = parse_u8_prefix(parts[0])?;
    let month = parse_month_token(parts[1])?;
    let year = parse_u32_prefix(parts[2])?;
    validate_ymd(year, month, day)
}

fn parse_month_token(token: &str) -> Option<u8> {
    let month = token
        .trim_matches(|ch: char| !ch.is_ascii_alphabetic())
        .to_ascii_lowercase();

    match month.as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

fn parse_u8_prefix(token: &str) -> Option<u8> {
    let len = token.chars().take_while(|ch| ch.is_ascii_digit()).count();
    (len > 0).then(|| token[..len].parse::<u8>().ok()).flatten()
}

fn parse_u32_prefix(token: &str) -> Option<u32> {
    let len = token.chars().take_while(|ch| ch.is_ascii_digit()).count();
    (len > 0)
        .then(|| token[..len].parse::<u32>().ok())
        .flatten()
}

/// Whether `year-month-day` names a real Gregorian date.
///
/// Shared by the date parsing here and the RFC 822 formatting in the RSS feed,
/// so the two can never disagree about which dates exist.
pub(crate) fn is_valid_calendar_date(year: u32, month: u8, day: u8) -> bool {
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    day != 0 && day <= max_day
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100))
}

fn validate_ymd(year: u32, month: u8, day: u8) -> Option<(u32, u8, u8)> {
    is_valid_calendar_date(year, month, day).then_some((year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression test: choosing date-order or string-order per pair produced a
    // cycle across these three values, and std's `sort_by` may panic outright
    // on a comparator that is not a total order.
    #[test]
    fn test_compare_values_stays_total_when_dates_and_non_dates_mix() {
        let mut values = ["10 May 2021", "5 drafts", "9 May 2020"];
        values.sort_by(|left, right| compare_values("date", left, right));
        // Unparseable values group first, real dates follow in date order.
        assert_eq!(values, ["5 drafts", "9 May 2020", "10 May 2021"]);

        // The old comparator's cycle, spelled out: each pair must now agree
        // with the sorted order above.
        assert_eq!(
            compare_values("date", "9 May 2020", "10 May 2021"),
            Ordering::Less
        );
        assert_eq!(
            compare_values("date", "10 May 2021", "5 drafts"),
            Ordering::Greater
        );
        assert_eq!(
            compare_values("date", "5 drafts", "9 May 2020"),
            Ordering::Less
        );
    }

    #[test]
    fn test_compare_values_parses_textual_month_dates() {
        assert_eq!(
            compare_values("date", "August 15, 2021", "January 2, 2020"),
            Ordering::Greater
        );
        assert_eq!(
            compare_values("date", "15 Aug 2021", "2021-08-15"),
            Ordering::Equal
        );
    }

    #[test]
    fn test_compare_values_falls_back_to_string_when_unparseable() {
        assert_eq!(compare_values("date", "unknown", "zzz"), Ordering::Less);
        assert_eq!(compare_values("title", "b", "a"), Ordering::Greater);
    }

    #[test]
    fn test_parse_date_applies_the_full_gregorian_leap_year_rule() {
        // All three clauses matter: divisible by 4, except centuries, except
        // those divisible by 400.
        assert_eq!(parse_date("2024-02-29"), Some((2024, 2, 29)), "leap year");
        assert_eq!(parse_date("2000-02-29"), Some((2000, 2, 29)), "400-year");
        assert_eq!(parse_date("1900-02-29"), None, "century, not a leap year");
        assert_eq!(parse_date("2023-02-29"), None, "not divisible by 4");
        assert_eq!(parse_date("2023-02-28"), Some((2023, 2, 28)));
    }

    #[test]
    fn test_parse_date_rejects_out_of_range_days() {
        assert_eq!(parse_date("2024-04-31"), None, "April has 30 days");
        assert_eq!(parse_date("2024-01-32"), None);
        assert_eq!(parse_date("2024-13-01"), None, "no thirteenth month");
        assert_eq!(parse_date("2024-01-00"), None, "no zeroth day");
    }
}
