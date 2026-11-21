//! Untrusted images are rejected atomically through the session restore path.

mod fixture;

use fixture::{IDENTITY, LIMIT, Rules, digest, image, runtime};
use thrum_core::{
    profile::StateDecodeError,
    sim::{GameRuntime, RuleSetIdentity, WorldImageError},
};

#[global_allocator]
static ALLOCATOR: thrum_test_support::CountingAllocator = thrum_test_support::CountingAllocator;

fn refuse(
    world: &mut GameRuntime<Rules>,
    game: &mut Rules,
    bytes: &[u8],
    limit: usize,
) -> (WorldImageError, usize) {
    let before = digest(world, game);
    let (outcome, counted) = thrum_test_support::allocations::count(|| {
        world.restore_world_image(IDENTITY, &mut game.resource, bytes, limit)
    });
    let allocated = usize::try_from(counted.bytes).expect("allocated bytes fit in memory");
    assert!(
        allocated <= limit,
        "restore allocated {allocated} bytes above {limit}"
    );
    let error = outcome.expect_err("untrusted input was accepted");
    assert_eq!(digest(world, game), before, "refusal changed authority");
    (error, allocated)
}

fn set_payload_length(bytes: &mut [u8]) {
    let payload = u64::try_from(bytes.len() - 48).expect("fixture length");
    bytes[40..48].copy_from_slice(&payload.to_le_bytes());
}

fn install_intact(world: &mut GameRuntime<Rules>, game: &mut Rules, valid: &[u8], expected: u64) {
    world
        .restore_world_image(IDENTITY, &mut game.resource, valid, LIMIT)
        .expect("the same destination accepts the unaltered image after the refusals");
    assert_eq!(digest(world, game), expected);
}

#[test]
fn every_truncation_and_trailing_data_leave_authority_intact() {
    let (host, rules) = runtime(1, true);
    let valid = image(&host, &rules);
    let (mut world, mut game) = runtime(2, false);
    for end in 0..valid.len() {
        refuse(&mut world, &mut game, &valid[..end], LIMIT);
        if end >= 48 {
            let mut corrected_length = valid[..end].to_vec();
            set_payload_length(&mut corrected_length);
            refuse(&mut world, &mut game, &corrected_length, LIMIT);
        }
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    refuse(&mut world, &mut game, &trailing, LIMIT);
    set_payload_length(&mut trailing);
    refuse(&mut world, &mut game, &trailing, LIMIT);
    install_intact(&mut world, &mut game, &valid, digest(&host, &rules));
}

#[test]
fn rejects_wrong_identity_and_lengths_without_allocating() {
    let (host, rules) = runtime(1, true);
    let valid = image(&host, &rules);
    let (mut world, mut game) = runtime(2, false);
    assert_eq!(
        refuse(&mut world, &mut game, &valid, valid.len() - 1),
        (WorldImageError::Decode(StateDecodeError::Limit), 0)
    );
    for payload in [0, u64::MAX] {
        let mut malformed = valid.clone();
        malformed[40..48].copy_from_slice(&payload.to_le_bytes());
        assert_eq!(refuse(&mut world, &mut game, &malformed, LIMIT).1, 0);
    }
    let mut other_rules = Vec::new();
    host.write_world_image(RuleSetIdentity([18; 32]), &rules.resource, &mut other_rules)
        .expect("image under another identity");
    assert_eq!(
        refuse(&mut world, &mut game, &other_rules, LIMIT),
        (WorldImageError::RuleSet, 0)
    );

    // The complete input fits, but the staged native storage does not.
    assert_eq!(
        refuse(&mut world, &mut game, &valid, valid.len()).0,
        WorldImageError::Decode(StateDecodeError::Limit)
    );
    install_intact(&mut world, &mut game, &valid, digest(&host, &rules));
}

#[test]
fn random_images_and_mutated_structural_fields_are_refused() {
    let (host, rules) = runtime(1, true);
    let valid = image(&host, &rules);
    let (mut world, mut game) = runtime(2, false);
    let mut random = 0xa51d_c328_u32;
    for size in (0..=LIMIT + 1).step_by(13).chain([LIMIT + 1]) {
        let bytes: Vec<_> = (0..size)
            .map(|_| {
                random ^= random << 13;
                random ^= random >> 17;
                random ^= random << 5;
                random.to_le_bytes()[0]
            })
            .collect();
        refuse(&mut world, &mut game, &bytes, LIMIT);
    }

    let free_length = 64 + 6 * 33;
    let free_entries = free_length + 8;
    let identity_index = free_entries + 4 * 4;
    let directory = identity_index + 6 * 5;
    let first_column = directory + 1 + 2 * (8 + 6 * 8) + 8;
    for offset in [56, free_length, directory + 1, directory + 1 + 8 + 6 * 8] {
        let mut bytes = valid.clone();
        bytes[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        refuse(&mut world, &mut game, &bytes, LIMIT);
    }
    for (offset, value) in [
        (56, 255),                           // capacity
        (64, 2),                             // live flag
        (65, 0),                             // zero generation
        (73 + 2 * 33, 1),                    // identity attached to vacant slot
        (89, 2),                             // structural tick beyond committed tick
        (free_length, 255),                  // oversized free list
        (free_entries, 4),                   // duplicate free slot
        (identity_index + 5 + 1, 3),         // incorrect lookup
        (directory + 1, 7),                  // wrong row map length
        (directory + 9, 0),                  // inconsistent inverse map
        (first_column, 13),                  // unexpected schema
        (first_column + 16, 2),              // future component change tick
        (first_column + 16 + 6 * 8 + 10, 1), // component on dead slot
    ] {
        let mut bytes = valid.clone();
        assert_ne!(bytes[offset], value, "mutation at offset {offset}");
        bytes[offset] = value;
        refuse(&mut world, &mut game, &bytes, LIMIT);
    }
    install_intact(&mut world, &mut game, &valid, digest(&host, &rules));
}
