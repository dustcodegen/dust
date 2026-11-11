use super::Index;

/// No source identity or facts survive here; generations separate every traversal.
#[derive(Default)]
pub(super) struct Scratch {
    /// Zero remains the unvisited sentinel, including after generation wrap.
    generation: u32,
    /// Rank-indexed generation marks replace per-root string-set allocations.
    marks: Vec<u32>,
    /// Pending ranks are marked before enqueueing so cycles cannot duplicate work.
    pending: Vec<usize>,
    /// Reached ranks become the exact lexical URI order before hashing.
    reached: Vec<usize>,
}

impl Scratch {
    /// Traverses only this graph, reusing capacity across roots and graph instances.
    pub(super) fn reachable(&mut self, index: &Index, root: usize) -> &[usize] {
        self.begin(index.uris.len());
        self.enqueue(root);
        while let Some(rank) = self.pending.pop() {
            for &dependency in &index.edges[rank] {
                self.enqueue(dependency);
            }
        }
        self.reached.sort_unstable();
        &self.reached
    }

    /// A wrapped generation must not inherit marks from the first traversal.
    fn begin(&mut self, count: usize) {
        self.marks.resize(count, 0);
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.marks.fill(0);
            self.generation = 1;
        }
        self.pending.clear();
        self.reached.clear();
    }

    /// Each rank enters the traversal at most once, including cycles and diamonds.
    fn enqueue(&mut self, rank: usize) {
        if self.marks[rank] != self.generation {
            self.marks[rank] = self.generation;
            self.pending.push(rank);
            self.reached.push(rank);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scratch;

    #[test]
    fn generation_wrap_clears_stale_marks_before_rank_reuse() {
        let mut scratch = Scratch {
            generation: u32::MAX,
            marks: vec![1, u32::MAX, 1],
            pending: vec![1],
            reached: vec![1],
        };
        scratch.begin(3);
        scratch.enqueue(0);
        scratch.enqueue(2);
        scratch.enqueue(0);
        assert_eq!(scratch.generation, 1);
        assert_eq!(scratch.marks, [1, 0, 1]);
        assert_eq!(scratch.pending, [0, 2]);
        assert_eq!(scratch.reached, [0, 2]);
        scratch.begin(1);
        scratch.enqueue(0);
        assert_eq!(scratch.reached, [0]);
        scratch.begin(4);
        scratch.enqueue(3);
        assert_eq!(scratch.reached, [3]);
    }
}
