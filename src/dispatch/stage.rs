use rayon::prelude::*;
use smallvec::SmallVec;

use dispatch::{SystemExecSend, SystemId, SystemInfo, SystemReads, SystemWrites};
use res::Resources;

pub struct Stage<'a> {
    systems: SmallVec<[SystemExecSend<'a>; 8]>,
}

impl<'a> Stage<'a> {
    fn new(systems: SmallVec<[SystemExecSend<'a>; 8]>) -> Self {
        Stage { systems: systems }
    }

    pub fn execute(&mut self, res: &Resources) {
        let systems = &mut self.systems;
        let systems: &mut [SystemExecSend<'a>] = &mut *systems;
        systems.par_iter_mut().for_each(move |x| x.run_now(res));
    }

    pub fn execute_seq(&mut self, res: &Resources) {
        for system in &mut self.systems {
            system.run_now(res);
        }
    }
}

#[derive(Default)]
pub struct StagesBuilder<'a> {
    barrier: usize,
    stages: Vec<StageMeta<'a>>,
}

impl<'a> StagesBuilder<'a> {
    pub fn add_barrier(&mut self) {
        self.barrier = self.stages.len();
    }

    pub fn insert(&mut self, mut info: SystemInfo<'a>) {
        let pos = if self.barrier < self.stages.len() {
            self.stages[self.barrier..]
                .iter_mut()
                .position(|x| x.dependency_check(&mut info) && !x.conflicts_with(&info))
                .unwrap_or_else(|| {
                                    self.stages.push(StageMeta::new());

                                    self.stages.len() - 1
                                })
        } else {
            self.stages.push(StageMeta::new());

            self.stages.len() - 1
        };

        self.stages[pos].insert(info);
    }

    pub fn build(self) -> Vec<Stage<'a>> {
        self.stages
            .into_iter()
            .map(|x| x.systems)
            .map(Stage::new)
            .collect()
    }
}

/// The stage type used for building stages (`StagesBuilder`).
#[derive(Default)]
pub struct StageMeta<'a> {
    ids: SmallVec<[SystemId; 8]>,
    reads: SystemReads,
    systems: SmallVec<[SystemExecSend<'a>; 8]>,
    writes: SystemWrites,
}

impl<'a> StageMeta<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn conflicts_with(&self, info: &SystemInfo) -> bool {
        info.reads
            .iter()
            .position(|x| self.writes.iter().position(|y| *x == *y).is_some())
            .is_some() ||
        info.writes
            .iter()
            .position(|x| {
                          self.reads.iter().position(|y| *x == *y).is_some() ||
                          self.writes.iter().position(|y| *x == *y).is_some()
                      })
            .is_some()
    }

    pub fn dependency_check(&self, info: &mut SystemInfo) -> bool {
        if info.dependencies.len() == 0 {
            true
        } else {
            for id in &self.ids {
                if let Some(pos) = info.dependencies.iter().position(|x| *x == *id) {
                    info.dependencies.swap_remove(pos);
                }
            }

            false
        }
    }

    pub fn insert(&mut self, info: SystemInfo<'a>) {
        self.ids.push(info.id);
        self.reads.extend_from_slice(&*info.reads);
        self.writes.extend_from_slice(&*info.writes);

        self.systems.push(info.system);
    }
}
