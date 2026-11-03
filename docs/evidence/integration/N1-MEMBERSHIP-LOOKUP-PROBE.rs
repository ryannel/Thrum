use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::time::Instant;

const PEERS: usize = 4;
const INTEREST: usize = 128;
const WORLD: usize = 256;

fn id(index: usize) -> u64 {
    index as u64 * 65_537 + 17
}

fn main() {
    let mut args = std::env::args().skip(1);
    let method = args.next().unwrap();
    let changed_count: usize = args.next().unwrap().parse().unwrap();
    let repeats = if changed_count == 8 { 100_000 } else { 20_000 };

    let rows: Vec<Vec<(u64, u8)>> = (0..PEERS)
        .map(|peer| {
            let mut rows: Vec<_> = (0..INTEREST)
                .map(|index| {
                    let world = (index + peer * 32) % WORLD;
                    (id(world), (index / 32) as u8)
                })
                .collect();
            rows.sort_unstable_by_key(|row| row.0);
            rows
        })
        .collect();
    let hashes: Vec<HashMap<u64, u8>> = rows
        .iter()
        .map(|rows| rows.iter().copied().collect())
        .collect();
    let trees: Vec<BTreeMap<u64, u8>> = rows
        .iter()
        .map(|rows| rows.iter().copied().collect())
        .collect();
    let mut changed: Vec<_> = (0..changed_count)
        .map(|index| id((index * 37 + 73) % WORLD))
        .collect();
    changed.sort_unstable();

    let start = Instant::now();
    let mut checksum = 0_u64;
    for _ in 0..repeats {
        let changes = black_box(&changed);
        for peer in 0..PEERS {
            match method.as_str() {
                "binary" => {
                    let rows = black_box(&rows[peer]);
                    for &key in changes {
                        if let Ok(index) = rows.binary_search_by_key(&key, |row| row.0) {
                            checksum = checksum.wrapping_add(rows[index].1 as u64 + key);
                        }
                    }
                }
                "merge" => {
                    let rows = black_box(&rows[peer]);
                    let mut cursor = 0;
                    for &key in changes {
                        while cursor < rows.len() && rows[cursor].0 < key {
                            cursor += 1;
                        }
                        if cursor < rows.len() && rows[cursor].0 == key {
                            checksum = checksum.wrapping_add(rows[cursor].1 as u64 + key);
                        }
                    }
                }
                "hash" => {
                    let map = black_box(&hashes[peer]);
                    for &key in changes {
                        if let Some(&partition) = map.get(&key) {
                            checksum = checksum.wrapping_add(partition as u64 + key);
                        }
                    }
                }
                "tree" => {
                    let map = black_box(&trees[peer]);
                    for &key in changes {
                        if let Some(&partition) = map.get(&key) {
                            checksum = checksum.wrapping_add(partition as u64 + key);
                        }
                    }
                }
                _ => panic!("unknown method"),
            }
        }
    }
    let nanos = start.elapsed().as_nanos() as f64 / repeats as f64;
    println!("{method},{changed_count},{nanos:.1},{checksum}");
}
