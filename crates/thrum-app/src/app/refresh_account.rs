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

    fn record_present(&mut self, present: Present) {
        self.multiple |= self.interval_present.is_some();
        if self
            .interval_present
            .is_none_or(|previous| previous.time <= present.time)
        {
            self.interval_present = Some(present);
        }
    }
}

struct Measurement {
    start: f64,
    end: f64,
    rows: Vec<Opportunity>,
}

#[derive(Clone, Copy)]
struct Closed {
    row: Opportunity,
    start: f64,
    index: Option<usize>,
}

struct Request {
    target: f64,
    late: bool,
    closed: Option<Closed>,
}

/// Only the render owner starts, completes and presents a request. The event loop adds
/// targets before dispatching work for them. Presents in known intervals go straight
/// to their opportunity. Only the in-flight frame and one for the latest callback can
/// present beyond the newest known target before another callback is processed.
#[derive(Default)]
pub(super) struct Account {
    current: Option<Opportunity>,
    previous_end: Option<f64>,
    latest_closed: Option<Closed>,
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
            // A half-period phase change is possible with variable refresh.
            // Count a missing slot only near an entire additional period.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a nonnegative elapsed period count, rounded before conversion"
            )]
            let periods = ((target - previous.target) / period + 0.25).floor() as u64;
            for gap in (1..periods).rev() {
                self.close(Opportunity::new(
                    target - gap as f64 * period,
                    Place::DisplayLink,
                ));
            }
        } else {
            self.previous_end = Some(target - period);
        }
        let mut current = Opportunity::new(target, Place::LateRequest);
        self.place_pending(&mut current);
        self.current = Some(current);
    }

    pub(super) fn begin(&mut self, target: f64, arrived: f64) {
        let closed = self
            .latest_closed
            .filter(|closed| closed.row.target.to_bits() == target.to_bits())
            .or_else(|| {
                let measurement = self.measurement.as_ref()?;
                let index = measurement
                    .rows
                    .binary_search_by(|row| row.target.total_cmp(&target))
                    .ok()?;
                Some(Closed {
                    row: measurement.rows[index],
                    start: measurement.start,
                    index: Some(index),
                })
            });
        self.request = Some(Request {
            target,
            late: arrived >= target,
            closed,
        });
    }

    pub(super) fn present(&mut self, time: f64, submission: u64) {
        let present = Present { time, submission };
        if let Some(mut closed) = self.latest_closed
            && (closed.start..closed.row.target).contains(&time)
        {
            // A new callback can be delivered just before the prior target's
            // deadline. A present in that last sliver still belongs to it.
            let before = closed.row.unserved();
            closed.row.record_present(present);
            self.revise(closed, before);
            if let Some(request) = &mut self.request
                && request.target.to_bits() == closed.row.target.to_bits()
            {
                request.closed = Some(closed);
            }
            return;
        }
        if let Some(row) = &mut self.current
            && (self.previous_end.expect("tracked interval start")..row.target).contains(&time)
        {
            row.record_present(present);
            return;
        }
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
            && row.target.to_bits() == request.target.to_bits()
        {
            update(row);
        } else if let Some(mut closed) = request.closed {
            let before = closed.row.unserved();
            update(&mut closed.row);
            self.revise(closed, before);
        }
    }

    fn revise(&mut self, closed: Closed, before: Option<Place>) {
        if let Some(before) = before {
            self.unserved[before as usize] -= 1;
        } else {
            self.served -= 1;
        }
        self.count(closed.row);
        if let (Some(measurement), Some(index)) = (&mut self.measurement, closed.index) {
            measurement.rows[index] = closed.row;
        }
        if self
            .latest_closed
            .is_some_and(|last| last.row.target.to_bits() == closed.row.target.to_bits())
        {
            self.latest_closed = Some(closed);
        }
    }

    fn count(&mut self, row: Opportunity) {
        if let Some(place) = row.unserved() {
            self.unserved[place as usize] += 1;
        } else {
            self.served += 1;
        }
    }

    fn place_pending(&mut self, row: &mut Opportunity) {
        let start = self.previous_end.expect("tracked interval start");
        for slot in &mut self.presents {
            if let Some(present) = *slot
                && present.time < row.target
            {
                if present.time >= start {
                    row.record_present(present);
                }
                *slot = None;
            }
        }
    }

    fn close(&mut self, mut row: Opportunity) {
        let start = self.previous_end.expect("tracked interval start");
        self.place_pending(&mut row);
        self.previous_end = Some(row.target);
        self.opportunities += 1;
        self.count(row);
        let index = self.measurement.as_mut().and_then(|measurement| {
            (measurement.start <= row.target && row.target < measurement.end).then(|| {
                measurement.rows.push(row);
                measurement.rows.len() - 1
            })
        });
        let closed = Closed { row, start, index };
        self.latest_closed = Some(closed);
        if let Some(request) = &mut self.request
            && row.target.to_bits() == request.target.to_bits()
        {
            request.closed = Some(closed);
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
            "target_media_seconds\tpresent_call_media_seconds\tsubmission_id\tinterval_call_media_seconds\tinterval_submission_id\tplace\trequest_place"
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
                "{:.9}\t{}\t{}\t{}\t{}",
                row.target,
                columns(row.requested_present),
                columns(row.interval_present),
                row.unserved().map_or("served", Place::name),
                row.place.name(),
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
    fn an_early_callback_does_not_lose_a_present_in_the_previous_interval() {
        let mut account = Account::default();
        account.measure(0.0, 1.0);
        account.callback(PERIOD, PERIOD);
        account.begin(PERIOD, 0.04);
        account.callback(PERIOD * 2.0, PERIOD);
        account.present(0.12, 3);
        account.complete(Place::EncodeSubmit, Some((0.12, 3)));
        account.finish();
        assert_eq!(account.served, 1);
        assert_eq!(account.unserved[Place::LateRequest as usize], 1);
        let rows = account.measurement.unwrap().rows;
        assert_eq!(rows[0].interval_present.unwrap().submission, 3);
        assert_eq!(rows[0].requested_present.unwrap().submission, 3);
    }

    #[test]
    fn an_old_request_records_its_present_against_the_original_target() {
        let mut account = Account::default();
        account.measure(0.0, 1.0);
        for tick in 1..=4 {
            account.callback(f64::from(tick) * PERIOD, PERIOD);
        }
        call(&mut account, PERIOD, 0.41, 0.42);
        account.finish();
        assert_eq!(account.served, 1);
        let rows = account.measurement.unwrap().rows;
        assert_eq!(rows[0].requested_present.unwrap().submission, 1);
        assert_eq!(rows[0].unserved(), Some(Place::LateRequest));
        assert_eq!(rows[3].interval_present.unwrap().submission, 1);
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

    #[test]
    fn learning_an_interval_releases_pending_space_for_the_next_present() {
        let mut account = Account::default();
        account.measure(0.0, 1.0);
        account.callback(PERIOD, PERIOD);

        account.begin(PERIOD, 0.1);
        account.present(0.27, 1);
        account.complete(Place::EncodeSubmit, Some((0.27, 1)));
        account.callback(2.0 * PERIOD, PERIOD);
        account.begin(2.0 * PERIOD, 0.27);
        account.present(0.28, 2);
        account.complete(Place::EncodeSubmit, Some((0.28, 2)));
        account.callback(3.0 * PERIOD, PERIOD);
        account.begin(3.0 * PERIOD, 0.36);
        account.present(0.39, 3);
        account.complete(Place::EncodeSubmit, Some((0.39, 3)));
        account.callback(4.0 * PERIOD, PERIOD);
        account.finish();

        assert_eq!(account.excess_presents, 0);
        let rows = account.measurement.unwrap().rows;
        assert!(rows[2].multiple);
        assert_eq!(rows[3].interval_present.unwrap().submission, 3);
        assert_eq!(rows[3].unserved(), None);
    }
}
