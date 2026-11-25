"""The paired wind series. The Mac presented-time witness owns display cadence."""
from bisect import bisect_left, bisect_right
from collections import Counter
from decimal import Decimal, ROUND_CEILING
import json

from mac_m2_presented import NS, table

HZ = 120
SECONDS = 20
PERIOD = NS / HZ
PLACES = {
    'display_link', 'late_request', 'frame_slot', 'drawable', 'encode_submit',
    'after_present_call',
}
ABSENT = ('off', 0, 1, False)
WINDS = (
    ('still', 0, 1, False),
    ('10ms-1x', 10, 1, False),
    ('40ms-1x', 40, 1, False),
    ('40ms-60x', 40, 60, False),
    ('40ms-turn', 40, 1, True),
)
ARMS = (ABSENT, *(arm for wind in WINDS for arm in (wind, ABSENT)))


class InvalidMeasurement(ValueError):
    """The witness cannot decide a run's cadence."""


class OccludedMeasurement(InvalidMeasurement):
    """The window or its surface reported occlusion during a run."""


class IncompleteWitness(InvalidMeasurement):
    """An unmatched observation could affect the measured interval."""


def losses(presented, start, end, period):
    """A stall is the unfilled refresh periods between two OS-presented frames."""
    if not presented or presented[0] > start or presented[-1] < end:
        raise InvalidMeasurement('presented times do not bracket the measured interval')
    stalls = []
    for left, right in zip(presented, presented[1:]):
        if right <= left:
            raise InvalidMeasurement('presented times are out of drawable order')
        if right < start or left >= end:
            continue
        # Use the same interval rounding as the platform witness. Re-anchor at
        # each presented frame so a phase change does not shift the rest of the run.
        periods = int(((right - left) / period).to_integral_value())
        first = max(1, int(((start - left) / period).to_integral_value(rounding=ROUND_CEILING)))
        last = min(periods, int(((end - left) / period).to_integral_value(rounding=ROUND_CEILING)))
        if first < last:
            stalls.append([left + period * index for index in range(first, last)])
    return [time for stall in stalls for time in stall], stalls


def measure(presented, start, end, period):
    lost, stalls = losses(presented, start, end, period)
    return {
        'lost': len(lost),
        'longest_stall': max(map(len, stalls), default=0),
        'stalls': [{'seconds': float((stall[0] - start) / NS), 'length': len(stall)}
                   for stall in stalls],
    }, stalls


def hold(cloud, before, after):
    """Apply the same count and stall rule to scripted and recorded runs."""
    neighbours = [before, after]
    count_limit = max(run['lost'] for run in neighbours) + 1
    stall_limit = max(run['longest_stall'] for run in neighbours) + 1
    stands_out = [stall for stall in cloud['stalls'] if stall['length'] > stall_limit]
    return {
        'holds': cloud['lost'] <= count_limit and not stands_out,
        'count_holds': cloud['lost'] <= count_limit,
        'stall_holds': not stands_out,
        'neighbours_lost': [run['lost'] for run in neighbours],
        'neighbours_longest_stalls': [run['longest_stall'] for run in neighbours],
        'stands_out': stands_out,
    }


def scripted_hold():
    """The five WP19.4 cases, including a partial 20-second witness."""
    def run(missing=(), stop=HZ * SECONDS + 1):
        times = [PERIOD * i for i in range(-1, stop) if i not in missing]
        return measure(times, Decimal(0), NS * SECONDS, PERIOD)[0]

    clear = run()
    one = run([30])
    result = hold(run([30, 31]), clear, one)
    assert result['holds'] and result['count_holds'] and result['stall_holds'], result
    result = hold(run([30, 60, 90]), clear, one)
    assert not result['holds'] and not result['count_holds'] and result['stall_holds'], result
    singles = run(range(60, 120, 2))
    result = hold(run(range(120, 150)), singles, singles)
    assert not result['holds'] and result['count_holds'] and not result['stall_holds'], result
    assert result['stands_out'] == [{'seconds': 1.0, 'length': 30}], result
    result = hold(run(range(0, 100, 2)), run(range(0, 80, 2)), run(range(0, 110, 2)))
    assert result['holds'] and result['neighbours_lost'] == [40, 55], result
    try:
        run(stop=HZ * SECONDS - 1)
    except InvalidMeasurement:
        pass
    else:
        raise AssertionError('the short witness received a measurement')
    print('All five scripted wind cadence cases passed.')


