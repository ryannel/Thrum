//! Client of the authoring dev server, for targets that cannot map the store (the browser).
//!
//! The protocol is target-independent and runs over any [`Fetch`] transport; the browser
//! transport (`BrowserFetch`, Wasm only) is a thin wrapper over the global `fetch`. A poll asks
//! `/head` (then long-polls `/head?after=N`), decodes the 64-byte record with the same parser as
//! the native reader, fetches the manifest and only the products it does not already hold, and
//! checks every fetched byte against the BLAKE3 name it was requested by before anything parses
//! it. The result is a [`RemoteGeneration`]: a validated manifest plus 64-aligned owned product
//! bytes, resolved exactly as a native consumer resolves a mapped store.

use std::collections::HashMap;
use std::future::Future;

use crate::{AssetError, ContentHash, Head, Manifest, ProductBytes, ProductKind, Profile};

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
pub use browser::BrowserFetch;

/// One HTTP reply as the client needs it: the status and the whole body.
#[derive(Debug)]
pub struct FetchReply {
    /// HTTP status code.
    pub status: u16,
    /// Response body in aligned storage, so a verified product is used without another copy.
    pub body: ProductBytes,
}

/// A GET transport for [`DevClient`].
pub trait Fetch {
    /// Request `url`. Implementations should start the request when this is called rather than
    /// when the future is first polled, so the client can issue every missing product at once.
    /// The future borrows neither the transport nor `url`.
    ///
    /// # Errors
    /// [`AssetError::Transport`] when no reply arrives. Any status is a reply, not an error.
    fn get(&self, url: &str) -> impl Future<Output = Result<FetchReply, AssetError>> + use<Self>;
}

/// A loaded publication: its record, its validated manifest and the products this client's
/// profile preferences resolve to.
#[derive(Debug)]
pub struct RemoteGeneration {
    head: Head,
    manifest: Manifest,
    products: HashMap<ContentHash, ProductBytes>,
}

impl RemoteGeneration {
    /// Publication record that selected this generation.
    pub fn head(&self) -> Head {
        self.head
    }

    /// Validated manifest; resolve names with [`Manifest::resolve`] as on native.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Verified bytes of a product, shared rather than copied. Only products the client's
    /// preferences select are present; open them with the same typed views as mapped bytes.
    pub fn product(&self, hash: &ContentHash) -> Option<ProductBytes> {
        self.products.get(hash).cloned()
    }
}

/// Polls a dev server and loads each new publication.
///
/// Generations never go backwards: within one store epoch a record at or below the resident
/// generation is ignored, and a new epoch (the store was recreated) always counts as new. A
/// failed load leaves the resident generation in place and, like a native [`crate::HeadWatcher`],
/// is not retried until a later publication.
#[derive(Debug)]
pub struct DevClient<F> {
    fetch: F,
    base: String,
    preferences: [Vec<Profile>; 3],
    /// Epoch and generation of the last generation returned.
    resident: Option<(u32, u64)>,
    /// Generation the next long poll waits beyond; `None` asks for the current record at once.
    after: Option<u64>,
    /// Verified products of the resident generation (and its manifest), by content address.
    held: HashMap<ContentHash, ProductBytes>,
}

impl<F: Fetch> DevClient<F> {
    /// Client of the server at `base` (`http://127.0.0.1:7878`). Every kind prefers
    /// [`Profile::Portable`] until [`DevClient::prefer`] says otherwise.
    ///
    /// # Errors
    /// [`AssetError::Malformed`] unless `base` is an `http`/`https` origin with no path.
    pub fn new(fetch: F, base: &str) -> Result<Self, AssetError> {
        let base = base.strip_suffix('/').unwrap_or(base);
        let authority = base
            .strip_prefix("http://")
            .or_else(|| base.strip_prefix("https://"))
            .ok_or(AssetError::Malformed("dev server URL scheme"))?;
        if authority.is_empty() || authority.contains(['/', '?', '#', '@']) {
            return Err(AssetError::Malformed("dev server URL authority"));
        }
        Ok(Self {
            fetch,
            base: base.to_owned(),
            preferences: std::array::from_fn(|_| vec![Profile::Portable]),
            resident: None,
            after: None,
            held: HashMap::new(),
        })
    }

    /// Profiles to resolve products of `kind` with, best first, as passed to
    /// [`Manifest::resolve`]. Only the resolved product of each name is fetched.
    pub fn prefer(&mut self, kind: ProductKind, profiles: &[Profile]) {
        self.preferences[kind_index(kind)] = profiles.to_vec();
    }

    /// URL of the next `HEAD` request.
    pub fn head_url(&self) -> String {
        match self.after {
            None => format!("{}/head", self.base),
            Some(after) => format!("{}/head?after={after}", self.base),
        }
    }

    /// URL of a content-addressed product.
    pub fn product_url(&self, hash: &ContentHash) -> String {
        format!("{}/cas/{}", self.base, hash.to_hex())
    }

    /// Epoch and generation of the last generation returned.
    pub fn resident(&self) -> Option<(u32, u64)> {
        self.resident
    }

