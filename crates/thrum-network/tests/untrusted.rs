//! Generated peer bytes go through the production session decoders on conditioned links.

/// Shared independent world and virtual time.
use crate::support;

use support::{Codec, Match, local};
use thrum_core::{ActorId, Origin, PeerId, sim::InputFrame};
use thrum_network::{
    envelope::{self, HEADER_BYTES, Header, Kind, MAX_MESSAGE_BYTES},
    input_batch::{self, EncodeRecord},
    session::{ClientState, Controls, Disposition, Input, Refusal, ResultEvent, Settings, World},
};
use thrum_transport::{CloseReason, Lane};

fn header(kind: Kind, generation: u64, tick: u64, sequence: u64) -> Header {
    let named = !matches!(kind, Kind::Hello | Kind::Reject);
    Header {
        kind,
        match_id: if named { [4; 16] } else { [0; 16] },
        connection_epoch: if named { 2 } else { 0 },
        acknowledged_sequence: generation,
        server_tick: tick,
        message_sequence: sequence,
    }
}

fn packet(header: Header, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; HEADER_BYTES + payload.len()];
    let size = envelope::encode(header, payload, &mut bytes).expect("bounded fixture bytes");
    assert_eq!(size, bytes.len());
    bytes
}

fn neutral(_: u64) -> Controls<u16> {
    Controls::axes([0.0; 3])
}

fn startup() -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut game = Match::new(local(), 1, None, false);
    game.memory.advance(0);
    game.clients[0]
        .advance(0, neutral)
        .expect("encodable controls");
    game.memory.advance(0);
    let hello = game.links[0][1]
        .receive(Lane::Reliable)
        .expect("fixture hello");
    game.links[0][0]
        .send(Lane::Reliable, &hello, 0)
        .expect("bounded fixture bytes");
    game.memory.advance(0);
    game.host.advance(0, 0);
    game.memory.advance(0);
    let output: Vec<_> = std::iter::from_fn(|| game.links[0][0].receive(Lane::Reliable)).collect();
    assert_eq!(
        output
            .iter()
            .map(|bytes| envelope::decode(bytes)
                .expect("bounded fixture bytes")
                .header
                .kind)
            .collect::<Vec<_>>(),
        [Kind::Welcome, Kind::BaselineChunk, Kind::BaselineEnd]
    );
    (hello, output)
}

fn begin_client(game: &mut Match) {
    game.memory.advance(0);
    game.clients[0]
        .advance(0, neutral)
        .expect("encodable controls");
    game.memory.advance(0);
    assert!(game.links[0][1].receive(Lane::Reliable).is_some());
}

fn to_client(game: &mut Match, lane: Lane, bytes: &[u8]) {
    game.links[0][1]
        .send(lane, bytes, 0)
        .expect("bounded fixture bytes");
    game.memory.advance(0);
    game.clients[0]
        .advance(0, neutral)
        .expect("encodable controls");
}

fn to_host(game: &mut Match, lane: Lane, bytes: &[u8], time: u64, through: u64) {
    game.links[0][0]
        .send(lane, bytes, time / 1_000)
        .expect("bounded fixture bytes");
    game.memory.advance(time / 1_000);
    game.host.advance(time, through);
}

fn command() -> Vec<u8> {
    let mut body = Vec::from(7_u32.to_le_bytes());
    body.extend(2_u16.to_le_bytes());
    body.extend(19_u16.to_le_bytes());
    packet(header(Kind::Command, 0, 0, 1), &body)
}