def interval(arm):
    (row,) = table(arm / 'refresh-interval.tsv')
    start = Decimal(row['start_media_seconds']) * NS
    end = Decimal(row['end_media_seconds']) * NS
    if end - start != SECONDS * NS:
        raise InvalidMeasurement(f'{arm.name}: the window did not record exactly {SECONDS} seconds')
    return start, end


def account_rows(arm):
    return {Decimal(row['target_media_seconds']) * NS: row
            for row in table(arm / 'refresh-account.tsv')}


def window_conditions(arm):
    raw = (arm / 'refresh-window.txt').read_text().splitlines()
    return dict(line.split('=', 1) for line in raw)


def require_complete_observations(witness, start, end):
    frames = witness['drawables']
    ordinals = [frame['drawable'] for frame in frames]
    missing = {row['drawable'] for row in witness['unmatched']}
    missing.update(left + 1 for left, right in zip(ordinals, ordinals[1:]) if right > left + 1)
    for ordinal in sorted(missing):
        index = bisect_left(ordinals, ordinal)
        if index < len(frames) and Decimal(frames[index]['presented_media_ns']) <= start:
            continue
        if index and Decimal(frames[index - 1]['presented_media_ns']) >= end:
            continue
        raise IncompleteWitness(f'unmatched drawable {ordinal} can affect the measured interval')


def present_calls(rows):
    """Index both views of a present call by the game submission ID."""
    calls = {}
    for submission, timestamp in (
        ('submission_id', 'present_call_media_seconds'),
        ('interval_submission_id', 'interval_call_media_seconds'),
    ):
        for row in rows.values():
            if row[submission]:
                calls.setdefault(int(row[submission]), (row[timestamp], row))
    return calls


def before_present_call(arm, timestamp, following, call_row, rows, targets, dispatches):
    # A request that was dispatched before the miss owns its recorded outcome.
    # Its display target can be several refreshes behind the current deadline.
    dispatched = Decimal(following['opportunity_media_seconds']) * NS
    if (dispatched <= timestamp and call_row is not None
            and call_row['submission_id'] == str(following['drawable'])):
        place = call_row.get('request_place')
        if place is None and not call_row['interval_submission_id']:
            # Earlier records report the request outcome when the interval had no call.
            place = call_row['place']
        if place in PLACES and place != 'display_link':
            return place, call_row, 'drawable_request'

    # With no joined request outcome, only an already elapsed opportunity can
    # explain the miss. Its refusal must precede the miss as well.
    index = bisect_right(targets, timestamp) - 1
    if index >= 0 and timestamp - targets[index] < PERIOD * Decimal('1.5'):
        target = targets[index]
        row = rows[target]
        place = row['place']
        if place in ('display_link', 'late_request'):
            return place, row, 'unserved_opportunity'
        if (place in PLACES and not row['submission_id']
                and target in dispatches and dispatches[target] <= timestamp):
            return place, row, 'unserved_opportunity'
    raise InvalidMeasurement(f'{arm.name}: no causal refresh-account row for {timestamp} ns')


def require_unoccluded(arm, conditions):
    window_events = conditions['occluded_window_events']
    surface_attempts = conditions['occluded_surface_attempts']
    if window_events != '0' or surface_attempts != '0':
        raise OccludedMeasurement(
            f'{arm.name}: the window was occluded ({window_events} window events, '
            f'{surface_attempts} surface attempts)'
        )


