use crate::error::FormatError;
use chrono::prelude::*;
use std::time::SystemTime;

struct PdateYDM {
    // These types picked to be friendly for conversion to system time.
    year: i32,
    day: u32,
    month: u32,
}

fn normalize_pdate_year(pdate: u16) -> i32 {
    let offset = ((pdate & 0xfe00) >> 9) as i32;

    // This logic assumes "offset" is between 0-100.  If it's ever > 100,
    // We'll have overlap in 2001-2027
    if offset < 70 {
        // If before 1970, assume it's 20xx.
        offset + 2000
    } else {
        offset + 1900
    }
}

impl PdateYDM {
    fn new(pdate: u16) -> PdateYDM {
        PdateYDM {
            year: normalize_pdate_year(pdate),
            day: ((pdate & 0x01f0) >> 4) as u32,
            month: (pdate & 0x0F) as u32,
        }
    }
}

pub fn pdate_to_systime(pdate: u16) -> SystemTime {
    let ydm = PdateYDM::new(pdate);

    // Meanwhile, since we only get day (not time) we will set it to 0000
    // in whatever timezone TZ is set to. This may cause off-by-one-day
    // problems in the timestamp.

    SystemTime::from(
        Local
            .with_ymd_and_hms(ydm.year, ydm.month, ydm.day, 0, 0, 0)
            .unwrap(),
    )
}

pub fn pdate_to_string(pdate: u16) -> String {
    let ydm = PdateYDM::new(pdate);

    format!("{:04}-{:02}-{:02}", ydm.year, ydm.month, ydm.day)
}

// Inverse of normalize_pdate_year: representable range is 1970-2069, matching
// the offsets that decoding can actually produce (offset 0-99).
pub fn ymd_to_pdate(year: i32, month: u32, day: u32) -> Result<u16, FormatError> {
    let offset: i32 = if (2000..2070).contains(&year) {
        year - 2000
    } else if (1970..2000).contains(&year) {
        year - 1900
    } else {
        return Err(FormatError::InvalidValue {
            field: "pdate year",
            value: year as u32,
        });
    };
    Ok(((offset as u16) << 9) | ((day as u16) << 4) | (month as u16))
}

pub fn systime_to_pdate(t: SystemTime) -> Result<u16, FormatError> {
    let dt: DateTime<Local> = t.into();
    ymd_to_pdate(dt.year(), dt.month(), dt.day())
}

pub fn now_to_pdate() -> Result<u16, FormatError> {
    systime_to_pdate(SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymd_to_pdate_round_trip() {
        let cases = [
            (2026, 7, 25),
            (1984, 11, 7),
            (1999, 12, 31),
            (2000, 1, 1),
            (2069, 6, 15),
        ];
        for (year, month, day) in cases {
            let pdate = ymd_to_pdate(year, month, day).unwrap();
            assert_eq!(
                pdate_to_string(pdate),
                format!("{year:04}-{month:02}-{day:02}")
            );
        }
    }

    #[test]
    fn ymd_to_pdate_rejects_out_of_range() {
        assert!(ymd_to_pdate(1969, 1, 1).is_err());
        assert!(ymd_to_pdate(2070, 1, 1).is_err());
    }
}
