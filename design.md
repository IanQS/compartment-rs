# compartment-rs Design Notes

## SWC ID and Root Convention

- SWC uses `-1` parent for the root node.
- Internally, we use unsigned IDs (`u64`) and represent the root as self-parenting: `root.parent_id == root.node_id`, which avoids us needing to store the IDs as `i32` for just the one value

## Adjacency Map Invariant

- `parent_child_map` keeps the root self-edge (`root -> root`) as part of the sentinel convention.
- the topological sort ignores self-edges during the traversal: for a node `n`, use filtered children `c where c != n` for branch degree and path walking.

## Section Coalescing

- Pipeline: SWC nodes (topologically ordered) -> section coalescing -> d_lambda subdivision -> compartments.
- A section starts at:
  - the soma/root, or
  - a node whose parent is soma, or
  - a node whose parent is a branch point (after self-edge filtering).
- Section tracing follows a single non-self child until a leaf or branch point.
- Section topology is reconstructed from first-node parent relations and then wired as section parent/children links.