def analyze_arm(arm, wind):
    witness = json.loads((arm / 'witness.json').read_text())
    conditions = window_conditions(arm)
    require_unoccluded(arm, conditions)
    if not witness['coverage'] or not witness['conversion_validated']:
        raise InvalidMeasurement(f'{arm.name}: witness coverage or conversion failed')
    if witness['reported_refresh_hz'] != HZ:
        raise InvalidMeasurement(f'{arm.name}: the game did not report a {HZ} Hz display')
    start, end = interval(arm)
    require_complete_observations(witness, start, end)
    first_drawable = []
    last_drawable = []
    presented = []
    for frame in witness['drawables']:
        timestamp = Decimal(frame['presented_media_ns'])
        if presented and timestamp < presented[-1]:
            raise InvalidMeasurement(f'{arm.name}: presented times are out of drawable order')
        if presented and timestamp == presented[-1]:
            # The OS can report two submissions at the same refresh. The
            # witness keeps both; only the latest can bracket the next loss.
            last_drawable[-1] = frame
        else:
            first_drawable.append(frame)
            last_drawable.append(frame)
            presented.append(timestamp)
    frames = {int(frame['submission_id']): frame for frame in table(arm / 'cadence-frames.tsv')}
    if not frames or any(frame['timing_requested'] != 'false' for frame in frames.values()):
        raise InvalidMeasurement(f'{arm.name}: timing was enabled or no frame samples were collected')
    result, stalls = measure(presented, start, end, PERIOD)
    rows = account_rows(arm)
    counts = (arm / 'refresh-counts.txt').read_text()
    if 'excess_present_records=0;' not in counts or not rows:
        raise InvalidMeasurement(f'{arm.name}: incomplete refresh account')
    calls = present_calls(rows)
    targets = sorted(rows)
    dispatches = {}
    for request in table(arm / 'requests.tsv'):
        target = Decimal(request['target_media_seconds']) * NS
        dispatched = Decimal(request['media_seconds']) * NS
        dispatches[target] = max(dispatched, dispatches.get(target, dispatched))
    drawable = 0
    all_places = Counter()
    for record, stall in zip(result['stalls'], stalls, strict=True):
        missing = []
        for timestamp in stall:
            while drawable + 1 < len(presented) and presented[drawable + 1] < timestamp:
                drawable += 1
            following = first_drawable[drawable + 1]
            # The witness's surface sample is stamped after queue.present.
            # The account has the exact call when it falls in a retained row.
            call, call_row = calls.get(following['drawable'],
                                       (following['present_call_media_seconds'], None))
            if Decimal(call) * NS <= timestamp:
                place = 'after_present_call'
                row = call_row
                opportunity = None
                account_source = 'present_call'
            else:
                if call_row is None and Decimal(following['opportunity_media_seconds']) * NS <= timestamp:
                    raise InvalidMeasurement(f'{arm.name}: present call for drawable {following["drawable"]}'
                                             f' may straddle {timestamp} ns')
                place, row, account_source = before_present_call(
                    arm, timestamp, following, call_row, rows, targets, dispatches)
                opportunity = row['target_media_seconds']
            missing.append(dict(
                seconds=float((timestamp - start) / NS), presented_grid_media_ns=str(timestamp),
                place=place, previous_drawable=last_drawable[drawable]['drawable'],
                next_drawable=following['drawable'], next_present_call_media_seconds=call,
                present_call_source='window_account' if call_row is not None else 'surface_sample_after_call',
                opportunity_target_media_seconds=opportunity, account_source=account_source, account=row,
            ))
        record['refreshes'] = missing
        record['places'] = dict(Counter(row['place'] for row in missing))
        record['wind'] = wind_at(wind, frames, missing)
        all_places.update(record['places'])
    result.update(places=dict(all_places), conditions=conditions, unmatched=len(witness['unmatched']),
                  witness='witness.json', account='refresh-account.tsv')
    return result


def wind_at(wind, frames, missing):
    name, speed, scale, turns = wind
    state = {'configuration': name, 'metres_per_second': speed, 'time_scale': scale}
    if turns:
        left = frames.get(missing[0]['previous_drawable'])
        right = frames.get(missing[-1]['next_drawable'])
        if left and right:
            # The game selects the direction from the committed weather tick at 60 Hz.
            ticks = [int(left['tick']), int(right['tick'])]
            state['bracketing_ticks'] = ticks
            state['target_direction'] = (
                '+X' if ticks[1] < 600 else '+Y' if ticks[0] >= 600 else 'changes at tick 600'
            )
        else:
            state['target_direction'] = 'turn at tick 600; one bracketing frame sample missing'
    return state


def differences(cloud, before, after):
    verdict = hold(cloud, before, after)
    outside = Counter(cloud['places'])
    for stall in verdict['stands_out']:
        outside.subtract(stall['places'])
    entries = [
        {'kind': 'stall', 'excess': True, **stall}
        for stall in verdict['stands_out']
    ]
    for place, count in sorted(outside.items()):
        if count:
            neighbours = [run['places'].get(place, 0) for run in (before, after)]
            entries.append({'kind': 'place', 'place': place, 'lost': count,
                            'neighbours_lost': neighbours, 'excess': count > max(neighbours)})
    return verdict, entries