    /// Whether verified bytes for `hash` are held and would not be fetched again.
    pub fn holds(&self, hash: &ContentHash) -> bool {
        self.held.contains_key(hash)
    }

    /// Ask for the next publication (long-polling after the first) and load it when it is new.
    /// `Ok(None)` means nothing new arrived, including a long-poll timeout; call again.
    ///
    /// # Errors
    /// Transport failures, unexpected statuses, malformed records or manifests, a manifest
    /// whose generation disagrees with its record, and bytes that do not match their hash. The
    /// resident generation is unchanged on every error.
    pub async fn poll(&mut self) -> Result<Option<RemoteGeneration>, AssetError> {
        let reply = self.fetch.get(&self.head_url()).await?;
        let Some(head) = classify_head(&reply)? else {
            // Nothing published yet: wait for the first publication.
            self.after = Some(self.after.unwrap_or(0));
            return Ok(None);
        };
        if !is_newer(self.resident, &head) {
            return Ok(None);
        }
        // The server orders publications by generation alone; poll beyond this one even if its
        // load fails, as the native watcher does.
        self.after = Some(head.generation);
        self.load(head).await.map(Some)
    }

    async fn load(&mut self, head: Head) -> Result<RemoteGeneration, AssetError> {
        let manifest_bytes = if let Some(bytes) = self.held.get(&head.manifest) {
            bytes.clone()
        } else {
            let reply = self.fetch.get(&self.product_url(&head.manifest)).await?;
            accept_product(&head.manifest, None, reply)?
        };
        let manifest = Manifest::parse(manifest_bytes.clone())?;
        if manifest.generation() != head.generation {
            return Err(AssetError::Malformed(
                "manifest generation differs from HEAD",
            ));
        }
        let wanted = wanted_products(&manifest, &self.preferences);
        // Issue every missing request before awaiting any, so transfers overlap.
        let pending: Vec<_> = wanted
            .iter()
            .filter(|(hash, _)| !self.held.contains_key(*hash))
            .map(|(&hash, &len)| (hash, len, self.fetch.get(&self.product_url(&hash))))
            .collect();
        let mut fetched = HashMap::with_capacity(pending.len());
        for (hash, len, reply) in pending {
            let bytes = accept_product(&hash, Some(len), reply.await?)?;
            fetched.insert(hash, bytes);
        }
        let mut products = HashMap::with_capacity(wanted.len());
        for hash in wanted.keys() {
            let bytes = fetched
                .remove(hash)
                .or_else(|| self.held.get(hash).cloned())
                .ok_or(AssetError::Malformed("product neither held nor fetched"))?;
            products.insert(*hash, bytes);
        }
        // Hold exactly this generation; earlier products stay alive only in earlier generations.
        self.held.clone_from(&products);
        self.held.insert(head.manifest, manifest_bytes);
        self.resident = Some((head.epoch, head.generation));
        Ok(RemoteGeneration {
            head,
            manifest,
            products,
        })
    }
}

fn kind_index(kind: ProductKind) -> usize {
    match kind {
        ProductKind::Mesh => 0,
        ProductKind::Texture => 1,
        ProductKind::Volume => 2,
    }
}

/// Whether `head` replaces the resident generation (same rule as the native watcher).
fn is_newer(resident: Option<(u32, u64)>, head: &Head) -> bool {
    !resident
        .is_some_and(|(epoch, generation)| head.epoch == epoch && head.generation <= generation)
}

/// `200` carries a record, `204` means nothing is published yet; anything else is refused.
fn classify_head(reply: &FetchReply) -> Result<Option<Head>, AssetError> {
    match reply.status {
        200 => Head::decode(&reply.body).map(Some),
        204 => Ok(None),
        status => Err(AssetError::Status(status)),
    }
}

/// Accept a product reply only when it is a `200` whose bytes are exactly what `hash` names.
fn accept_product(
    hash: &ContentHash,
    expected_len: Option<u64>,
    reply: FetchReply,
) -> Result<ProductBytes, AssetError> {
    if reply.status != 200 {
        return Err(AssetError::Status(reply.status));
    }
    if expected_len.is_some_and(|len| len != reply.body.len() as u64) {
        return Err(AssetError::HashMismatch);
    }
    hash.verify(&reply.body)?;
    Ok(reply.body)
}

/// The product each logical name resolves to under `preferences`, with its manifest length.
fn wanted_products(
    manifest: &Manifest,
    preferences: &[Vec<Profile>; 3],
) -> HashMap<ContentHash, u64> {
    let mut wanted = HashMap::new();
    let mut previous = None;
    // Entries are sorted by name key, so each name's profiles are adjacent.
    for entry in manifest.entries() {
        if previous == Some(entry.name) {
            continue;
        }
        previous = Some(entry.name);
        if let Some(chosen) = manifest.resolve(entry.name, &preferences[kind_index(entry.kind)]) {
            wanted.insert(chosen.product, chosen.byte_len);
        }
    }
    wanted
}

#[cfg(test)]
mod tests;
