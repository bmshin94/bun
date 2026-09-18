//! Waiting for the GPU: wgpu-core fires callbacks only inside `device_poll`, so one [`Job`] per device polls: on the work pool at first, and on the device's own thread when the wait is long.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use bun_jsc::job::JsAffine;
use bun_jsc::{Completion, ContextId, Job, JobContext, JsCell, JsResult, JsThread};
use bun_threading::Guarded;

use super::device::DeviceRef;

enum SlotState<R> {
    Empty,
    Filled(R),
    Closed,
}

/// Where a wgpu-core callback, on any thread, leaves its result for the JS thread.
pub(crate) struct Slot<R> {
    state: Guarded<SlotState<R>>,
    /// The device's count of filled slots: a change is what ends its poller's run.
    filled: Arc<AtomicUsize>,
}

impl<R> Slot<R> {
    pub(crate) fn fill(&self, value: R) {
        let mut state = self.state.lock();
        if matches!(*state, SlotState::Empty) {
            *state = SlotState::Filled(value);
            self.filled.fetch_add(1, Ordering::Release);
        }
    }

    fn is_filled(&self) -> bool {
        matches!(*self.state.lock(), SlotState::Filled(_))
    }

    /// Takes the result, if there is one, and refuses any later one.
    fn close(&self) -> Option<R> {
        match core::mem::replace(&mut *self.state.lock(), SlotState::Closed) {
            SlotState::Filled(result) => Some(result),
            SlotState::Empty | SlotState::Closed => None,
        }
    }
}

/// What to do on the JS thread once the callback has fired.
pub(crate) trait Waiter: 'static {
    type Result: Send + 'static;
    /// JS-thread state of the wait: the promise, the wrapper it belongs to.
    type Js: 'static;
    /// `result` is `None` if the device was lost before the GPU answered.
    fn settle(result: Option<Self::Result>, js: Self::Js, cx: &JsThread<'_>) -> JsResult<()>;
}

trait Pending {
    fn is_ready(&self) -> bool;
    fn context(&self) -> ContextId;
    fn settle(self: Box<Self>, cx: &JsThread<'_>) -> JsResult<()>;
}

struct PendingWait<W: Waiter> {
    slot: Arc<Slot<W::Result>>,
    js: W::Js,
    /// The context of the script that made the call. Its promise settles in that one, or not at all once it has stopped.
    context: ContextId,
}

impl<W: Waiter> Pending for PendingWait<W> {
    fn is_ready(&self) -> bool {
        self.slot.is_filled()
    }

    fn context(&self) -> ContextId {
        self.context
    }

