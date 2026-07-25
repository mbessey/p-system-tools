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
