//! The window's account of present calls in display-link refresh intervals.

use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Place {
    DisplayLink,
    LateRequest,
    FrameSlot,
    Drawable,
    EncodeSubmit,
}

impl Place {
    const ALL: [Self; 5] = [
        Self::DisplayLink,
        Self::LateRequest,
        Self::FrameSlot,
        Self::Drawable,
        Self::EncodeSubmit,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::DisplayLink => "display_link",
            Self::LateRequest => "late_request",
            Self::FrameSlot => "frame_slot",
            Self::Drawable => "drawable",
            Self::EncodeSubmit => "encode_submit",
        }
    }
}

#[derive(Clone, Copy)]
struct Present {
    time: f64,
    submission: u64,
}

#[derive(Clone, Copy)]
struct Opportunity {
    target: f64,
    place: Place,
    requested_present: Option<Present>,
    interval_present: Option<Present>,
    multiple: bool,
}

impl Opportunity {
    fn new(target: f64, place: Place) -> Self {
        Self {
            target,
            place,
            requested_present: None,
            interval_present: None,
            multiple: false,
        }
    }

    fn unserved(self) -> Option<Place> {
        match (self.interval_present, self.multiple) {
            (_, true) => Some(Place::EncodeSubmit),
            (None, false) => Some(self.place),
            (Some(_), false) => None,
        }
    }
}

struct Measurement {
    start: f64,
    end: f64,
    rows: Vec<Opportunity>,
}

struct Request {
    target: f64,
    late: bool,
    closed: Option<(Opportunity, Option<usize>)>,
}

/// Only the render owner starts, completes and presents a request. The event loop adds
/// targets before dispatching work for them. Between targets it can get at most the
/// present already in flight and one for the latest callback; a refusal may be retried.
#[derive(Default)]
pub(super) struct Account {
    current: Option<Opportunity>,
    previous_end: Option<f64>,
    presents: [Option<Present>; 2],
    request: Option<Request>,
    measurement: Option<Measurement>,
    opportunities: u64,
    served: u64,
    unserved: [u64; 5],
    excess_presents: u64,
}

impl Account {
    pub(super) fn measure(&mut self, start: f64, length: f64) {
        if self.measurement.is_none() {
            self.measurement = Some(Measurement {
                start,
                end: start + length,
                rows: Vec::new(),
            });
        }
    }

    pub(super) fn callback(&mut self, target: f64, period: f64) {
        if !target.is_finite() || !period.is_finite() || period <= 0.0 {
            return;
        }
        if self.current.is_some_and(|row| target <= row.target) {
            return;
        }
        if let Some(previous) = self.current.take() {
            self.close(previous);
            // A target gap contains the nominal deadlines the link did not send.
            // Anchor them to the new target so a small change in phase cannot drift.
            let periods = ((target - previous.target) / period).round() as u64;
            for gap in (1..periods).rev() {
                self.close(Opportunity::new(
                    target - gap as f64 * period,
                    Place::DisplayLink,
                ));
            }
        } else {
            self.previous_end = Some(target - period);
        }
        self.current = Some(Opportunity::new(target, Place::LateRequest));
    }

    pub(super) fn begin(&mut self, target: f64, arrived: f64) {
        self.request = Some(Request {
            target,
            late: arrived >= target,
            closed: None,
        });
    }

