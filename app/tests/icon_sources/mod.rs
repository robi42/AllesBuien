//! The icon transport `weather_icons` scripts: one code held back, one refused
//! once, and the rest served at once.
//!
//! This lives beside its one consumer rather than in `allesbuien-testkit`,
//! because `IconSource` belongs to the app crate and the testkit is Qt-free and
//! app-free on purpose. The body is a real, decodable 1x1 PNG, so a delivered
//! icon reaches `Image.Ready` and its skeleton can be watched going away.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, PoisonError},
};

use allesbuien::icon_cache::{IconFuture, IconSource};
use futures::{
    FutureExt,
    channel::oneshot::{self, Receiver, Sender},
    future::Shared,
};

/// A complete, transparent 1x1 RGBA PNG.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\
    \x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\
    \x00\x00\x00\x0bIDAT\x78\x9c\x63\x60\x00\x02\x00\x00\x05\x00\x01\x7a\x5e\xab\x3f\
    \x00\x00\x00\x00IEND\xae\x42\x60\x82";

/// Serves [`PNG`] for every code, except as scripted per key.
pub struct ScriptedIconSource {
    gated: BTreeSet<String>,
    gate: Shared<Receiver<()>>,
    refuse_once: Mutex<BTreeSet<String>>,
    calls: Mutex<BTreeMap<String, usize>>,
}

impl ScriptedIconSource {
    /// Holds every `gated` key until the returned sender fires, and answers
    /// each `refuse_once` key with nothing on its first request only.
    pub fn new(gated: &[&str], refuse_once: &[&str]) -> (Arc<Self>, Sender<()>) {
        let (release, gate) = oneshot::channel();
        let source = Self {
            gated: gated.iter().map(|key| (*key).to_owned()).collect(),
            gate: gate.shared(),
            refuse_once: Mutex::new(refuse_once.iter().map(|key| (*key).to_owned()).collect()),
            calls: Mutex::new(BTreeMap::new()),
        };
        (Arc::new(source), release)
    }

    /// How many times `key` has been requested so far.
    pub fn calls(&self, key: &str) -> usize {
        let calls = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
        calls.get(key).copied().unwrap_or(0)
    }
}

impl IconSource for ScriptedIconSource {
    fn get(&self, url: &str) -> IconFuture<'_> {
        let key = url
            .rsplit('/')
            .next()
            .and_then(|file| file.strip_suffix(".png"))
            .unwrap_or_default()
            .to_owned();
        *self
            .calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(key.clone())
            .or_default() += 1;
        let refused = self
            .refuse_once
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&key);
        let gate = self.gated.contains(&key).then(|| self.gate.clone());
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.await.ok()?;
            }
            (!refused).then(|| PNG.to_vec())
        })
    }
}
