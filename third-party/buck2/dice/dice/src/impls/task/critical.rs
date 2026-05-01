/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use allocative::Allocative;
use allocative::Visitor;
use dice_error::result::CancellationReason;
use dupe::Dupe;
use futures::task::AtomicWaker;
use parking_lot::Mutex;
use slab::Slab;

use crate::arc::Arc;
use crate::impls::key::ParentKey;
use crate::impls::task::dice::Cancellations;
use crate::impls::task::dice::SlabId;

/// Wrapper around the mutex-protected critical section of `DiceTaskInternal`.
/// All access to dependant/termination-observer slabs goes through methods here,
/// keeping lock acquisition encapsulated.
#[derive(Allocative)]
pub(super) struct DiceTaskInternalCritical(Mutex<Data>);

/// Result of attempting to register as a dependant of a task.
pub(crate) enum DependedOnByResult {
    Cancelled(CancellationReason),
    Pending(SlabId, Arc<AtomicWaker>),
    Finished,
}
impl DiceTaskInternalCritical {
    pub(crate) fn depended_on_by(
        &self,
        cancellations: &Cancellations,
        k: ParentKey,
    ) -> DependedOnByResult {
        let mut critical = self.0.lock();
        if let Some(reason) = cancellations.is_cancelled(&critical) {
            return DependedOnByResult::Cancelled(reason);
        }
        match &mut critical.dependants {
            None => DependedOnByResult::Finished,
            Some(wakers) => {
                let waker = Arc::new(AtomicWaker::new());
                let id = wakers.insert((k, waker.dupe()));

                DependedOnByResult::Pending(SlabId::Dependants(id), waker)
            }
        }
    }

    pub(crate) fn get_waiters_copy(&self) -> Option<Vec<ParentKey>> {
        self.0
            .lock()
            .dependants
            .as_ref()
            .map(|deps| deps.iter().map(|(_, (k, _))| *k).collect())
    }

    pub(crate) fn cancel(&self, cancellations: &Cancellations, reason: CancellationReason) {
        let lock = self.0.lock();
        cancellations.cancel(&lock, reason);
    }

    pub(crate) fn await_termination(&self) -> Option<(SlabId, Arc<AtomicWaker>)> {
        let mut critical = self.0.lock();
        match &mut critical.termination_observers {
            None => None,
            Some(wakers) => {
                let waker = Arc::new(AtomicWaker::new());
                let id = wakers.insert(waker.dupe());

                Some((SlabId::TerminationObserver(id), waker))
            }
        }
    }

    pub(crate) fn new() -> Self {
        Self(Mutex::new(Data {
            dependants: Some(Slab::new()),
            termination_observers: Some(Slab::new()),
        }))
    }

    pub(crate) fn drop_waiter(&self, slab: &SlabId, cancellations: &Cancellations) {
        let mut critical = self.0.lock();
        match slab {
            SlabId::Dependants(id) => match critical.dependants {
                None => {}
                Some(ref mut deps) => {
                    deps.remove(*id);
                    if deps.is_empty() {
                        cancellations.cancel(&critical, CancellationReason::AllDependentsDropped);
                    }
                }
            },
            SlabId::TerminationObserver(id) => match critical.termination_observers {
                None => {}
                Some(ref mut deps) => {
                    deps.remove(*id);
                    if deps.is_empty() {
                        cancellations.cancel(&critical, CancellationReason::AllObserversDropped);
                    }
                }
            },
        }
    }

    pub(super) fn wake_dependents(&self) {
        let mut critical = self.0.lock();
        let mut deps = critical
            .dependants
            .take()
            .expect("Invalid state where deps where taken already");
        let mut termination_observers = critical
            .termination_observers
            .take()
            .expect("Invalid state where deps where taken already");

        deps.drain().for_each(|(_k, waker)| waker.wake());
        // wake up all the `TerminationObserver::poll`
        termination_observers.drain().for_each(|waker| waker.wake());
    }
}

struct Data {
    /// Other DiceTasks that are awaiting the completion of this task.
    ///
    /// We hold a pair DiceKey and Waker.
    /// Compared to 'Shared', which just holds a standard 'Waker', the Waker itself is now an
    /// AtomicWaker, which is an extra AtomicUsize, so this is marginally larger than the standard
    /// Shared future.
    dependants: Option<Slab<(ParentKey, Arc<AtomicWaker>)>>,
    termination_observers: Option<Slab<Arc<AtomicWaker>>>,
}

impl Allocative for Data {
    fn visit<'a, 'b: 'a>(&self, visitor: &'a mut Visitor<'b>) {
        let mut visitor = visitor.enter_self_sized::<Self>();
        visitor.visit_field(allocative::Key::new("dependants"), &self.dependants);
        visitor.exit();
    }
}
