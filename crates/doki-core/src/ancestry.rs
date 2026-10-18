//! How two checkpoints relate, decided only by parent links (I4).
//!
//! Clocks, file sizes and modification times never enter this module, so
//! clock skew between devices cannot choose which progress survives.

use std::collections::{BTreeSet, HashSet, VecDeque};

use crate::model::CheckpointId;

/// Read access to checkpoint parent links.
pub trait Graph {
    /// Parents of `id`, or `None` when this device does not have that
    /// checkpoint (history is incomplete, not empty).
    fn parents(&self, id: &CheckpointId) -> Option<Vec<CheckpointId>>;
}

/// Relation of an incoming checkpoint to the local one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relation {
    /// The same checkpoint.
    Same,
    /// Incoming descends from local: it continues this device's progress.
    Ahead,
    /// Incoming is an ancestor of local: this device already moved past it.
    Behind,
    /// Both continued from a shared checkpoint. `common` lists the nearest
    /// shared ancestors (usually one).
    Divergent { common: Vec<CheckpointId> },
    /// Complete histories share nothing, e.g. each device captured the game
    /// before they were connected. Never treated as Ahead.
    Unrelated,
    /// Some ancestry is missing, so the relation cannot be decided yet.
    Unknown { missing: Vec<CheckpointId> },
}

struct Ancestors {
    /// Includes the starting checkpoint itself.
    all: HashSet<CheckpointId>,
    missing: BTreeSet<CheckpointId>,
}

fn ancestors(graph: &impl Graph, start: &CheckpointId) -> Ancestors {
    let mut all = HashSet::new();
    let mut missing = BTreeSet::new();
    let mut queue = VecDeque::from([start.clone()]);
    while let Some(id) = queue.pop_front() {
        if !all.insert(id.clone()) {
            continue;
        }
        match graph.parents(&id) {
            Some(parents) => queue.extend(parents),
            None => {
                missing.insert(id);
            }
        }
    }
    Ancestors { all, missing }
}

/// Decide how `incoming` relates to `local`.
pub fn relate(graph: &impl Graph, local: &CheckpointId, incoming: &CheckpointId) -> Relation {
    if local == incoming {
        return Relation::Same;
    }
    let of_local = ancestors(graph, local);
    let of_incoming = ancestors(graph, incoming);
    // A path found through known links is conclusive even if other
    // branches of history are missing.
    if of_incoming.all.contains(local) && !of_local.missing.contains(local) {
        return Relation::Ahead;
    }
    if of_local.all.contains(incoming) && !of_incoming.missing.contains(incoming) {
        return Relation::Behind;
    }
    let missing: Vec<_> = of_local.missing.union(&of_incoming.missing).cloned().collect();
    if !missing.is_empty() {
        return Relation::Unknown { missing };
    }
    let common: HashSet<_> = of_local.all.intersection(&of_incoming.all).cloned().collect();
    if common.is_empty() {
        return Relation::Unrelated;
    }
    // Keep only the nearest shared ancestors: drop any that is a strict
    // ancestor of another shared one.
    let mut nearest: Vec<_> = common
        .iter()
        .filter(|c| {
            !common.iter().any(|other| other != *c && ancestors(graph, other).all.contains(*c))
        })
        .cloned()
        .collect();
    nearest.sort();
    Relation::Divergent { common: nearest }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Map(HashMap<CheckpointId, Vec<CheckpointId>>);

    impl Graph for Map {
        fn parents(&self, id: &CheckpointId) -> Option<Vec<CheckpointId>> {
            self.0.get(id).cloned()
        }
    }

    fn id(n: u32) -> CheckpointId {
        CheckpointId::parse(&format!("{n:08x}")).unwrap()
    }

    fn graph(edges: &[(u32, &[u32])]) -> Map {
        Map(edges.iter().map(|(c, ps)| (id(*c), ps.iter().map(|p| id(*p)).collect())).collect())
    }

    #[test]
    fn linear_history() {
        // 1 <- 2 <- 3
        let g = graph(&[(1, &[]), (2, &[1]), (3, &[2])]);
        assert_eq!(relate(&g, &id(3), &id(3)), Relation::Same);
        assert_eq!(relate(&g, &id(1), &id(3)), Relation::Ahead);
        assert_eq!(relate(&g, &id(3), &id(1)), Relation::Behind);
    }

    #[test]
    fn divergence_reports_nearest_common_ancestor() {
        // 1 <- 2 <- {3, 4}
        let g = graph(&[(1, &[]), (2, &[1]), (3, &[2]), (4, &[2])]);
        assert_eq!(relate(&g, &id(3), &id(4)), Relation::Divergent { common: vec![id(2)] });
    }

    #[test]
    fn choice_checkpoint_resolves_divergence_everywhere() {
        // Handheld chose 4 over 3 by recording choice 5 = [4, 3].
        // A third device still at 3 now sees 5 as Ahead, not a new conflict.
        let g = graph(&[(1, &[]), (3, &[1]), (4, &[1]), (5, &[4, 3])]);
        assert_eq!(relate(&g, &id(3), &id(5)), Relation::Ahead);
        assert_eq!(relate(&g, &id(4), &id(5)), Relation::Ahead);
    }

    #[test]
    fn unrelated_histories() {
        let g = graph(&[(1, &[]), (2, &[])]);
        assert_eq!(relate(&g, &id(1), &id(2)), Relation::Unrelated);
    }

    #[test]
    fn missing_history_is_unknown_not_unrelated() {
        // 3's parent 2 is not on this device.
        let g = graph(&[(1, &[]), (3, &[2])]);
        assert_eq!(relate(&g, &id(1), &id(3)), Relation::Unknown { missing: vec![id(2)] });
    }

    #[test]
    fn conclusive_path_wins_over_unrelated_missing_branch() {
        // 3 = choice of [2, 9]; 9 is missing but 2 descends from local 1.
        let g = graph(&[(1, &[]), (2, &[1]), (3, &[2, 9])]);
        assert_eq!(relate(&g, &id(1), &id(3)), Relation::Ahead);
    }

    #[test]
    fn criss_cross_reports_both_nearest_ancestors() {
        // 1 <- {2, 3}; 4 = [2, 3]; 5 = [3, 2]
        let g = graph(&[(1, &[]), (2, &[1]), (3, &[1]), (4, &[2, 3]), (5, &[3, 2])]);
        assert_eq!(relate(&g, &id(4), &id(5)), Relation::Divergent { common: vec![id(2), id(3)] });
    }
}