    pub(super) fn present(&mut self, time: f64, submission: u64) {
        let present = Present { time, submission };
        if let Some(slot) = self.presents.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(present);
        } else {
            // A failed device can discard a present reply and retry the same target.
            // Expose such a discontinuity in the record, without an unbounded backlog.
            self.excess_presents += 1;
        }
    }

    pub(super) fn complete(&mut self, place: Place, call: Option<(f64, u64)>) {
        let Some(request) = self.request.take() else {
            return;
        };
        // Refusals can be retried on the same target. Preserve a stage reached by
        // an earlier on-time attempt if a later retry only finds an expired target.
        let update = |row: &mut Opportunity| {
            if !request.late {
                row.place = place;
            }
            if let Some((time, submission)) = call {
                row.requested_present = Some(Present { time, submission });
            }
        };
        if let Some(row) = &mut self.current
            && row.target == request.target
        {
            update(row);
        } else if let Some((mut row, index)) = request.closed {
            let before = row.unserved();
            update(&mut row);
            if let (Some(before), Some(after)) = (before, row.unserved()) {
                self.unserved[before as usize] -= 1;
                self.unserved[after as usize] += 1;
            }
            if let (Some(measurement), Some(index)) = (&mut self.measurement, index) {
                measurement.rows[index] = row;
            }
        }
    }

    fn close(&mut self, mut row: Opportunity) {
        for slot in &mut self.presents {
            if let Some(present) = *slot
                && present.time < row.target
            {
                if self.previous_end.is_some_and(|start| present.time >= start) {
                    row.multiple |= row.interval_present.replace(present).is_some();
                }
                *slot = None;
            }
        }
        self.previous_end = Some(row.target);
        self.opportunities += 1;
        if let Some(place) = row.unserved() {
            self.unserved[place as usize] += 1;
        } else {
            self.served += 1;
        }
        let index = self.measurement.as_mut().and_then(|measurement| {
            (measurement.start <= row.target && row.target < measurement.end).then(|| {
                measurement.rows.push(row);
                measurement.rows.len() - 1
            })
        });
        if let Some(request) = &mut self.request
            && row.target == request.target
        {
            request.closed = Some((row, index));
        }
    }

    pub(super) fn finish(&mut self) {
        if let Some(row) = self.current.take() {
            self.close(row);
        }
    }

    pub(super) fn counts(&self) -> String {
        let mut report = format!(
            "refresh opportunities={}; served={}; excess_present_records={}",
            self.opportunities, self.served, self.excess_presents,
        );
        for place in Place::ALL {
            use std::fmt::Write as _;
            let _ = write!(
                report,
                "; {}={}",
                place.name(),
                self.unserved[place as usize]
            );
        }
        report
    }

    pub(super) fn write(&self, output: &std::path::Path) -> io::Result<()> {
        std::fs::write(output.join("refresh-counts.txt"), self.counts() + "\n")?;
        let Some(measurement) = &self.measurement else {
            return Ok(());
        };
        std::fs::write(
            output.join("refresh-interval.tsv"),
            format!(
                "start_media_seconds\tend_media_seconds\n{:.9}\t{:.9}\n",
                measurement.start, measurement.end
            ),
        )?;
        let mut file =
            io::BufWriter::new(std::fs::File::create(output.join("refresh-account.tsv"))?);
        writeln!(
            file,
            "target_media_seconds\tpresent_call_media_seconds\tsubmission_id\tinterval_call_media_seconds\tinterval_submission_id\tplace"
        )?;
        for row in &measurement.rows {
            let columns = |present: Option<Present>| {
                present.map_or_else(
                    || "\t".to_owned(),
                    |present| format!("{:.9}\t{}", present.time, present.submission),
                )
            };
            writeln!(
                file,
                "{:.9}\t{}\t{}\t{}",
                row.target,
                columns(row.requested_present),
                columns(row.interval_present),
                row.unserved().map_or("served", Place::name),
            )?;
        }
        file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::{Account, Place};

    const PERIOD: f64 = 0.125;

    fn call(account: &mut Account, target: f64, arrived: f64, presented: f64) {
        account.begin(target, arrived);
        account.present(presented, 1);
        account.complete(Place::EncodeSubmit, Some((presented, 1)));
    }

    #[test]
    fn serves_one_new_present_in_each_interval() {
        let mut account = Account::default();
        for tick in 1..=3 {
            let target = f64::from(tick) * PERIOD;
            account.callback(target, PERIOD);
            call(&mut account, target, target - 0.10, target - 0.01);
        }
        account.finish();
        assert_eq!((account.opportunities, account.served), (3, 3));
        assert_eq!(account.unserved, [0; 5]);
        assert!(account.measurement.is_none());
    }

    #[test]
    fn a_skipped_callback_has_its_own_opportunity() {
        let mut account = Account::default();
        account.measure(0.0, 1.0);
        account.callback(PERIOD, PERIOD);
        call(&mut account, PERIOD, 0.01, 0.10);
        account.callback(PERIOD * 3.0, PERIOD);
        call(&mut account, PERIOD * 3.0, 0.27, 0.32);
        account.finish();
        assert_eq!((account.opportunities, account.served), (3, 2));
        assert_eq!(
            account.measurement.unwrap().rows[1].unserved(),
            Some(Place::DisplayLink)
        );
    }

    #[test]
    fn an_expired_request_is_placed_by_its_arrival() {
        let mut account = Account::default();
        account.callback(PERIOD, PERIOD);
        call(&mut account, PERIOD, 0.13, 0.15);
        account.callback(2.0 * PERIOD, PERIOD);
        account.finish();
        assert_eq!((account.opportunities, account.served), (2, 1));
        assert_eq!(account.unserved[Place::LateRequest as usize], 1);
    }

    #[test]
    fn a_frame_slot_refusal_can_arrive_after_its_opportunity_closed() {
        let mut account = Account::default();
        account.measure(0.0, 1.0);
        account.callback(PERIOD, PERIOD);
        account.begin(PERIOD, 0.04);
        account.callback(2.0 * PERIOD, PERIOD);
        account.complete(Place::FrameSlot, None);
        account.finish();
        assert_eq!(account.unserved[Place::FrameSlot as usize], 1);
        assert_eq!(
            account.measurement.unwrap().rows[0].unserved(),
            Some(Place::FrameSlot)
        );
    }

    #[test]
    fn encoding_that_passes_the_target_serves_the_interval_of_the_call() {
        let mut account = Account::default();
        account.measure(0.0, 1.0);
        account.callback(PERIOD, PERIOD);
        account.begin(PERIOD, 0.04);
        account.callback(2.0 * PERIOD, PERIOD);
        account.present(0.15, 7);
        account.complete(Place::EncodeSubmit, Some((0.15, 7)));
        account.finish();
        assert_eq!(account.unserved[Place::EncodeSubmit as usize], 1);
        let rows = account.measurement.unwrap().rows;
        assert_eq!(rows[0].requested_present.unwrap().submission, 7);
        assert!(rows[0].interval_present.is_none());
        assert_eq!(rows[1].interval_present.unwrap().submission, 7);
        assert!(rows[1].unserved().is_none());
    }

    #[test]
    fn only_the_measured_interval_keeps_rows() {
        let mut account = Account::default();
        account.measure(2.0, 1.0);
        for tick in 1..=80 {
            account.callback(f64::from(tick) * PERIOD, PERIOD);
        }
        account.finish();
        assert_eq!(account.opportunities, 80);
        let rows = account.measurement.unwrap().rows;
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0].target, 2.0);
        assert_eq!(rows[7].target, 3.0 - PERIOD);
    }

    #[test]
    fn two_presents_in_one_interval_do_not_serve_it() {
        let mut account = Account::default();
        account.callback(PERIOD, PERIOD);
        call(&mut account, PERIOD, 0.01, 0.14);
        account.callback(2.0 * PERIOD, PERIOD);
        call(&mut account, 2.0 * PERIOD, 0.16, 0.20);
        account.finish();
        assert_eq!(account.served, 0);
        assert_eq!(account.unserved[Place::EncodeSubmit as usize], 2);
    }
}
