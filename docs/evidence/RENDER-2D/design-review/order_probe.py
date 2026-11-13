"""Independent design arithmetic; no renderer, GPU, or performance qualification."""
import itertools
import json
import math
import random


def near(a, b, band):
    return math.dist(a[1:3], b[1:3]) <= a[3] + b[3] + 2 * band


def settle(order, rows, band):
    order = list(order)
    exchanges = 0
    while True:
        changed = False
        for j in range(len(order) - 1):
            a, b = order[j:j + 2]
            threshold = band if near(rows[a], rows[b], band) else 0
            # All rows in this probe share one layer. Stable identity breaks ties
            # for exact ordering; positive-band near pairs retain their history.
            exchange = (rows[a][0] > rows[b][0] + threshold if threshold else
                        (rows[a][0], a) > (rows[b][0], b))
            if exchange:
                order[j], order[j + 1] = b, a
                exchanges += 1
                changed = True
        if not changed:
            return order, exchanges
        assert exchanges <= len(order) * (len(order) - 1) // 2


def check(before, after, rows, band):
    assert sorted(before) == sorted(after)
    previous = {v: i for i, v in enumerate(before)}
    # Inspect all final pair relations independently of the adjacent-swap update.
    for i, a in enumerate(after):
        for b in after[i + 1:]:
            if previous[a] > previous[b] and near(rows[a], rows[b], band):
                assert rows[b][0] - rows[a][0] > band
    for a, b in zip(after, after[1:]):
        if near(rows[a], rows[b], band) and band > 0:
            assert rows[a][0] <= rows[b][0] + band
        else:
            assert (rows[a][0], a) < (rows[b][0], b)


def main():
    order = [0, 1]
    travelling = []
    for values in [(0, .1), (2, 2.1), (2.2, 2.1), (2.1, 2.3)]:
        rows = {i: (v, 0, 0, 1) for i, v in enumerate(values)}
        updated, _ = settle(order, rows, 1)
        check(order, updated, rows, 1)
        travelling.append(updated)
        order = updated
    assert travelling == [[0, 1]] * 4
    crossing, _ = settle(order, {0: (4, 0, 0, 1), 1: (2.3, 0, 0, 1)}, 1)
    assert crossing == [1, 0]
    chain, _ = settle([0, 1, 2], {0: (1.8, 0, 0, 1),
                                      1: (.9, 0, 0, 1),
                                      2: (0, 0, 0, 1)}, 1)
    assert chain == [0, 1, 2]
    exhaustive = 0
    for values in itertools.product([-2, -1, 0, 1, 2], repeat=3):
        for before in itertools.permutations(range(3)):
            rows = {i: (v, 0, 0, 1) for i, v in enumerate(values)}
            after, _ = settle(before, rows, 1)
            check(before, after, rows, 1)
            exhaustive += 1
    rng = random.Random(20261113)
    order = list(range(24))
    exchanges = 0
    for tick in range(2000):
        rows = {i: (rng.uniform(-8, 8), rng.uniform(-4, 4),
                    rng.uniform(-4, 4), rng.uniform(.1, 1)) for i in order}
        after, count = settle(order, rows, 1)
        check(order, after, rows, 1)
        exact, _ = settle(order, rows, 0)
        assert exact == sorted(rows, key=lambda i: (rows[i][0], i))
        order = after
        exchanges += count
    print(json.dumps(dict(travelling_pair=travelling, passing_pair=crossing,
                          chain=chain, exhaustive_three_row_states=exhaustive,
                          random_ticks=2000, random_rows=24,
                          random_exchanges=exchanges, result="PASS"), indent=2))


if __name__ == "__main__":
    main()