    fn settle(self: Box<Self>, cx: &JsThread<'_>) -> JsResult<()> {
        let PendingWait { slot, js, .. } = *self;
        W::settle(slot.close(), js, cx)
    }
}

/// A device's unsettled `mapAsync()` and `onSubmittedWorkDone()` calls, in the order script made them.
#[derive(Default)]
pub(crate) struct Waits {
    filled: Arc<AtomicUsize>,
    pending: JsCell<Vec<Box<dyn Pending>>>,
    /// The value of `filled` the last delivery accounted for.
    delivered: Cell<usize>,
    polling: Cell<bool>,
    long_waits: LongWaits,
}

impl Waits {
    pub(crate) fn slot<R>(&self) -> Arc<Slot<R>> {
        Arc::new(Slot {
            state: Guarded::new(SlotState::Empty),
            filled: Arc::clone(&self.filled),
        })
    }
}

/// Settles `js` through `W` once `slot` is filled. Among the waits that are ready, settling follows call order.
pub(crate) fn wait<W: Waiter>(
    device: &DeviceRef,
    cx: &JsThread<'_>,
    slot: Arc<Slot<W::Result>>,
    js: W::Js,
) {
    let entry: Box<dyn Pending> = Box::new(PendingWait::<W> {
        slot,
        js,
        context: cx.context().id(),
    });
    device.waits.pending.with_mut(|pending| pending.push(entry));
    start_poller(device, cx);
}

fn start_poller(device: &DeviceRef, cx: &JsThread<'_>) {
    let waits = &device.waits;
    if waits.polling.get() || waits.pending.with_mut(|pending| pending.is_empty()) {
        return;
    }
    waits.polling.set(true);
    // The realm's context, not the caller's: one poller serves the waits of every context that uses the device, and a `Bun.ModuleGraph` that is disposed must not take it along.
    let realm = cx.global().js_thread(cx.vm().root_context());
    Job::<Poller>::schedule(
        &realm,
        PollOff {
            shared: Arc::new(PollShared {
                device: Arc::clone(&device.raw),
                filled: Arc::clone(&waits.filled),
                seen: waits.delivered.get(),
                failed: AtomicBool::new(false),
                cancelled: AtomicBool::new(false),
            }),
            long_waits: Arc::clone(&waits.long_waits),
        },
        PollJs(Some(Rc::clone(device))),
    );
}

/// Settles every ready wait, oldest first: wgpu-core fills a `mapAsync()` slot before a later `onSubmittedWorkDone()` one.
fn deliver(device: &DeviceRef, failed: bool, cx: &JsThread<'_>) -> JsResult<()> {
    let waits = &device.waits;
    // Read before the scan: the next poller sees a slot filled after this as a change and returns at once.
    let accounted = waits.filled.load(Ordering::Acquire);
    let vm = cx.vm();
    let ready: Vec<Box<dyn Pending>> = waits.pending.with_mut(|pending| {
        // A context that has stopped (a disposed `Bun.ModuleGraph`) waits for nothing.
        pending.retain(|entry| vm.is_context_live(entry.context()));
        if failed {
            return core::mem::take(pending);
        }
        pending.extract_if(.., |entry| entry.is_ready()).collect()
    });
    waits.delivered.set(accounted);
    let mut result = Ok(());
    for entry in ready {
        let context = entry.context();
        // Settling an earlier wait runs script, which can stop this one's context.
        if !vm.is_context_live(context) {
            continue;
        }
        let _scope = vm.enter_context(context);
        let settled = entry.settle(&cx.global().js_thread(vm.context_of(context)));
        if result.is_ok() {
            result = settled;
        }
    }
    // Last: resolving `lost` can run script (a `then` getter on Object.prototype).
    let delivered = device.deliver_loss(cx.global());
    result.and(delivered)
}

struct Poller;

/// Shared with the thread that takes a long wait over from the pool.
struct PollShared {
    device: Arc<bun_webgpu::Device>,
    filled: Arc<AtomicUsize>,
    /// The poller runs until `filled` differs from this.
    seen: usize,
    /// A lost device fails every poll and answers nothing more: every pending wait settles then.
    failed: AtomicBool,
    cancelled: AtomicBool,
}

impl PollShared {
    /// Polls until a slot fills, the job is cancelled, or `budget` runs out. `true`: the job can complete.
    fn poll(&self, done: &Completion<Poller>, budget: Option<Duration>) -> bool {
        let start = Instant::now();
        loop {
            if self.filled.load(Ordering::Acquire) != self.seen
                || self.cancelled.load(Ordering::Acquire)
                || done.ticket().cancelled()
            {
                return true;
            }
            if !self.device.poll() {
                self.failed.store(true, Ordering::Release);
                return true;
            }
            if self.filled.load(Ordering::Acquire) != self.seen {
                return true;
            }
            if budget.is_some_and(|budget| start.elapsed() >= budget) {
                return false;
            }
            // An eighth of the time waited so far: short work is noticed fast, long work costs few wakeups.
            std::thread::sleep((start.elapsed() / 8).clamp(MIN_PAUSE, MAX_PAUSE));
        }
    }
}

/// A wait that outlasted [`POOL_BUDGET`], on its way to the device's own thread.
type LongWait = (Arc<PollShared>, Completion<Poller>);

/// The way to that thread, which starts with the first such wait and ends when the device's [`Waits`] and its poller jobs are gone.
type LongWaits = Arc<Guarded<Option<Sender<LongWait>>>>;

/// Moves a wait from the pool, which every other async job of the process shares, to the device's own thread. Gives `done` back if there is no such thread.
fn leave_pool(
    long_waits: &LongWaits,
    shared: Arc<PollShared>,
    done: Completion<Poller>,
) -> Option<Completion<Poller>> {
    let mut sender = long_waits.lock();
    if sender.is_none() {
        let (to_thread, waits) = mpsc::channel::<LongWait>();
        // SAFETY: all the thread ever holds is what a job sends it: the job's `Completion`, which carries the VM's `Ticket` until `finish()` has posted the job, and a `PollShared`, which is no VM's state.
        let spawned = std::thread::Builder::new()
            .name(String::from("bun-webgpu-poll"))
            .spawn(move || {
                while let Ok((shared, done)) = waits.recv() {
                    shared.poll(&done, None);
                    done.finish();
                }
            });
        if spawned.is_err() {
            return Some(done);
        }
        *sender = Some(to_thread);
    }
    match sender.as_ref()?.send((shared, done)) {
        Ok(()) => None,
        Err(mpsc::SendError((_, done))) => Some(done),
    }
}

struct PollOff {
    shared: Arc<PollShared>,
    long_waits: LongWaits,
}

/// The device a poller serves. Dropped without `then` (the VM is stopping), it releases the device's waits.
struct PollJs(Option<DeviceRef>);

// SAFETY: an `Rc` that is created, used and dropped on the JS thread only, which is where a job's `Js` half lives.
unsafe impl JsAffine for PollJs {}

impl Drop for PollJs {
    fn drop(&mut self) {
        if let Some(device) = self.0.take() {
            device.waits.polling.set(false);
            drop(device.waits.pending.take());
        }
    }
}

impl JobContext for Poller {
    type OffThread = PollOff;
    type Js = PollJs;

    const CANCELLABLE: bool = true;

    fn run(off: &mut Self::OffThread, done: Completion<Self>) -> Option<Completion<Self>> {
        if off.shared.poll(&done, Some(POOL_BUDGET)) {
            return Some(done);
        }
        let done = leave_pool(&off.long_waits, Arc::clone(&off.shared), done)?;
        off.shared.poll(&done, None);
        Some(done)
    }

    fn then(off: Self::OffThread, mut js: Self::Js, cx: &JsThread<'_>) -> JsResult<()> {
        let Some(device) = js.0.take() else {
            return Ok(());
        };
        device.waits.polling.set(false);
        let result = deliver(&device, off.shared.failed.load(Ordering::Acquire), cx);
        start_poller(&device, cx);
        result
    }

    unsafe fn cancel(off: *mut Self::OffThread) {
        // SAFETY: `off` points at the live job's off-thread half. Its `Arc` is only ever read, here and in `run`, and the flag behind it is an atomic.
        let shared = unsafe { &(*off).shared };
        shared.cancelled.store(true, Ordering::Release);
    }
}

const MIN_PAUSE: Duration = Duration::from_micros(50);
const MAX_PAUSE: Duration = Duration::from_millis(4);
/// How long a wait may keep a pool thread before it moves to the device's own.
const POOL_BUDGET: Duration = Duration::from_millis(2);
