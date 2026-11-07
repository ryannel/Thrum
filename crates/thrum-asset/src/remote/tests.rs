use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use super::{DevClient, Fetch, FetchReply};
use crate::layout::{
    CONTAINER_FORMAT, ContainerHeader, MANIFEST_MAGIC, ManifestMeta, ManifestRecord, SECTION_ALIGN,
    Section, manifest_tag,
};
use crate::{AssetError, ContentHash, Head, ProductBytes, ProductKind, Profile};

const BASE: &str = "http://127.0.0.1:7878";

/// In-memory dev server: fixed replies by URL, and a log of every request made.
#[derive(Default)]
struct FakeServer {
    replies: RefCell<HashMap<String, (u16, Vec<u8>)>>,
    log: RefCell<Vec<String>>,
}

impl FakeServer {
    fn reply(&self, path: &str, status: u16, body: &[u8]) {
        self.replies
            .borrow_mut()
            .insert(format!("{BASE}{path}"), (status, body.to_vec()));
    }

    /// Store `bytes` under their content address and return it.
    fn put(&self, bytes: &[u8]) -> ContentHash {
        let hash = ContentHash::of(bytes);
        self.reply(&format!("/cas/{hash}"), 200, bytes);
        hash
    }

    fn publish(&self, path: &str, epoch: u32, generation: u64, manifest: ContentHash) {
        let head = Head {
            epoch,
            generation,
            manifest,
        };
        self.reply(path, 200, &head.encode());
    }

    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
            .into_iter()
            .map(|url| url.trim_start_matches(BASE).to_owned())
            .collect()
    }
}

impl<'a> Fetch for &'a FakeServer {
    fn get(&self, url: &str) -> impl Future<Output = Result<FetchReply, AssetError>> + use<'a> {
        self.log.borrow_mut().push(url.to_owned());
        let reply = self
            .replies
            .borrow()
            .get(url)
            .map(|(status, body)| FetchReply {
                status: *status,
                body: ProductBytes::copy_from(body),
            })
            .ok_or_else(|| AssetError::Transport(format!("no route {url}")));
        std::future::ready(reply)
    }
}

/// Drive a future whose transport is always ready.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

/// A manifest container in the cooker's layout.
fn manifest(
    generation: u64,
    entries: &[(&str, ProductKind, Profile, ContentHash, u64)],
) -> Vec<u8> {
    let mut strings = Vec::new();
    let mut records: Vec<ManifestRecord> = entries
        .iter()
        .map(|&(name, kind, profile, product, byte_len)| {
            let record = ManifestRecord {
                name_key: ContentHash::of(name.as_bytes()).prefix_u64(),
                product: *product.as_bytes(),
                byte_len,
                name_offset: u32::try_from(strings.len()).unwrap(),
                name_len: u16::try_from(name.len()).unwrap(),
                kind: kind.code(),
                profile: profile.code(),
                reserved: [0; 8],
            };
            strings.extend_from_slice(name.as_bytes());
            record
        })
        .collect();
    records.sort_by_key(|record| (record.name_key, record.profile));
    let meta = ManifestMeta {
        generation,
        entry_count: u32::try_from(records.len()).unwrap(),
        reserved: [0; 13],
    };
    let sections: [([u8; 4], usize, &[u8]); 3] = [
        (
            manifest_tag::META,
            size_of::<ManifestMeta>(),
            bytemuck::bytes_of(&meta),
        ),
        (
            manifest_tag::ENTRIES,
            size_of::<ManifestRecord>(),
            bytemuck::cast_slice(&records),
        ),
        (manifest_tag::STRINGS, 1, &strings),
    ];
    let table_end = size_of::<ContainerHeader>() + sections.len() * size_of::<Section>();
    let mut out = vec![0; table_end];
    let mut table = Vec::new();
    for (tag, element_size, payload) in sections {
        out.resize(out.len().next_multiple_of(SECTION_ALIGN), 0);
        table.push(Section {
            tag,
            element_size: u32::try_from(element_size).unwrap(),
            offset: out.len() as u64,
            count: (payload.len() / element_size) as u64,
            reserved: 0,
        });
        out.extend_from_slice(payload);
    }
    let header = ContainerHeader {
        magic: MANIFEST_MAGIC,
        format: CONTAINER_FORMAT,
        section_count: 3,
        total_len: out.len() as u64,
        reserved: [0; 5],
    };
    out[..size_of::<ContainerHeader>()].copy_from_slice(bytemuck::bytes_of(&header));
    out[size_of::<ContainerHeader>()..table_end].copy_from_slice(bytemuck::cast_slice(&table));
    out
}

