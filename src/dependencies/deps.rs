use super::*;
use std::collections::{HashMap, HashSet};

pub(super) fn find_deps_for_var(
    sorted_access: &[Access],
    instances: &[StatementInstance],
    deps: &mut HashMap<DependencyEdge, HashSet<LevelDependency>>,
) {
    let mut tracker = InstanceTracker::new();
    let mut iter = sorted_access.iter();
    let mut last = match iter.next() {
        Some(l) => l,
        None => return,
    };

    while let Some(access) = iter.next() {
        // Add access in instance to tracker
        tracker.add_access(last);

        // Check if we're still on the same element
        let same = access.indices == last.indices;
        last = access;
        if same {
            continue;
        }

        // Find levels for found dependencies and store in global set
        for (edge, dep) in tracker.calc_dependencies() {
            let (si1, si2) = to_instances(edge, instances);
            let edge = DependencyEdge(si1.statement, si2.statement);

            deps.entry(edge)
                .and_modify(|set| {
                    let c = find_level(si1, si2);
                    set.insert(LevelDependency(c, dep.clone()));
                })
                .or_insert_with(|| {
                    let mut set = HashSet::new();

                    let c = find_level(si1, si2);
                    set.insert(LevelDependency(c, dep.clone()));

                    set
                });
        }

        tracker = InstanceTracker::new();
    }

    // Add access in instance to tracker
    tracker.add_access(last);

    // Find levels for found dependencies and store in global set
    for (edge, dep) in tracker.calc_dependencies() {
        let (si1, si2) = to_instances(edge, instances);
        let edge = DependencyEdge(si1.statement, si2.statement);

        deps.entry(edge)
            .and_modify(|set| {
                let c = find_level(si1, si2);
                set.insert(LevelDependency(c, dep.clone()));
            })
            .or_insert_with(|| {
                let mut set = HashSet::new();

                let c = find_level(si1, si2);
                set.insert(LevelDependency(c, dep.clone()));

                set
            });
    }
}

fn to_instances(
    dep: DependencyEdge,
    instances: &[StatementInstance],
) -> (&StatementInstance, &StatementInstance) {
    let DependencyEdge(i1, i2) = dep;
    (&instances[i1 as usize], &instances[i2 as usize])
}

fn find_level(s1: &StatementInstance, s2: &StatementInstance) -> i32 {
    let maxlevel = max_common_level(s1, s2);

    // Take maximum common loop
    let levels = s1
        .iteration
        .iter()
        .zip(s2.iteration.iter())
        .take(maxlevel)
        .enumerate();

    // Find loop carrying level
    for (c, (i, j)) in levels {
        if i != j {
            return c as i32 + 1;
        }
    }

    // If we have no loop carried dependencies,
    // dependency is loop independent
    0
}

fn max_common_level(s1: &StatementInstance, s2: &StatementInstance) -> usize {
    let loops = s1.loops.iter().zip(s2.loops.iter());
    let mut counter = 0;
    for (l1, l2) in loops {
        if l1 != l2 {
            break;
        }

        counter += 1;
    }

    counter
}

struct InstanceTracker {
    use_def: Vec<(Statement, (bool, bool))>,
}

impl InstanceTracker {
    fn new() -> Self {
        InstanceTracker {
            use_def: Vec::new(),
        }
    }

    fn add_access(&mut self, access: &Access) {
        let mut was_present = false;

        if let Some((last, (use_flag, def_flag))) = self.use_def.last_mut() {
            if *last == access.statement {
                match access.category {
                    Category::Read => *use_flag = true,
                    Category::Write => *def_flag = true,
                }

                was_present = true;
            }
        };

        if !was_present {
            self.use_def.push((
                access.statement,
                match access.category {
                    Category::Read => (true, false),
                    Category::Write => (false, true),
                },
            ));
        }
    }

    fn calc_dependencies(&self) -> Vec<(DependencyEdge, DependencyType)> {
        let mut deps = Vec::new();

        let iter = self.use_def.iter().zip(self.use_def.iter().skip(1));
        let mut last_write = -1i32;
        let mut uses = Vec::new();

        for ((s1, (u1, d1)), (s2, (u2, d2))) in iter {
            let ((s1, (u1, d1)), (s2, (u2, d2))) = ((*s1, (*u1, *d1)), (*s2, (*u2, *d2)));
            let edge = DependencyEdge(s1, s2);

            if !u1 && d1 && !u2 && d2 {
                deps.push((edge, DependencyType::Output));
                last_write = s2;
                uses.clear();
            } else if !u1 && d1 && u2 && !d2 {
                deps.push((edge, DependencyType::True));
                last_write = s1;
                uses.clear();
            } else if !u1 && d1 && u2 && d2 {
                deps.push((edge.clone(), DependencyType::True));
                deps.push((edge, DependencyType::Output));
                last_write = s2;
                uses.clear();
            } else if u1 && !d1 && !u2 && d2 {
                for last in uses.iter() {
                    let edge = DependencyEdge(*last, s2);
                    deps.push((edge, DependencyType::Anti));
                }

                if last_write >= 0 {
                    let edge = DependencyEdge(last_write, s2);
                    deps.push((edge, DependencyType::Output));
                }

                deps.push((edge, DependencyType::Anti));
                last_write = s2;
                uses.clear();
            } else if u1 && !d1 && u2 && d2 {
                for last in uses.iter() {
                    let edge = DependencyEdge(*last, s2);
                    deps.push((edge, DependencyType::Anti));
                }

                if last_write >= 0 {
                    let edge = DependencyEdge(last_write, s2);
                    deps.push((edge, DependencyType::True));
                }

                deps.push((edge, DependencyType::Anti));
                last_write = s2;
                uses.clear();
            } else if u1 && d1 && !u2 && d2 {
                deps.push((edge.clone(), DependencyType::Anti));
                deps.push((edge, DependencyType::Output));
                last_write = s2;
                uses.clear();
            } else if u1 && d1 && u2 && !d2 {
                deps.push((edge, DependencyType::True));
                last_write = s1;
                uses.clear();
            } else if u1 && d1 && u2 && d2 {
                deps.push((edge.clone(), DependencyType::True));
                deps.push((edge.clone(), DependencyType::Output));
                deps.push((edge.clone(), DependencyType::Anti));
                last_write = s2;
                uses.clear();
            } else {
                uses.push(s1);

                if last_write >= 0 {
                    let edge = DependencyEdge(last_write, s2);
                    deps.push((edge, DependencyType::True));
                }
            }
        }

        deps
    }
}
