"""Summarize the retained current-build runtime and real-window records."""
import argparse
import csv
import gzip
import json
from pathlib import Path


def load(path):
    if not path.exists():
        path = path.with_suffix(path.suffix + '.gz')
    opener = gzip.open if path.suffix == '.gz' else open
    with opener(path, 'rt') as stream:
        return json.load(stream)


def table(path):
    if not path.exists():
        path = path.with_suffix(path.suffix + '.gz')
    opener = gzip.open if path.suffix == '.gz' else open
    with opener(path, 'rt') as stream:
        return list(csv.DictReader(stream, delimiter='\t'))


def motion(directory):
    manifest = load(directory / 'manifest.json')
    runtime = load(directory / 'runtime.json')
    records = sorted(manifest['records'], key=lambda row: row['submit_ns'])
    opportunities = {row['submission_id']: row for row in runtime['opportunities'] if row['submitted']}
    first_pan = next(row['submit_ns'] for row in records
                     if opportunities[row['submission_id']]['accepted_yaw_degrees'] > 0)
    first_final = next(row['submit_ns'] for row in records
                       if opportunities[row['submission_id']]['accepted_yaw_degrees'] >= 35.999)
    gaps = [{'from': a['submission_id'], 'to': b['submission_id'],
             'start_ns': a['submit_ns'], 'end_ns': b['submit_ns'],
             'gap_ms': (b['submit_ns'] - a['submit_ns']) / 1e6,
             'yaw': opportunities[a['submission_id']]['accepted_yaw_degrees']}
            for a, b in zip(records, records[1:])]
    repeats = [gap for a, b, gap in zip(records, records[1:], gaps)
               if a['rgba_sha256'] == b['rgba_sha256']
               and 0 < opportunities[a['submission_id']]['accepted_yaw_degrees'] < 35.999]
    return {'binary_sha256': manifest['binary_sha256'],
            'revision': manifest['source_revision'],
            'build_source_digest': manifest['build_source_digest'],
            'resolved_recipe_digest': manifest['resolved_recipe_digest'],
            'complete': manifest['complete'], 'originals': len(records),
            'initial_hold_seconds': first_pan / 1e9,
            'first_final_yaw_seconds': first_final / 1e9,
            'interval_end_seconds': runtime['interval_end_ns'] / 1e9,
            'final_hold_seconds': (runtime['interval_end_ns'] - first_final) / 1e9,
            'maximum_image_gap_ms': max(row['gap_ms'] for row in gaps),
            'gaps_over_two_requested_periods': [row for row in gaps if row['gap_ms'] > 1000 / 60],
            'exact_repeats_inside_sweep': repeats,
            'skipped_requested_opportunities': sum(row.get('skipped_before', 0) for row in runtime['opportunities']),
            'ring_omissions': runtime['ring_omissions'], 'writer_omissions': runtime['writer_omissions'],
            'ticks': sorted({row['tick'] for row in runtime['images']}),
            'desktop_capture_independence_qualified': manifest['desktop_capture_independence_qualified'],
            'console_before': manifest['console_before'], 'console_after': manifest['console_after'],
            'preview_pts_max_error_seconds': manifest['preview_pts_max_error_seconds']}


def window(directory):
    result = {}
    for name in ('grid-replacement', 'valley-replacement', 'off-probe'):
        arm = directory / name
        surface = table(arm / 'surface.tsv')
        images = table(arm / 'images.tsv')
        row = {'presentations': len(surface), 'images': len(images),
               'all_presented': all(item['presented'] == 'true' for item in surface),
               'tap_enabled': sorted({item['tap_enabled'] for item in surface}),
               'lease_generations': sorted({int(item['lease_generation']) for item in surface}),
               'presenter_generations': sorted({int(item['presenter_generation']) for item in images}),
               'retirements': table(arm / 'retirements.tsv')}
        if name != 'off-probe':
            manifest = load(arm / 'window-manifest.json')
            row.update(missing_submissions=manifest['missing_submissions'],
                       pattern_rgba_sha256=manifest['pattern_rgba_sha256'],
                       stale_recycle=(arm / 'stale-recycle.txt').read_text().strip())
            records = manifest['records']
            transition = next(index for index in range(1, len(records))
                              if records[index]['presenter_generation'] != records[index - 1]['presenter_generation'])
            before, after = records[transition - 1:transition + 1]
            row['replacement_boundary'] = {'before': before, 'after': after,
                                           'identical_rgba': before['rgba_sha256'] == after['rgba_sha256'],
                                           'gap_ms': (after['submit_ns'] - before['submit_ns']) / 1e6}
        result[name] = row
    return result


def window_motion(directory):
    manifest = load(directory / 'window-manifest.json')
    records = manifest['records']
    markers = table(directory / 'signposts.tsv')
    anchor = table(directory / 'clock-anchor.tsv')[0]
    media_origin = (float(anchor['media_before']) + float(anchor['media_after'])) / 2
    media_origin -= int(anchor['origin_elapsed_ns']) / 1e9
    original_images = {int(row['submission_id']): row for row in table(directory / 'images.tsv')}
    first_pan = next(row for row in records if row['accepted_yaw_degrees'] > 0)
    first_final = next(row for row in records if row['accepted_yaw_degrees'] >= 35.999)
    def media(row):
        return media_origin + int(original_images[row['submission_id']]['submit_origin_ns']) / 1e9
    gaps = [{'from': a['submission_id'], 'to': b['submission_id'],
             'start_ns': a['submit_ns'], 'end_ns': b['submit_ns'],
             'gap_ms': (b['submit_ns'] - a['submit_ns']) / 1e6,
             'yaw': a['accepted_yaw_degrees']}
            for a, b in zip(records, records[1:])]
    repeats = [gap for a, b, gap in zip(records, records[1:], gaps)
               if a['rgba_sha256'] == b['rgba_sha256'] and 0 < a['accepted_yaw_degrees'] < 35.999]
    return {'originals': len(records), 'continuous': manifest['continuous'],
            'missing_submissions': manifest['missing_submissions'],
            'initial_hold_seconds': media(first_pan) - float(markers[0]['media_before']),
            'pan_marker_seconds': float(markers[2]['media_before']) - float(markers[1]['media_before']),
            'final_hold_seconds': float(markers[-1]['media_after']) - media(first_final),
            'first_final_accepted_yaw': first_final['accepted_yaw_degrees'],
            'maximum_image_gap_ms': max(row['gap_ms'] for row in gaps),
            'gaps_over_two_requested_periods': [row for row in gaps if row['gap_ms'] > 1000 / 60],
            'exact_repeats_inside_sweep': repeats,
            'yaw_monotonic': all(a['accepted_yaw_degrees'] <= b['accepted_yaw_degrees']
                                 for a, b in zip(records, records[1:])),
            'scope': 'host-time submitted engine images; media conversion uses the retained same-process clock bracket'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--motion', required=True, type=Path)
    parser.add_argument('--window', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--presented', type=Path)
    args = parser.parse_args()
    result = {'motion': motion(args.motion), 'window': window(args.window)}
    if args.presented:
        result['matched_on_motion'] = {name: window_motion(args.presented / name)
                                       for name in ('on-untraced', 'on-traced')}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
