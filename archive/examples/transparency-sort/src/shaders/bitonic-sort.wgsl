// Pass 2 of the GPU sort: one bitonic merge step.
//
// Bitonic sort runs as a fixed schedule of O(log^2 n) compare-exchange steps.
// The host dispatches this entry once per (k, j) step with the matching params;
// each invocation handles one element `i` and conditionally swaps it with its
// partner `i ^ j`. We sort DESCENDING by `dist` (farthest first) so the draw
// pass renders back-to-front for correct alpha blending.

struct Key {
  dist: f32,
  index: u32,
}

struct SortParams {
  j: u32,     // compare distance for this step
  k: u32,     // size of the current bitonic sequence
  total: u32, // padded element count
  pad: u32,
}

@group(0) @binding(0) var<uniform> params: SortParams;
@group(0) @binding(1) var<storage, read_write> keys: array<Key>;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) globalId: vec3<u32>) {
  let i = globalId.x;
  if (i >= params.total) {
    return;
  }

  let partner = i ^ params.j;
  // Only the lower index of each pair performs the swap.
  if (partner <= i) {
    return;
  }

  let a = keys[i];
  let b = keys[partner];

  // `ascending` flips every `k` elements to build the bitonic sequence.
  // For an overall descending result we invert the usual comparisons.
  let ascending = (i & params.k) == 0u;
  let needSwap = select(a.dist > b.dist, a.dist < b.dist, ascending);
  if (needSwap) {
    keys[i] = b;
    keys[partner] = a;
  }
}