/// Publish a mesh and a texture cooked for two profiles; returns (manifest, mesh, portable
/// texture, BC texture).
fn publish_one(
    server: &FakeServer,
    path: &str,
    epoch: u32,
    generation: u64,
    mesh: &[u8],
    texture: &[u8],
) -> [ContentHash; 4] {
    let mesh_hash = server.put(mesh);
    let portable = server.put(texture);
    let bc = server.put(&[texture, b"-bc7"].concat());
    let manifest_hash = server.put(&manifest(
        generation,
        &[
            (
                "meshes/rock",
                ProductKind::Mesh,
                Profile::Portable,
                mesh_hash,
                mesh.len() as u64,
            ),
            (
                "textures/rock",
                ProductKind::Texture,
                Profile::Portable,
                portable,
                texture.len() as u64,
            ),
            (
                "textures/rock",
                ProductKind::Texture,
                Profile::DesktopBc,
                bc,
                texture.len() as u64 + 4,
            ),
        ],
    ));
    server.publish(path, epoch, generation, manifest_hash);
    [manifest_hash, mesh_hash, portable, bc]
}

#[test]
fn urls_are_built_from_a_checked_origin() {
    let server = FakeServer::default();
    let client = DevClient::new(&server, "http://localhost:7878/").unwrap();
    assert_eq!(client.head_url(), "http://localhost:7878/head");
    let hash = ContentHash::of(b"x");
    assert_eq!(
        client.product_url(&hash),
        format!("http://localhost:7878/cas/{hash}")
    );
    for bad in [
        "localhost:7878",
        "ftp://localhost",
        "http://",
        "http://a/b",
        "http://u@h",
        "http://h?x",
    ] {
        assert!(DevClient::new(&server, bad).is_err(), "{bad}");
    }
}

#[test]
fn first_poll_loads_only_the_preferred_products_and_then_long_polls() {
    let server = FakeServer::default();
    let [manifest, mesh, portable, bc] = publish_one(&server, "/head", 9, 1, b"mesh-1", b"tex-1");
    let mut client = DevClient::new(&server, BASE).unwrap();
    let generation = block_on(client.poll()).unwrap().expect("first publication");
    assert_eq!(generation.head().generation, 1);
    assert_eq!(&generation.product(&mesh).unwrap()[..], b"mesh-1");
    assert_eq!(&generation.product(&portable).unwrap()[..], b"tex-1");
    assert!(generation.product(&bc).is_none(), "BC is not preferred");
    let entry = generation
        .manifest()
        .resolve("textures/rock", &[Profile::Portable])
        .unwrap();
    assert_eq!(entry.product, portable);
    let mut log = server.take_log();
    log[2..].sort();
    let mut expected = vec![format!("/cas/{mesh}"), format!("/cas/{portable}")];
    expected.sort();
    assert_eq!(log[..2], ["/head".to_owned(), format!("/cas/{manifest}")]);
    assert_eq!(log[2..], expected);
    assert_eq!(client.head_url(), format!("{BASE}/head?after=1"));
}

#[test]
fn a_later_publication_fetches_only_what_changed_and_preferences_apply() {
    let server = FakeServer::default();
    publish_one(&server, "/head", 9, 1, b"mesh-1", b"tex-1");
    let mut client = DevClient::new(&server, BASE).unwrap();
    client.prefer(
        ProductKind::Texture,
        &[Profile::DesktopBc, Profile::Portable],
    );
    let first = block_on(client.poll()).unwrap().unwrap();
    server.take_log();
    let [manifest, mesh, portable, bc] =
        publish_one(&server, "/head?after=1", 9, 2, b"mesh-1", b"tex-2");
    let second = block_on(client.poll()).unwrap().unwrap();
    assert_eq!(second.head().generation, 2);
    assert_eq!(
        server.take_log(),
        [
            "/head?after=1".to_owned(),
            format!("/cas/{manifest}"),
            format!("/cas/{bc}")
        ],
        "the unchanged mesh is held, and only the preferred texture profile is fetched"
    );
    assert_eq!(&second.product(&bc).unwrap()[..], b"tex-2-bc7");
    assert!(second.product(&portable).is_none());
    assert_eq!(
        second.product(&mesh).unwrap().as_ptr(),
        first.product(&mesh).unwrap().as_ptr(),
        "held products are shared, not copied"
    );
    assert_eq!(second.product(&mesh).unwrap().as_ptr().align_offset(64), 0);
    assert!(client.holds(&mesh) && client.holds(&manifest));
    assert!(
        !client.holds(&ContentHash::of(b"tex-1-bc7")),
        "superseded products are released"
    );
}