fn sets() -> Vec<u8> {
    let mut body = vec![0; 10];
    // At interval 1 the first set includes a digest. This fixture checks
    // acceptance before enough ordered sets are available to step.
    body.extend(13_u16.to_le_bytes());
    body.extend([16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    packet(header(Kind::AdmittedSets, 1, 1, 1), &body)
}

fn append_command_set(body: &mut Vec<u8>, sequence: u8) {
    let mut set = vec![16];
    set.extend(0_u64.to_le_bytes());
    set.extend([65, sequence, 1]);
    set.extend(7_u32.to_le_bytes());
    set.extend(2_u16.to_le_bytes());
    set.extend(19_u16.to_le_bytes());
    set.extend([0; 3]);
    body.extend(u16::try_from(set.len()).expect("one set").to_le_bytes());
    body.extend(set);
}

fn report(first: u64, count: usize, actor: u64) -> Vec<u8> {
    let frames: Vec<InputFrame<u16>> = (first..)
        .take(count)
        .map(|sequence| {
            InputFrame::new(
                Origin {
                    peer: PeerId(0),
                    connection_epoch: 2,
                    sequence,
                },
                ActorId(actor - 1),
                10 + sequence,
                [1.0, 0.0, 0.0],
            )
        })
        .collect();
    let records: Vec<_> = frames
        .iter()
        .map(|frame| EncodeRecord {
            frame,
            actor_wire_id: actor,
        })
        .collect();
    let mut body = vec![0; 16 + 800];
    let size =
        input_batch::encode(&records, &Codec, &mut body[16..]).expect("bounded fixture bytes");
    body.truncate(16 + size);
    packet(
        header(Kind::InputBatch, 1, 0, first + count as u64 - 1),
        &body,
    )
}

fn divergence() -> Vec<u8> {
    let mut body = vec![1];
    body.extend(365_u64.to_le_bytes());
    body.extend(17_u128.to_le_bytes());
    body.extend(29_u64.to_le_bytes());
    packet(header(Kind::Divergence, 1, 0, 0), &body)
}

#[derive(Clone, Copy)]
enum Recipient {
    Negotiating,
    Image,
    ImageEnd,
    Replica,
    Lobby,
    Authority,
}

struct Case {
    recipient: Recipient,
    lane: Lane,
    message: Vec<u8>,
}

impl Case {
    fn deliver(&self, bytes: &[u8], baseline: &[Vec<u8>], expect_refusal: bool) {
        let mut game = Match::new(local(), 1, None, false);
        let client_side = match self.recipient {
            Recipient::Lobby => false,
            Recipient::Authority => {
                game.ready();
                assert!(game.host.ready(0));
                false
            }
            recipient => {
                begin_client(&mut game);
                let prefix = match recipient {
                    Recipient::Negotiating => 0,
                    Recipient::Image => 1,
                    Recipient::ImageEnd => 2,
                    Recipient::Replica => 3,
                    _ => unreachable!(),
                };
                for original in &baseline[..prefix] {
                    to_client(&mut game, Lane::Reliable, original);
                }
                true
            }
        };
        let digest = game.host.world().digest();
        if client_side {
            let before = game.clients[0].world().digest();
            let progress = (game.clients[0].held(), game.clients[0].newest());
            to_client(&mut game, self.lane, bytes);
            if expect_refusal {
                assert_eq!(game.clients[0].world().digest(), before);
                assert_eq!((game.clients[0].held(), game.clients[0].newest()), progress);
                assert_eq!(
                    game.clients[0].state(),
                    &ClientState::Closed(CloseReason::Protocol)
                );
            } else {
                assert!(!matches!(game.clients[0].state(), ClientState::Closed(_)));
                match envelope::decode(&self.message)
                    .expect("bounded fixture bytes")
                    .header
                    .kind
                {
                    Kind::Welcome | Kind::BaselineBegin => {
                        assert!(game.clients[0].baseline_progress().is_some());
                    }
                    Kind::MatchEnded => {
                        assert!(matches!(game.clients[0].state(), ClientState::Ended(_, 0)));
                    }
                    Kind::Reject => assert_eq!(
                        game.clients[0].state(),
                        &ClientState::Refused(Refusal::Full)
                    ),
                    Kind::BaselineChunk => {
                        let (held, total) = game.clients[0]
                            .baseline_progress()
                            .expect("offered baseline");
                        assert_eq!(
                            held,
                            envelope::decode(bytes).expect("framed chunk").payload.len()
                        );
                        assert_eq!(
                            total,
                            envelope::decode(&self.message)
                                .expect("original chunk")
                                .payload
                                .len()
                        );
                    }
                    Kind::BaselineEnd => assert!(game.clients[0].baseline_progress().is_none()),
                    Kind::AdmittedSets => {
                        let size = envelope::decode(bytes).expect("framed sets").payload.len();
                        assert_eq!(game.clients[0].held(), u64::from(size > 10));
                    }
                    _ => unreachable!(),
                }
            }
        } else {
            let now = match self.recipient {
                Recipient::Lobby => {
                    game.memory.advance(0);
                    0
                }
                _ => game.now,
            };
            to_host(&mut game, self.lane, bytes, now, 0);
            assert_eq!(game.host.world().digest(), digest);
            let goodbye = envelope::decode(&self.message)
                .expect("original message")
                .header
                .kind
                == Kind::Close;
            assert_eq!(game.links[0][1].is_open(), !expect_refusal && !goodbye);
            if !expect_refusal {
                match envelope::decode(&self.message)
                    .expect("bounded fixture bytes")
                    .header
                    .kind
                {
                    Kind::Hello => assert!(game.links[0][1].counters(0).lanes[0].sent >= 3),
                    Kind::Divergence => {
                        let report = game.host.take_divergence().expect("accepted divergence");
                        assert_eq!(report.actual, Some(365));
                        assert_eq!(report.tick, 0);
                    }
                    Kind::Close => assert_eq!(
                        game.links[0][1].event(),
                        Some(thrum_transport::Event::Closed(CloseReason::Local(2)))
                    ),
                    Kind::InputBatch => {
                        let bytes = envelope::decode(bytes).expect("framed input").payload;
                        let staged = game
                            .host
                            .stream(game.links[0][1].id())
                            .expect("welcomed peer")
                            .staged_frames;
                        assert_eq!(staged, if bytes.len() == 16 { 0 } else { 2 });
                    }
                    _ => assert!(game.host.stream(game.links[0][1].id()).is_some()),
                }
            }
        }
    }
}

#[test]
fn every_session_decoder_handles_full_frames_all_cuts_bad_lengths_and_random_bytes() {
    let (hello, images) = startup();
    let reliable = Lane::Reliable;
    let cases = [
        Case {
            recipient: Recipient::Lobby,
            lane: reliable,
            message: hello,
        },
        Case {
            recipient: Recipient::Negotiating,
            lane: reliable,
            message: images[0].clone(),
        },
        Case {
            recipient: Recipient::Negotiating,
            lane: reliable,
            message: packet(header(Kind::Reject, 0, 0, 0), &[3]),
        },
        Case {
            recipient: Recipient::Replica,
            lane: reliable,
            message: packet(
                header(Kind::BaselineBegin, 2, 0, 0),
                envelope::decode(&images[0])
                    .expect("offered baseline")
                    .payload,
            ),
        },
        Case {
            recipient: Recipient::Replica,
            lane: reliable,
            message: packet(header(Kind::MatchEnded, 0, 0, 0), &[2]),
        },
        Case {
            recipient: Recipient::Authority,
            lane: reliable,
            message: packet(header(Kind::Close, 0, 0, 0), &[]),
        },
        Case {
            recipient: Recipient::Image,
            lane: reliable,
            message: images[1].clone(),
        },
        Case {
            recipient: Recipient::ImageEnd,
            lane: reliable,
            message: images[2].clone(),
        },
        Case {
            recipient: Recipient::Replica,
            lane: reliable,
            message: sets(),
        },
        Case {
            recipient: Recipient::Replica,
            lane: Lane::Datagram,
            message: sets(),
        },
        Case {
            recipient: Recipient::Authority,
            lane: reliable,
            message: command(),
        },
        Case {
            recipient: Recipient::Authority,
            lane: reliable,
            message: divergence(),
        },
        Case {
            recipient: Recipient::Authority,
            lane: Lane::Datagram,
            message: report(1, 2, 1),
        },
    ];
    assert!(envelope::decode(&[]).is_err());
    let mut random = 0x581a_c38d_2e01_7e2b_u64;
    let mut envelopes = 0;
    let mut payloads = 0;
    for case in &cases {
        case.deliver(&case.message, &images, false);
        for size in 1..case.message.len() {
            case.deliver(&case.message[..size], &images, true);
            envelopes += 1;
        }
        let message = envelope::decode(&case.message).expect("bounded fixture bytes");
        // Each prefix has its own correct envelope length. A shorter chunk,
        // progress-only packet, or report without optional parts can be valid.
        for size in 0..message.payload.len() {
            let accepts = match message.header.kind {
                Kind::BaselineChunk => size != 0,
                Kind::InputBatch => size == 16,
                Kind::AdmittedSets => size == 10,
                Kind::Divergence => size == 9,
                _ => false,
            };
            case.deliver(
                &packet(message.header, &message.payload[..size]),
                &images,
                !accepts,
            );
            payloads += 1;
        }
        let mut bad_envelope = case.message.clone();
        bad_envelope[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        case.deliver(&bad_envelope, &images, true);

        // Reframe a deliberately invalid complete payload so validity of
        // the application decoder, independently of envelope cuts, matters.
        let invalid = match message.header.kind {
            Kind::BaselineEnd | Kind::Close => vec![1],
            Kind::AdmittedSets => [vec![0; 10], vec![255, 255, 6]].concat(),
            Kind::Command => [vec![7, 0, 0, 0], vec![255, 255]].concat(),
            Kind::Divergence => [vec![1], vec![0; 11]].concat(),
            Kind::Welcome | Kind::BaselineBegin => {
                let mut invalid = message.payload.to_vec();
                invalid[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
                invalid
            }
            _ => Vec::new(),
        };
        case.deliver(&packet(message.header, &invalid), &images, true);
        for length in 1..=24 {
            let bytes: Vec<_> = (0..length)
                .map(|_| {
                    random ^= random << 13;
                    random ^= random >> 7;
                    random ^= random << 17;
                    random.to_le_bytes()[0]
                })
                .collect();
            case.deliver(&bytes, &images, true);
            // Random baseline chunks are valid framing and are checked when
            // the complete image is restored.
            let framed = packet(message.header, &bytes);
            match (message.header.kind, bytes.as_slice()) {
                (Kind::BaselineChunk, _) => case.deliver(&framed, &images, false),
                (Kind::Reject, [1..=3]) | (Kind::MatchEnded, [1 | 2]) | (Kind::Divergence, [0]) => {
                }
                (Kind::Divergence, [1, ..]) if bytes.len() == 9 => {}
                _ => case.deliver(&framed, &images, true),
            }
        }
        assert!(case.message.len() <= MAX_MESSAGE_BYTES);
    }
    println!(
        "{} lane/decoder fixtures, {envelopes} envelope cuts and {payloads} reframed payload cuts",
        cases.len()
    );
}

#[test]
fn refusing_a_set_batch_preserves_progress_and_leaves_unresolved_commands_uncertain() {
    let (_, images) = startup();
    for lane in [Lane::Reliable, Lane::Datagram] {
        for buffered in [false, true] {
            let mut game = Match::new(local(), 1, None, false);
            begin_client(&mut game);
            for image in &images {
                to_client(&mut game, Lane::Reliable, image);
            }
            assert_eq!(
                game.clients[0].command(&19).expect("first queued command"),
                1
            );
            assert_eq!(
                game.clients[0].command(&19).expect("second queued command"),
                2
            );

            let mut body = vec![0; 10];
            if buffered {
                append_command_set(&mut body, 3);
                to_client(
                    &mut game,
                    lane,
                    &packet(header(Kind::AdmittedSets, 1, 2, 1), &body),
                );
                body.truncate(10);
            }
            let client = &mut game.clients[0];
            let before = (client.held(), client.newest(), client.world().digest());
            assert_eq!(before.0, 0);
            assert_eq!(client.result(), None);
            append_command_set(&mut body, 1);
            if !buffered {
                append_command_set(&mut body, 3);
            }
            to_client(
                &mut game,
                lane,
                &packet(header(Kind::AdmittedSets, 1, 1, 1), &body),
            );
            let client = &mut game.clients[0];
            assert_eq!(client.state(), &ClientState::Closed(CloseReason::Protocol));
            assert_eq!(
                (client.held(), client.newest(), client.world().digest()),
                before
            );
            let commands: Vec<_> = std::iter::from_fn(|| client.result())
                .filter_map(|ResultEvent { input, disposition }| {
                    assert_eq!(disposition, Disposition::Uncertain);
                    match input {
                        Input::Command(sequence) => Some(sequence),
                        Input::Frame(_) => None,
                    }
                })
                .collect();
            assert_eq!(commands, [1, 2]);
        }
    }
}

#[test]
fn welcomed_peers_cannot_stage_out_of_state_foreign_or_over_rate_input() {
    let (_, images) = startup();
    let patterns = [
        (
            Lane::Reliable,
            packet(header(Kind::Welcome, 1, 0, 0), &images[0]),
        ),
        (Lane::Datagram, report(1, 2, 2)),
        (Lane::Datagram, report(1, 3, 1)),
    ];
    for (lane, bytes) in patterns {
        let mut game = Match::new(local(), 1, None, false);
        game.ready();
        let now = game.now;
        to_host(&mut game, lane, &bytes, now, 0);
        assert!(!game.links[0][1].is_open());
        game.host
            .advance(now + 25 * u64::from(game.settings.tick_us), 25);
        assert_eq!(game.host.world().log(), []);
        assert_eq!(game.host.admissions(), [false; 4]);
    }

    // These distinct records each fit the declared repeat limit. They still
    // cannot enter more than the negotiated burst without the host clock earning credit.
    let settings = Settings {
        behind: 4,
        ..Settings::default()
    };
    let mut game = Match::with_settings(local(), 1, None, false, settings);
    game.ready();
    let now = game.now;
    for first in [1, 3, 5, 7] {
        to_host(&mut game, Lane::Datagram, &report(first, 2, 1), now, 0);
        assert!(game.links[0][1].is_open());
    }
    let link = game.links[0][1].id();
    assert_eq!(
        game.host
            .stream(link)
            .expect("bounded fixture bytes")
            .staged_frames,
        8
    );
    to_host(&mut game, Lane::Datagram, &report(9, 2, 1), now, 0);
    assert!(!game.links[0][1].is_open());
    assert!(game.host.stream(link).is_none());
}

#[test]
fn an_invalid_complete_image_leaves_the_replica_world_untouched_and_requests_repair() {
    let (_, images) = startup();
    let mut game = Match::new(local(), 1, None, false);
    begin_client(&mut game);
    let before = game.clients[0].world().digest();
    to_client(&mut game, Lane::Reliable, &images[0]);
    let chunk = envelope::decode(&images[1]).expect("fixture image");
    let mut corrupted = chunk.payload.to_vec();
    corrupted[0] ^= 1;
    to_client(&mut game, Lane::Reliable, &packet(chunk.header, &corrupted));
    to_client(&mut game, Lane::Reliable, &images[2]);
    assert_eq!(game.clients[0].state(), &ClientState::Repairing);
    assert_eq!(game.clients[0].world().digest(), before);
    game.memory.advance(0);
    let request = game.links[0][1].receive(Lane::Reliable);
    let request = request
        .as_deref()
        .expect("repair requested over the same link");
    assert_eq!(
        envelope::decode(request)
            .expect("repair request")
            .header
            .kind,
        Kind::Divergence
    );
}
