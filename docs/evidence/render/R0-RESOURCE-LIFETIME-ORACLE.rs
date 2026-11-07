// Standalone R0 schedule oracle. Run with:
// rustc --edition=2021 --test docs/evidence/render/R0-RESOURCE-LIFETIME-ORACLE.rs -o tmp/r0-resource-lifetime-oracle
// tmp/r0-resource-lifetime-oracle
//
// This models logical texture bytes and queue/lease lifetime. It does not call a GPU.
use std::collections::BTreeMap;

const MIB: u64 = 1024 * 1024;
const SHADOW_BYTES: u64 = 2048 * 2048 * 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Extent {
    width: u64,
    height: u64,
}

impl Extent {
    fn pixels(self) -> Option<u64> {
        self.width.checked_mul(self.height)
    }

    fn view_bytes(self) -> Option<u64> {
        self.pixels()?.checked_mul(20) // D32 + two RGBA16Float
    }

    fn output_bytes(self) -> Option<u64> {
        self.pixels()?.checked_mul(4) // harness-owned RGBA8Unorm
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Receipt {
    device: u64,
    serial: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Reservation {
    device: u64,
    graph_epoch: u64,
    nonce: u64,
    slot: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Credit {
    Free,
    Reserved(Reservation),
    Submitted(Receipt),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Allocation {
    descriptor: TextureDescriptor,
    bytes: u64,
    last_use: u64,
    cpu_leases: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextureRole {
    Shadow,
    Depth,
    Linear,
    Encoded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TextureDescriptor {
    role: TextureRole,
    extent: Extent,
    bytes_per_pixel: u64,
    // The bounded proof fixes dimension=2D, one layer/mip/sample, one device,
    // and the role's exact render/sample usages. Wider keys are a real-V1 gate.
}

impl TextureDescriptor {
    fn bytes(self) -> Option<u64> {
        self.extent.pixels()?.checked_mul(self.bytes_per_pixel)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Graph {
    extent: Extent,
    shadow: u64,
    depth: u64,
    linear: u64,
    encoded: u64,
}

impl Graph {
    fn ids(self) -> [u64; 4] {
        [self.shadow, self.depth, self.linear, self.encoded]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Arithmetic,
    OwnerBudget,
    CreditsFull,
    NoGraph,
    Busy,
}

struct Schedule {
    device: u64,
    next_serial: u64,
    next_reservation: u64,
    graph_epoch: u64,
    completed: u64,
    next_id: u64,
    owner_budget: u64,
    allocations: BTreeMap<u64, Allocation>,
    retired_cpu_leases: BTreeMap<u64, u32>,
    active: Option<Graph>,
    pending: Option<Graph>,
    credits: [Credit; 2],
    last_submitted_sample: u64,
}

impl Schedule {
    fn new(owner_budget: u64) -> Self {
        Self {
            device: 1,
            next_serial: 0,
            next_reservation: 0,
            graph_epoch: 0,
            completed: 0,
            next_id: 0,
            owner_budget,
            allocations: BTreeMap::new(),
            retired_cpu_leases: BTreeMap::new(),
            active: None,
            pending: None,
            credits: [Credit::Free; 2],
            last_submitted_sample: 0,
        }
    }

    fn live_bytes(&self) -> u64 {
        self.allocations.values().map(|a| a.bytes).sum()
    }

    fn add(&mut self, descriptor: TextureDescriptor) -> Result<u64, Refusal> {
        let bytes = descriptor.bytes().ok_or(Refusal::Arithmetic)?;
        let projected = self
            .live_bytes()
            .checked_add(bytes)
            .ok_or(Refusal::Arithmetic)?;
        if projected > self.owner_budget {
            return Err(Refusal::OwnerBudget);
        }
        self.next_id = self.next_id.checked_add(1).ok_or(Refusal::Arithmetic)?;
        self.allocations.insert(
            self.next_id,
            Allocation {
                descriptor,
                bytes,
                last_use: 0,
                cpu_leases: 0,
            },
        );
        Ok(self.next_id)
    }

    // Checks the whole candidate before allocating any of its textures.
    fn prepare(&mut self, extent: Extent) -> Result<(), Refusal> {
        if self.pending.is_some() {
            return Err(Refusal::Busy);
        }
        let depth = TextureDescriptor {
            role: TextureRole::Depth,
            extent,
            bytes_per_pixel: 4,
        };
        let linear = TextureDescriptor {
            role: TextureRole::Linear,
            extent,
            bytes_per_pixel: 8,
        };
        let encoded = TextureDescriptor {
            role: TextureRole::Encoded,
            extent,
            bytes_per_pixel: 8,
        };
        let shadow_desc = TextureDescriptor {
            role: TextureRole::Shadow,
            extent: Extent {
                width: 2048,
                height: 2048,
            },
            bytes_per_pixel: 4,
        };
        let shadow = self.active.map(|g| g.shadow);
        let added = depth
            .bytes()
            .ok_or(Refusal::Arithmetic)?
            .checked_add(linear.bytes().ok_or(Refusal::Arithmetic)?)
            .and_then(|n| n.checked_add(encoded.bytes()?))
            .ok_or(Refusal::Arithmetic)?
            .checked_add(if shadow.is_none() { SHADOW_BYTES } else { 0 })
            .ok_or(Refusal::Arithmetic)?;
        let projected = self
            .live_bytes()
            .checked_add(added)
            .ok_or(Refusal::Arithmetic)?;
        if projected > self.owner_budget {
            return Err(Refusal::OwnerBudget);
        }
        self.next_id
            .checked_add(if shadow.is_some() { 3 } else { 4 })
            .ok_or(Refusal::Arithmetic)?;
        let shadow = match shadow {
            Some(id) => id,
            None => self.add(shadow_desc)?,
        };
        let depth = self.add(depth)?;
        let linear = self.add(linear)?;
        let encoded = self.add(encoded)?;
        self.pending = Some(Graph {
            extent,
            shadow,
            depth,
            linear,
            encoded,
        });
        Ok(())
    }

    // Healthy-device cancellation releases only candidate allocations.
    fn abort_pending(&mut self) -> Result<(), Refusal> {
        if let Some(candidate) = self.pending {
            let removed = [candidate.depth, candidate.linear, candidate.encoded];
            if removed.into_iter().any(|id| {
                self.allocations
                    .get(&id)
                    .expect("pending texture")
                    .cpu_leases
                    != 0
            }) || (self.active.is_none()
                && self
                    .allocations
                    .get(&candidate.shadow)
                    .expect("pending shadow")
                    .cpu_leases
                    != 0)
            {
                return Err(Refusal::Busy);
            }
            self.pending = None;
            for id in removed {
                self.allocations.remove(&id);
            }
            if self.active.is_none() {
                self.allocations.remove(&candidate.shadow);
            }
        }
        Ok(())
    }

    fn publish(&mut self) -> Result<(), Refusal> {
        if self
            .credits
            .iter()
            .any(|credit| matches!(credit, Credit::Reserved(_)))
        {
            return Err(Refusal::Busy);
        }
        if self.pending.is_none() {
            return Err(Refusal::NoGraph);
        }
        let epoch = self.graph_epoch.checked_add(1).ok_or(Refusal::Arithmetic)?;
        self.active = self.pending.take();
        self.graph_epoch = epoch;
        self.reclaim();
        Ok(())
    }

    fn reserve(&mut self) -> Result<Reservation, Refusal> {
        self.active.ok_or(Refusal::NoGraph)?;
        let slot = self
            .credits
            .iter()
            .position(|c| *c == Credit::Free)
            .ok_or(Refusal::CreditsFull)?;
        self.next_reservation = self
            .next_reservation
            .checked_add(1)
            .ok_or(Refusal::Arithmetic)?;
        let reservation = Reservation {
            device: self.device,
            graph_epoch: self.graph_epoch,
            nonce: self.next_reservation,
            slot,
        };
        self.credits[slot] = Credit::Reserved(reservation);
        Ok(reservation)
    }

    fn cancel(&mut self, reservation: Reservation) -> Result<(), Refusal> {
        if reservation.graph_epoch != self.graph_epoch
            || self.credits.get(reservation.slot) != Some(&Credit::Reserved(reservation))
        {
            return Err(Refusal::Busy);
        }
        self.credits[reservation.slot] = Credit::Free;
        Ok(())
    }

    fn submit(&mut self, reservation: Reservation, sample: u64) -> Result<Receipt, Refusal> {
        if reservation.graph_epoch != self.graph_epoch
            || self.credits.get(reservation.slot) != Some(&Credit::Reserved(reservation))
        {
            return Err(Refusal::Busy);
        }
        let graph = self.active.ok_or(Refusal::NoGraph)?;
        let serial = self.next_serial.checked_add(1).ok_or(Refusal::Arithmetic)?;
        self.next_serial = serial;
        let receipt = Receipt {
            device: self.device,
            serial,
        };
        self.credits[reservation.slot] = Credit::Submitted(receipt);
        for id in graph.ids() {
            self.allocations
                .get_mut(&id)
                .expect("prepared allocation")
                .last_use = serial;
        }
        self.last_submitted_sample = sample;
        Ok(receipt)
    }

    fn complete(&mut self, receipt: Receipt) {
        if receipt.device != self.device || receipt.serial > self.next_serial {
            return; // stale device callback cannot affect the new generation
        }
        if receipt.serial > self.completed
            && !self
                .credits
                .iter()
                .any(|credit| *credit == Credit::Submitted(receipt))
        {
            return; // only an issued, outstanding serial advances the watermark
        }
        self.completed = self.completed.max(receipt.serial);
        for credit in &mut self.credits {
            if let Credit::Submitted(held) = credit {
                if held.device == self.device && held.serial <= self.completed {
                    *credit = Credit::Free;
                }
            }
        }
        self.reclaim();
    }

    fn reclaim(&mut self) {
        let active = self.active;
        let pending = self.pending;
        self.allocations.retain(|id, a| {
            let prepared = [active, pending]
                .into_iter()
                .flatten()
                .any(|g| g.ids().contains(id));
            prepared || a.last_use > self.completed || a.cpu_leases != 0
        });
    }

    fn lease(&mut self, id: u64) {
        self.allocations
            .get_mut(&id)
            .expect("allocation")
            .cpu_leases += 1;
    }

    fn release(&mut self, id: u64) {
        if let Some(allocation) = self.allocations.get_mut(&id) {
            assert!(allocation.cpu_leases > 0);
            allocation.cpu_leases -= 1;
            self.reclaim();
        } else {
            let leases = self
                .retired_cpu_leases
                .get_mut(&id)
                .expect("old device CPU lease");
            assert!(*leases > 0);
            *leases -= 1;
            if *leases == 0 {
                self.retired_cpu_leases.remove(&id);
            }
        }
    }

    // Explicit visible stop/load. No old V1 graph remains available for rollback.
    fn stop_and_release_graph(&mut self) -> Result<(), Refusal> {
        if self.credits != [Credit::Free; 2] || self.pending.is_some() {
            return Err(Refusal::Busy);
        }
        self.active.ok_or(Refusal::NoGraph)?;
        if self
            .allocations
            .values()
            .any(|allocation| allocation.cpu_leases != 0 || allocation.last_use > self.completed)
        {
            return Err(Refusal::Busy);
        }
        self.active = None;
        self.reclaim();
        Ok(())
    }

    fn lose_device(&mut self) -> Result<(), Refusal> {
        self.device = self.device.checked_add(1).ok_or(Refusal::Arithmetic)?;
        self.next_serial = 0;
        self.completed = 0;
        self.active = None;
        self.pending = None;
        for (id, allocation) in &self.allocations {
            if allocation.cpu_leases != 0 {
                self.retired_cpu_leases.insert(*id, allocation.cpu_leases);
            }
        }
        self.allocations.clear(); // terminal loss discharges GPU holds, not CPU leases
        self.credits = [Credit::Free; 2];
        Ok(())
    }
}

// The harness account is deliberately separate from V1's owner budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HarnessOutput {
    old_bytes: u64,
    new_bytes: u64,
    old_last_use: u64,
    loading_diagnostic_ready: bool,
}

impl HarnessOutput {
    fn replace(old: Extent, new: Extent, old_last_use: u64) -> Result<Self, Refusal> {
        let old_bytes = old.output_bytes().ok_or(Refusal::Arithmetic)?;
        let new_bytes = new.output_bytes().ok_or(Refusal::Arithmetic)?;
        old_bytes
            .checked_add(new_bytes)
            .ok_or(Refusal::Arithmetic)?;
        Ok(Self {
            old_bytes,
            new_bytes,
            old_last_use,
            loading_diagnostic_ready: true,
        })
    }
    fn overlap_bytes(self) -> u64 {
        self.old_bytes + self.new_bytes
    }

    fn release_old_after(&mut self, completed: u64) -> Result<(), Refusal> {
        if completed < self.old_last_use {
            return Err(Refusal::Busy);
        }
        self.old_bytes = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HD: Extent = Extent {
        width: 1280,
        height: 720,
    };
    const FHD: Extent = Extent {
        width: 1920,
        height: 1080,
    };

    #[test]
    fn arithmetic_and_cross_frame_reuse() {
        assert_eq!(HD.view_bytes(), Some(18_432_000));
        assert_eq!(FHD.view_bytes(), Some(41_472_000));
        assert_eq!(HD.output_bytes(), Some(3_686_400));
        assert_eq!(FHD.output_bytes(), Some(8_294_400));
        assert_eq!(SHADOW_BYTES + HD.view_bytes().unwrap(), 35_209_216);
        assert_eq!(SHADOW_BYTES + FHD.view_bytes().unwrap(), 58_249_216);
        assert_eq!(
            SHADOW_BYTES + HD.view_bytes().unwrap() + FHD.view_bytes().unwrap(),
            76_681_216
        );
        assert_eq!(2 * (SHADOW_BYTES + HD.view_bytes().unwrap()), 70_418_432);
        assert_eq!(
            Extent {
                width: u64::MAX,
                height: 2
            }
            .view_bytes(),
            None
        );

        let mut s = Schedule::new(60 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let base = s.live_bytes();
        let a = s.reserve().unwrap();
        let n = s.submit(a, 7).unwrap();
        let b = s.reserve().unwrap();
        let n1 = s.submit(b, 8).unwrap();
        assert_eq!(s.live_bytes(), base); // both submissions use one scratch set
        assert_eq!(s.reserve(), Err(Refusal::CreditsFull));
        assert_eq!(s.last_submitted_sample, 8);
        assert_eq!(s.prepare(FHD), Err(Refusal::OwnerBudget));
        s.complete(n);
        assert_eq!(s.live_bytes(), base); // N+1 still uses it
        s.complete(n1);
        assert_eq!(s.prepare(FHD), Err(Refusal::OwnerBudget)); // still prepared

        let mut output = HarnessOutput::replace(HD, FHD, n1.serial).unwrap();
        assert_eq!(output.overlap_bytes(), 11_980_800);
        assert!(output.loading_diagnostic_ready);
        assert_eq!(output.release_old_after(n.serial), Err(Refusal::Busy));
        s.stop_and_release_graph().unwrap();
        assert_eq!(s.live_bytes(), 0);
        output.release_old_after(n1.serial).unwrap();
        assert_eq!(output.overlap_bytes(), 8_294_400);
        s.prepare(FHD).unwrap();
        s.publish().unwrap();
        assert_eq!(s.live_bytes(), 58_249_216);
    }

    #[test]
    fn credit_cancel_rollback_and_lease() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let old = s.active.unwrap();
        let reserved = s.reserve().unwrap();
        s.cancel(reserved).unwrap();
        assert_eq!(s.credits, [Credit::Free; 2]);
        s.prepare(FHD).unwrap();
        assert_eq!(s.live_bytes(), 76_681_216);
        s.abort_pending().unwrap(); // failed creation on healthy device
        assert_eq!(s.active, Some(old));
        assert_eq!(s.live_bytes(), 35_209_216);

        let c = s.reserve().unwrap();
        let serial = s.submit(c, 9).unwrap();
        s.lease(old.depth);
        s.complete(serial);
        assert_eq!(s.stop_and_release_graph(), Err(Refusal::Busy));
        s.release(old.depth);
        s.stop_and_release_graph().unwrap();
        assert_eq!(s.live_bytes(), 0);
    }

    #[test]
    fn later_completion_and_device_loss() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let a = s.reserve().unwrap();
        let first = s.submit(a, 1).unwrap();
        let b = s.reserve().unwrap();
        let second = s.submit(b, 2).unwrap();
        s.complete(second);
        assert_eq!(s.credits, [Credit::Free; 2]);
        s.lose_device().unwrap();
        assert_eq!(s.live_bytes(), 0);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let c = s.reserve().unwrap();
        let new_first = s.submit(c, 3).unwrap();
        s.complete(first); // old generation, same serial number
        assert_eq!(s.credits[c.slot], Credit::Submitted(new_first));
        s.complete(new_first);
        assert_eq!(s.credits[c.slot], Credit::Free);

        let depth = s.active.unwrap().depth;
        s.lease(depth);
        s.lose_device().unwrap();
        assert_eq!(s.retired_cpu_leases.get(&depth), Some(&1));
        s.release(depth);
        assert!(s.retired_cpu_leases.is_empty());
    }

    #[test]
    fn stale_reservation_cannot_steal_new_device_credit() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let old = s.reserve().unwrap();
        s.lose_device().unwrap();
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let fresh = s.reserve().unwrap();
        assert_eq!(s.submit(old, 44), Err(Refusal::Busy));
        assert_eq!(s.cancel(old), Err(Refusal::Busy));
        assert_eq!(s.credits[fresh.slot], Credit::Reserved(fresh));
        assert_eq!(s.last_submitted_sample, 0);
        s.submit(fresh, 45).unwrap();
        assert_eq!(s.last_submitted_sample, 45);
    }

    #[test]
    fn pending_and_retired_cpu_leases_block_release() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        s.prepare(FHD).unwrap();
        let pending_depth = s.pending.unwrap().depth;
        s.lease(pending_depth);
        assert_eq!(s.abort_pending(), Err(Refusal::Busy));
        s.release(pending_depth);
        s.abort_pending().unwrap();

        let old_depth = s.active.unwrap().depth;
        s.lease(old_depth);
        s.prepare(FHD).unwrap();
        s.publish().unwrap();
        assert_eq!(s.stop_and_release_graph(), Err(Refusal::Busy));
        s.release(old_depth);
        s.stop_and_release_graph().unwrap();
    }

    #[test]
    fn shared_shadow_lease_does_not_block_candidate_rollback() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let shadow = s.active.unwrap().shadow;
        s.lease(shadow);
        s.prepare(FHD).unwrap();
        s.abort_pending().unwrap();
        assert_eq!(s.live_bytes(), 35_209_216);
        s.release(shadow);
    }

    #[test]
    fn harness_output_overlap_rejects_overflow() {
        let huge = Extent {
            width: u64::MAX / 4,
            height: 1,
        };
        assert_eq!(
            HarnessOutput::replace(huge, huge, 1),
            Err(Refusal::Arithmetic)
        );
    }

    #[test]
    fn published_overlap_keeps_old_view_until_its_last_submission() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let old = s.active.unwrap();
        let first = s.reserve().unwrap();
        let held = s.submit(first, 10).unwrap();
        s.prepare(FHD).unwrap();
        assert_eq!(s.live_bytes(), 76_681_216);
        s.publish().unwrap();
        assert_eq!(s.live_bytes(), 76_681_216);
        assert_eq!(s.allocations.len(), 7); // one shared shadow plus two three-slot views
        let next = s.reserve().unwrap();
        let newer = s.submit(next, 11).unwrap();
        assert_eq!(s.active.unwrap().shadow, old.shadow);
        s.complete(held);
        assert_eq!(s.live_bytes(), 58_249_216);
        assert_eq!(s.allocations.len(), 4);
        s.complete(newer);
        assert_eq!(s.live_bytes(), 58_249_216); // current prepared graph stays charged
    }

    #[test]
    fn reserved_packet_blocks_graph_publication_until_canceled() {
        let mut s = Schedule::new(100 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let packet = s.reserve().unwrap();
        s.prepare(FHD).unwrap();
        assert_eq!(s.publish(), Err(Refusal::Busy));
        assert_eq!(s.submit(packet, 12).unwrap().serial, 1);
        s.publish().unwrap();
        assert_eq!(s.last_submitted_sample, 12);
    }

    #[test]
    fn failure_after_stop_cannot_resurrect_old_graph() {
        let mut s = Schedule::new(60 * MIB);
        s.prepare(HD).unwrap();
        s.publish().unwrap();
        let output = HarnessOutput::replace(HD, FHD, 0).unwrap();
        assert!(output.loading_diagnostic_ready);
        s.stop_and_release_graph().unwrap();
        assert_eq!(s.live_bytes(), 0);
        s.prepare(FHD).unwrap();
        s.abort_pending().unwrap(); // simulated healthy-device preparation fault
        assert!(s.active.is_none());
        assert_eq!(s.live_bytes(), 0);
        assert_eq!(s.last_submitted_sample, 0);
        assert_eq!(s.reserve(), Err(Refusal::NoGraph));
    }
}