#[test]
fn tampered_bytes_are_refused_and_the_resident_generation_stays() {
    let server = FakeServer::default();
    publish_one(&server, "/head", 9, 1, b"mesh-1", b"tex-1");
    let mut client = DevClient::new(&server, BASE).unwrap();
    block_on(client.poll()).unwrap().unwrap();

    // A product whose served bytes are not what its name says.
    let [_, mesh, ..] = publish_one(&server, "/head?after=1", 9, 2, b"mesh-2", b"tex-1");
    server.reply(&format!("/cas/{mesh}"), 200, b"mesh-X");
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::HashMismatch)
    ));
    assert_eq!(client.resident(), Some((9, 1)));
    assert!(!client.holds(&mesh));

    // A truncated product (length disagrees with the manifest) and a tampered manifest.
    let [_, mesh, ..] = publish_one(&server, "/head?after=2", 9, 3, b"mesh-3", b"tex-1");
    server.reply(&format!("/cas/{mesh}"), 200, b"mesh");
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::HashMismatch)
    ));
    let [manifest, ..] = publish_one(&server, "/head?after=3", 9, 4, b"mesh-4", b"tex-1");
    let mut bytes = server.replies.borrow()[&format!("{BASE}/cas/{manifest}")]
        .1
        .clone();
    bytes[200] ^= 1;
    server.reply(&format!("/cas/{manifest}"), 200, &bytes);
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::HashMismatch)
    ));
    assert_eq!(client.resident(), Some((9, 1)));

    // A later good publication still loads.
    publish_one(&server, "/head?after=4", 9, 5, b"mesh-5", b"tex-1");
    assert_eq!(
        block_on(client.poll()).unwrap().unwrap().head().generation,
        5
    );
    assert_eq!(client.resident(), Some((9, 5)));
}

#[test]
fn head_statuses_are_classified() {
    let server = FakeServer::default();
    server.reply("/head", 204, &[]);
    server.reply("/head?after=0", 204, &[]);
    let mut client = DevClient::new(&server, BASE).unwrap();
    assert!(
        block_on(client.poll()).unwrap().is_none(),
        "204: nothing published"
    );
    assert_eq!(
        client.head_url(),
        format!("{BASE}/head?after=0"),
        "then long-poll"
    );
    assert!(block_on(client.poll()).unwrap().is_none());
    assert_eq!(server.take_log(), ["/head", "/head?after=0"]);

    server.reply("/head?after=0", 503, &[]);
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::Status(503))
    ));
    let mut damaged = Head {
        epoch: 1,
        generation: 1,
        manifest: ContentHash::of(b"m"),
    }
    .encode();
    damaged[20] ^= 1;
    server.reply("/head?after=0", 200, &damaged);
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::Malformed(_))
    ));
    server.reply("/head?after=0", 200, &damaged[..32]);
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::Malformed(_))
    ));

    // A missing product is a status error, not an empty product.
    let missing = ContentHash::of(b"absent");
    server.publish("/head?after=0", 1, 1, missing);
    server.reply(&format!("/cas/{missing}"), 404, &[]);
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::Status(404))
    ));
    assert_eq!(client.resident(), None);
}

#[test]
fn generations_never_go_backwards_within_an_epoch() {
    let server = FakeServer::default();
    publish_one(&server, "/head", 9, 3, b"mesh-3", b"tex-3");
    let mut client = DevClient::new(&server, BASE).unwrap();
    block_on(client.poll()).unwrap().unwrap();
    server.take_log();

    // A long-poll timeout returns the same record; an older one must not be loaded either.
    publish_one(&server, "/head?after=3", 9, 3, b"mesh-3", b"tex-3");
    assert!(block_on(client.poll()).unwrap().is_none());
    publish_one(&server, "/head?after=3", 9, 2, b"mesh-2", b"tex-2");
    assert!(block_on(client.poll()).unwrap().is_none());
    assert_eq!(
        server.take_log(),
        ["/head?after=3", "/head?after=3"],
        "no product fetches"
    );
    assert_eq!(client.resident(), Some((9, 3)));
    assert_eq!(client.head_url(), format!("{BASE}/head?after=3"));

    // A manifest that disagrees with its record is refused.
    let mesh = server.put(b"mesh-4");
    let wrong = server.put(&manifest(
        7,
        &[("meshes/rock", ProductKind::Mesh, Profile::Portable, mesh, 6)],
    ));
    server.publish("/head?after=3", 9, 4, wrong);
    assert!(matches!(
        block_on(client.poll()),
        Err(AssetError::Malformed(_))
    ));
    assert_eq!(client.resident(), Some((9, 3)));

    // A recreated store (new epoch) restarts at generation 1 and is accepted.
    publish_one(&server, "/head?after=4", 10, 1, b"mesh-n", b"tex-n");
    assert_eq!(block_on(client.poll()).unwrap().unwrap().head().epoch, 10);
    assert_eq!(client.resident(), Some((10, 1)));
}
