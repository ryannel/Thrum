"""The paired wind series. The Mac presented-time witness owns display cadence."""
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


def losses(presented, start, end, period):
    """Index nominal refreshes between the OS-presented frames bracketing the run."""
    presented = sorted(presented)
    if not presented or presented[0] > start or presented[-1] < end:
        raise InvalidMeasurement('presented times do not bracket the measured interval')
    anchor = max(time for time in presented if time <= start)

    def boundary(time):
        return int(((time - anchor) / period).to_integral_value(rounding=ROUND_CEILING))

    begin, finish = boundary(start), boundary(end)
    shown = set()
    for time in presented:
        offset = (time - anchor) / period
        index = int(offset.to_integral_value())
        if begin <= index < finish:
            if abs(offset - index) >= Decimal('0.45'):
                raise InvalidMeasurement('presented times cannot be placed on the refresh grid')
            shown.add(index)
    lost = [anchor + period * index for index in range(begin, finish) if index not in shown]
    stalls = []
    for time in lost:
        if stalls and abs(time - stalls[-1][-1] - period) < Decimal('0.001'):
            stalls[-1].append(time)
        else:
            stalls.append([time])
    return lost, stalls


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


def analyze_arm(arm, wind):
    witness = json.loads((arm / 'witness.json').read_text())
    conditions = window_conditions(arm)
    if any(conditions[key] != '0' for key in ('occluded_window_events', 'occluded_surface_attempts')):
        raise InvalidMeasurement(f'{arm.name}: the window was occluded')
    if not witness['coverage'] or not witness['conversion_validated']:
        raise InvalidMeasurement(f'{arm.name}: witness coverage or conversion failed')
    if witness['reported_refresh_hz'] != HZ:
        raise InvalidMeasurement(f'{arm.name}: the game did not report a {HZ} Hz display')
    drawables = witness['drawables']
    presented = [Decimal(frame['presented_media_ns']) for frame in drawables]
    if any(b <= a for a, b in zip(presented, presented[1:])):
        raise InvalidMeasurement(f'{arm.name}: presented times are out of drawable order')
    frames = {int(frame['submission_id']): frame for frame in table(arm / 'cadence-frames.tsv')}
    if not frames or any(frame['timing_requested'] != 'false' for frame in frames.values()):
        raise InvalidMeasurement(f'{arm.name}: timing was enabled or no frame samples were collected')
    start, end = interval(arm)
    result, stalls = measure(presented, start, end, PERIOD)
    rows = account_rows(arm)
    counts = (arm / 'refresh-counts.txt').read_text()
    if 'excess_present_records=0;' not in counts or not rows:
        raise InvalidMeasurement(f'{arm.name}: incomplete refresh account')
    targets = sorted(rows)
    # Walk the two monotonic grids once. The account locates a loss that the
    # presented-time witness already found. It never supplies a shown refresh.
    position = 0
    drawable = 0
    all_places = Counter()
    for record, stall in zip(result['stalls'], stalls, strict=True):
        missing = []
        for timestamp in stall:
            while position + 1 < len(targets) and abs(targets[position + 1] - timestamp) < abs(
                    targets[position] - timestamp):
                position += 1
            target = targets[position]
            if abs(target - timestamp) >= PERIOD / 2:
                raise InvalidMeasurement(f'{arm.name}: missing refresh-account row for {timestamp} ns')
            while drawable + 1 < len(drawables) and presented[drawable + 1] < timestamp:
                drawable += 1
            row = rows[target]
            place = 'after_present_call' if row['place'] == 'served' else row['place']
            if place not in PLACES:
                raise InvalidMeasurement(f'{arm.name}: invalid loss place {place!r}')
            missing.append(dict(
                seconds=float((timestamp - start) / NS), presented_grid_media_ns=str(timestamp),
                place=place, previous_drawable=drawables[drawable]['drawable'],
                next_drawable=drawables[drawable + 1]['drawable'], account=row,
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
            state['direction'] = '+X' if ticks[1] < 600 else '+Y' if ticks[0] >= 600 else 'turn at tick 600'
        else:
            state['direction'] = 'turn at tick 600; one bracketing frame sample missing'
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
